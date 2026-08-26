use anyhow::{Context, Result};
use jarvis_memory::Memory;
use jarvis_mesh::Mesh;
use jarvis_protocol::{detect_lang, ChatTurn, ClientMessage, Lang, ServerMessage};
use jarvis_tasks::TaskQueue;
use jarvis_tools::ToolHost;
use jarvis_voice::Voice;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

mod visual;
pub use visual::{parse_visual_tag, visual_from_prompt, wants_visual};

pub struct Agent {
    pub memory: Memory,
    pub tasks: TaskQueue,
    pub tools: ToolHost,
    pub mesh: Mesh,
    pub persona: String,
    pub pending_confirm: Option<PendingConfirm>,
    pub model_name: String,
    voice: Voice,
}

pub struct PendingConfirm {
    pub id: String,
    pub tool: String,
    pub args: String,
    pub lang: Lang,
}

pub struct Shared(pub Arc<Mutex<Agent>>);

/// Send-safe handle: sqlite lives on one thread.
pub struct AgentHandle {
    tx: tokio::sync::mpsc::Sender<(ClientMessage, tokio::sync::oneshot::Sender<Vec<ServerMessage>>)>,
    pub model_name: String,
    pub local_device: jarvis_protocol::DeviceInfo,
}

impl AgentHandle {
    pub fn spawn(repo_root: PathBuf) -> Result<Self> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(
            ClientMessage,
            tokio::sync::oneshot::Sender<Vec<ServerMessage>>,
        )>(32);
        let agent = Agent::open(repo_root)?;
        let model_name = agent.model_name.clone();
        let local_device = agent.mesh.local.clone();
        std::thread::Builder::new()
            .name("jarvis-agent".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("rt");
                let mut agent = agent;
                rt.block_on(async move {
                    while let Some((msg, reply)) = rx.recv().await {
                        let out = agent.handle(msg).await;
                        let _ = reply.send(out);
                    }
                });
            })?;
        Ok(Self {
            tx,
            model_name,
            local_device,
        })
    }

    pub async fn handle(&self, msg: ClientMessage) -> Vec<ServerMessage> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if self.tx.send((msg, tx)).await.is_err() {
            return vec![ServerMessage::Error {
                message: "agent thread dead".into(),
            }];
        }
        rx.await.unwrap_or_else(|_| {
            vec![ServerMessage::Error {
                message: "agent dropped".into(),
            }]
        })
    }

    pub fn presence_hello(&self) -> Vec<ServerMessage> {
        vec![
            ServerMessage::DeviceHello {
                device: self.local_device.clone(),
            },
            ServerMessage::Presence {
                io_device: self.local_device.id.clone(),
                leader: self.local_device.id.clone(),
                devices: vec![self.local_device.clone()],
            },
        ]
    }
}

impl Agent {
    pub fn open(repo_root: PathBuf) -> Result<Self> {
        let data = repo_root.join("data");
        std::fs::create_dir_all(&data)?;
        let persona = std::fs::read_to_string(repo_root.join("skills/persona.md"))
            .unwrap_or_else(|_| "You are Jarvis. Reply in the user's language.".into());
        Ok(Self {
            memory: Memory::open(&data.join("memory.sqlite"))?,
            tasks: TaskQueue::open(&data.join("tasks.sqlite"))?,
            tools: ToolHost::new(repo_root.clone()),
            mesh: Mesh::new(),
            persona,
            pending_confirm: None,
            model_name: std::env::var("JARVIS_LOCAL_LLM_MODEL")
                .or_else(|_| std::env::var("OPENROUTER_MODEL"))
                .unwrap_or_else(|_| "unknown".into()),
            voice: Voice::from_env(&repo_root),
        })
    }

    pub async fn handle(&mut self, msg: ClientMessage) -> Vec<ServerMessage> {
        match msg {
            ClientMessage::Ping {} => vec![ServerMessage::Pong {}],
            ClientMessage::Hello { device } => {
                self.mesh.hello(device.clone());
                vec![
                    ServerMessage::DeviceHello { device },
                    self.presence(),
                ]
            }
            ClientMessage::HandoffRequest { target_device } => {
                let io_ok = self.mesh.handoff_io(&target_device);
                let _ = self.mesh.handoff_leader(&target_device);
                if io_ok {
                    let turns = self
                        .memory
                        .recent_turns(20)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(role, content, lang)| ChatTurn {
                            role,
                            content,
                            lang: if lang == "pl" { Lang::Pl } else { Lang::En },
                        })
                        .collect();
                    vec![
                        ServerMessage::HandoffReady {
                            snapshot: self.mesh.snapshot(turns),
                        },
                        self.presence(),
                    ]
                } else {
                    vec![ServerMessage::Error {
                        message: format!("unknown device {target_device}"),
                    }]
                }
            }
            ClientMessage::PullCore {} => {
                let path = self.tools.repo_root.join("releases/manifest.json");
                match jarvis_update::load_manifest(&path) {
                    Ok(m) => vec![ServerMessage::CoreUpdate {
                        version: m.core_version,
                    }],
                    Err(e) => vec![ServerMessage::Error {
                        message: e.to_string(),
                    }],
                }
            }
            ClientMessage::Confirm { id, accepted } => {
                if let Some(p) = self.pending_confirm.take() {
                    if p.id == id && accepted {
                        match self.tools.run(&p.tool, &p.args, true) {
                            Ok(r) => self.spoken_reply(id, r.output, p.lang),
                            Err(e) => vec![ServerMessage::Error {
                                message: e.to_string(),
                            }],
                        }
                    } else {
                        let content = match p.lang {
                            Lang::Pl => "Anulowano.".to_string(),
                            Lang::En => "Cancelled.".to_string(),
                        };
                        self.spoken_reply(id, content, p.lang)
                    }
                } else {
                    vec![ServerMessage::Error {
                        message: "no pending confirm".into(),
                    }]
                }
            }
            ClientMessage::DismissVisual {} => vec![],
            ClientMessage::Text { id, content, lang, device_id } => {
                if let Some(d) = device_id {
                    self.mesh.claim_io(&d);
                }
                self.handle_user_text(id, content, lang).await
            }
            ClientMessage::Utterance {
                id,
                transcript,
                audio_b64: _,
                device_id,
            } => {
                if let Some(d) = device_id {
                    self.mesh.claim_io(&d);
                }
                let content = transcript.unwrap_or_default();
                self.handle_user_text(id, content, None).await
            }
        }
    }

    async fn handle_user_text(
        &mut self,
        id: String,
        content: String,
        lang_hint: Option<Lang>,
    ) -> Vec<ServerMessage> {
        let lang = lang_hint.unwrap_or_else(|| detect_lang(&content));
        let _ = self.memory.log_turn("user", &content, lang.as_str());

        if let Some(deferred) = maybe_defer_desktop(&content, lang, &self.mesh) {
            let job = self
                .tasks
                .enqueue(&content, "deferred until desktop")
                .ok();
            if let Some(j) = &job {
                let _ = self.tasks.defer(&j.id, "desktop", &deferred);
            }
            return vec![ServerMessage::JobDeferred {
                job_id: job.map(|j| j.id).unwrap_or_default(),
                until: "desktop".into(),
                message: deferred,
            }];
        }

        if let Some(tool_hit) = heuristic_tool(&content) {
            return self.exec_tool(id, lang, tool_hit.0, &tool_hit.1);
        }

        let visual = if wants_visual(&content) {
            Some(visual_from_prompt(&content))
        } else {
            None
        };

        match complete_llm(&self.persona, &self.memory, &content, lang, visual.is_some()).await {
            Ok(reply) => {
                if let Some((tool, args)) = parse_tool_tag(&reply) {
                    return self.exec_tool(id, lang, tool, &args);
                }
                let spec = parse_visual_tag(&reply).or(visual);
                let clean = strip_visual_tag(&reply);
                let _ = self.memory.log_turn("assistant", &clean, lang.as_str());
                let mut out = self.spoken_reply(id.clone(), clean, lang);
                if let Some(spec) = spec {
                    out.push(ServerMessage::Visual { id, spec, lang });
                }
                out
            }
            Err(e) => {
                let fallback = if visual.is_some() {
                    match lang {
                        Lang::Pl => "Wyświetlam hologram.".into(),
                        Lang::En => "Projecting hologram.".into(),
                    }
                } else {
                    local_fallback(&content, lang)
                };
                let _ = self.memory.log_turn("assistant", &fallback, lang.as_str());
                let mut out = self.spoken_reply(id.clone(), fallback, lang);
                out.push(ServerMessage::Error {
                    message: format!("llm: {e}"),
                });
                if let Some(spec) = visual {
                    out.push(ServerMessage::Visual { id, spec, lang });
                    // drop llm error noise when we still have a hologram
                    out.retain(|m| !matches!(m, ServerMessage::Error { .. }));
                }
                out
            }
        }
    }

    fn exec_tool(&mut self, id: String, lang: Lang, tool: String, args: &str) -> Vec<ServerMessage> {
        match self.tools.run(&tool, args, false) {
            Ok(r) if r.needs_confirm => {
                self.pending_confirm = Some(PendingConfirm {
                    id: id.clone(),
                    tool,
                    args: args.to_string(),
                    lang,
                });
                vec![ServerMessage::Confirm {
                    id,
                    prompt: r.confirm_prompt.unwrap_or_default(),
                    lang,
                }]
            }
            Ok(r) => {
                let prefix = match lang {
                    Lang::Pl => "Zrobione: ",
                    Lang::En => "Done: ",
                };
                let content = format!("{prefix}{}", r.output);
                let _ = self.memory.log_turn("assistant", &content, lang.as_str());
                self.spoken_reply(id, content, lang)
            }
            Err(e) => vec![ServerMessage::Error {
                message: e.to_string(),
            }],
        }
    }

    pub fn presence(&self) -> ServerMessage {
        ServerMessage::Presence {
            io_device: self.mesh.io_device.clone(),
            leader: self.mesh.leader.clone(),
            devices: self.mesh.list(),
        }
    }

    fn spoken_reply(&self, id: String, content: String, lang: Lang) -> Vec<ServerMessage> {
        let mut out = vec![ServerMessage::Reply {
            id: id.clone(),
            content: content.clone(),
            lang,
        }];
        let cloud = std::env::var("JARVIS_KIND").ok().as_deref() == Some("cloud");
        let or_key = std::env::var("OPENROUTER_API_KEY")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        if cloud && or_key.is_none() {
            return out;
        }
        match self.voice.speak_audio_b64(&content, lang) {
            Ok((mime, audio_b64)) => {
                out.push(ServerMessage::Speech {
                    id,
                    mime,
                    audio_b64,
                });
            }
            Err(e) => tracing::warn!("tts skipped: {e}"),
        }
        out
    }
}

fn maybe_defer_desktop(content: &str, lang: Lang, mesh: &Mesh) -> Option<String> {
    let needs = content.to_lowercase();
    let rewrite = needs.contains("przepis") || needs.contains("rewrite") || needs.contains("jądro")
        || needs.contains("kernel") && needs.contains("compile");
    let desktop_alive = mesh.list().iter().any(|d| {
        matches!(
            d.kind,
            jarvis_protocol::DeviceKind::Windows | jarvis_protocol::DeviceKind::LinuxDesktop
        )
    });
    if rewrite && !desktop_alive {
        Some(match lang {
            Lang::Pl => {
                "Desktop śpi; zacznę kompilację / przepisanie jądra, gdy wieża wstanie.".into()
            }
            Lang::En => {
                "The desktop is asleep; I will start compiling / rewriting the core when it wakes."
                    .into()
            }
        })
    } else {
        None
    }
}

fn heuristic_tool(content: &str) -> Option<(String, String)> {
    let c = content.to_lowercase();
    if c.contains("notatk") || c.contains("note ") || c.starts_with("note:") {
        return Some(("vault_write".into(), content.to_string()));
    }
    if c.contains("kalendarz") || c.contains("calendar") || c.contains("wydarzen") {
        return Some(("calendar_add".into(), content.to_string()));
    }
    if c.contains("otwórz")
        || c.contains("otworz")
        || c.contains("uruchom")
        || c.contains("włącz")
        || c.contains("wlacz")
        || c.contains("odpal")
        || c.contains("open ")
        || c.contains("launch ")
        || c.starts_with("start ")
    {
        let app = content
            .split_once("otwórz")
            .or_else(|| content.split_once("otworz"))
            .or_else(|| content.split_once("uruchom"))
            .or_else(|| content.split_once("włącz"))
            .or_else(|| content.split_once("wlacz"))
            .or_else(|| content.split_once("odpal"))
            .or_else(|| content.split_once("launch "))
            .or_else(|| content.split_once("open "))
            .or_else(|| content.split_once("start "))
            .map(|(_, r)| r.trim().trim_matches(|ch: char| ch == '.' || ch == '!' || ch == '?').to_string())
            .unwrap_or_else(|| content.to_string());
        return Some(("open_app".into(), app));
    }
    if c.contains("git push --force") || c.contains("push --force") {
        return Some(("shell".into(), content.to_string()));
    }
    None
}

fn parse_tool_tag(reply: &str) -> Option<(String, String)> {
    let re = regex::Regex::new(r"\[\[tool:([a-z_]+)\|([\s\S]*?)\]\]").ok()?;
    let cap = re.captures(reply)?;
    Some((cap[1].to_string(), cap[2].trim().to_string()))
}

fn local_fallback(content: &str, lang: Lang) -> String {
    match lang {
        Lang::Pl => format!(
            "Nie mam teraz modelu LLM, ale przyjęłem: „{content}”. Uruchom Bionic (localhost:1234) albo ustaw OPENROUTER_API_KEY."
        ),
        Lang::En => format!(
            "No LLM is reachable, but I noted: “{content}”. Start Bionic on localhost:1234 or set OPENROUTER_API_KEY."
        ),
    }
}

fn strip_visual_tag(reply: &str) -> String {
    if let Some(start) = reply.find("[[visual:") {
        if let Some(rel_end) = reply[start..].find("]]") {
            let mut s = String::new();
            s.push_str(&reply[..start]);
            s.push_str(&reply[start + rel_end + 2..]);
            return s.trim().to_string();
        }
    }
    reply.trim().to_string()
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

async fn complete_llm(
    persona: &str,
    memory: &Memory,
    user: &str,
    lang: Lang,
    want_visual: bool,
) -> Result<String> {
    let mut messages = vec![json!({
        "role": "system",
        "content": format!(
            "{persona}\nLanguage for this turn: {}.\nYou may emit a tool call as [[tool:name|args]] using vault_write, calendar_add, open_app, list_vault, shell.\n{}\nIf you emit a visual, use [[visual:JSON]] matching VisualSpec (kind scene3d|slides|diagram|video, title, scene3d.bodies with optional orbit).",
            lang.as_str(),
            if want_visual {
                "The HUD will already project a hologram. You may refine it with [[visual:{...}]]. Speak briefly about what is shown."
            } else {
                "If the user asks to see / draw / present something, emit [[visual:JSON]] as well as a short reply."
            }
        )
    })];
    if let Ok(facts) = memory.all_facts() {
        if !facts.is_empty() {
            let blob = facts
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; ");
            messages.push(json!({"role":"system","content": format!("Known facts: {blob}")}));
        }
    }
    if let Ok(turns) = memory.recent_turns(8) {
        for (role, content, _) in turns {
            messages.push(json!({"role": role, "content": content}));
        }
    }
    messages.push(json!({"role":"user","content": user}));

    let cloud = std::env::var("JARVIS_KIND").ok().as_deref() == Some("cloud");
    let local_url = env_nonempty("JARVIS_LOCAL_LLM_URL");
    let try_bionic = !cloud && local_url.is_some();

    if try_bionic {
        let url = local_url.unwrap();
        let model = env_nonempty("JARVIS_LOCAL_LLM_MODEL").unwrap_or_else(|| "local-model".into());
        match chat_completions(&url, None, &model, &messages, true).await {
            Ok(text) => {
                tracing::info!("llm bionic {model}");
                return Ok(text);
            }
            Err(e) => {
                tracing::warn!("Bionic/LM Studio down ({e}); falling back to OpenRouter");
            }
        }
    }

    let Some(key) = env_nonempty("OPENROUTER_API_KEY") else {
        anyhow::bail!("no LLM: Bionic unreachable and OPENROUTER_API_KEY is empty");
    };
    let model = env_nonempty("OPENROUTER_MODEL").unwrap_or_else(|| "openrouter/free".into());
    match chat_completions(
        "https://openrouter.ai/api/v1",
        Some(&key),
        &model,
        &messages,
        false,
    )
    .await
    {
        Ok(text) => {
            tracing::info!("llm openrouter {model}");
            Ok(text)
        }
        Err(e) if model != "openrouter/free" => {
            tracing::warn!("OpenRouter {model} failed ({e}); retry openrouter/free");
            chat_completions(
                "https://openrouter.ai/api/v1",
                Some(&key),
                "openrouter/free",
                &messages,
                false,
            )
            .await
        }
        Err(e) => Err(e),
    }
}

async fn chat_completions(
    base: &str,
    api_key: Option<&str>,
    model: &str,
    messages: &[serde_json::Value],
    local: bool,
) -> Result<String> {
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));
    let mut builder = reqwest::Client::builder();
    if local {
        builder = builder
            .connect_timeout(std::time::Duration::from_secs(2))
            .timeout(std::time::Duration::from_secs(12));
    } else {
        builder = builder.timeout(std::time::Duration::from_secs(90));
    }
    let client = builder.build()?;
    let mut req = client.post(&url).json(&json!({
        "model": model,
        "messages": messages,
        "temperature": 0.4,
    }));
    if let Some(k) = api_key {
        req = req
            .header("Authorization", format!("Bearer {k}"))
            .header("HTTP-Referer", "https://jarvis.local")
            .header("X-Title", "Jarvis");
    }
    let resp = req.send().await.context(if local {
        "connect Bionic/LM Studio"
    } else {
        "connect OpenRouter"
    })?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        anyhow::bail!("http {status}: {}", body.chars().take(280).collect::<String>());
    }
    let v: serde_json::Value = serde_json::from_str(&body).context("llm json")?;
    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();
    if text.is_empty() {
        anyhow::bail!("empty completion");
    }
    Ok(text)
}
