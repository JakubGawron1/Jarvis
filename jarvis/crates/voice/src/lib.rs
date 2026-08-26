//! STT/TTS wrappers. Binaries are optional; text channel always works.
//!
//! Speak order (desktop): Piper / Kokoro if installed, otherwise OpenRouter
//! free TTS (same `OPENROUTER_API_KEY`). SAPI / espeak-ng only if OpenRouter
//! is unset or fails. Cloud is OpenRouter only.
use anyhow::{bail, Context, Result};
use base64::Engine;
use jarvis_protocol::Lang;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_CHARS: usize = 800;
const OPENROUTER_SPEECH: &str = "https://openrouter.ai/api/v1/audio/speech";
/// Multilingual free TTS (PL + EN). Best default for Render.
const FISH_FREE: &str = "fish-audio/s2.1-pro-free:free";
/// English-only free TTS; British male `flux-sean-en`.
const FLUX_FREE: &str = "deepgram/flux-tts:free";
const FLUX_VOICE_EN: &str = "flux-sean-en";

pub struct Voice {
    pub whisper: Option<PathBuf>,
    pub piper: Option<PathBuf>,
    pub piper_pl: Option<PathBuf>,
    pub piper_en: Option<PathBuf>,
    pub kokoro_url: Option<String>,
    openrouter_key: Option<String>,
    openrouter_model: String,
    openrouter_model_en: Option<String>,
    openrouter_voice_en: Option<String>,
}

impl Voice {
    pub fn from_env(repo_root: &Path) -> Self {
        let vendor = repo_root.join("vendor").join("piper");
        let piper = env_path("PIPER_BIN").or_else(|| first_existing(&[
            vendor.join("piper.exe"),
            vendor.join("piper"),
        ]));
        let piper_pl = env_path("PIPER_VOICE_PL").or_else(|| find_onnx(&vendor, "pl_PL"));
        let piper_en = env_path("PIPER_VOICE_EN")
            .or_else(|| find_onnx(&vendor, "en_GB"))
            .or_else(|| find_onnx(&vendor, "en_US"));
        Self {
            whisper: env_path("WHISPER_BIN"),
            piper,
            piper_pl,
            piper_en,
            kokoro_url: env_nonempty("KOKORO_URL"),
            openrouter_key: env_nonempty("OPENROUTER_API_KEY"),
            openrouter_model: env_nonempty("OPENROUTER_TTS_MODEL").unwrap_or_else(|| FISH_FREE.into()),
            openrouter_model_en: env_nonempty("OPENROUTER_TTS_MODEL_EN"),
            openrouter_voice_en: env_nonempty("OPENROUTER_TTS_VOICE_EN"),
        }
    }

    pub fn transcribe_file(&self, wav: &str) -> Result<String> {
        let Some(bin) = &self.whisper else {
            bail!("WHISPER_BIN not set");
        };
        let out = Command::new(bin).args(["-f", wav, "-nt"]).output()?;
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Audio bytes as base64 for the `speech` WS frame (`audio/wav` or `audio/mpeg`).
    pub fn speak_audio_b64(&self, text: &str, lang: Lang) -> Result<(String, String)> {
        let (mime, bytes) = self.speak_bytes(text, lang)?;
        if bytes.len() < 32 {
            bail!("tts produced empty audio");
        }
        Ok((
            mime,
            base64::engine::general_purpose::STANDARD.encode(bytes),
        ))
    }

    /// WAV bytes as base64, ready for the `speech` WS frame.
    pub fn speak_wav_b64(&self, text: &str, lang: Lang) -> Result<(String, String)> {
        self.speak_audio_b64(text, lang)
    }

    fn has_kokoro(&self) -> bool {
        self.kokoro_url.is_some()
    }

    fn has_piper(&self, lang: Lang) -> bool {
        self.piper.is_some()
            && match lang {
                Lang::En => self.piper_en.is_some(),
                Lang::Pl => self.piper_pl.is_some(),
            }
    }

    fn speak_bytes(&self, text: &str, lang: Lang) -> Result<(String, Vec<u8>)> {
        let clipped = clip_tts(text);
        if clipped.is_empty() {
            bail!("nothing to speak");
        }
        if is_cloud() {
            return self.openrouter_or_fish(clipped, lang);
        }

        // Movie-like neural voice first. Piper/SAPI sound like a toy synth.
        if self.openrouter_key.is_some() {
            match self.openrouter_or_fish(clipped, lang) {
                Ok(audio) => return Ok(audio),
                Err(e) => tracing::warn!("openrouter TTS failed ({e}); local engines"),
            }
        }

        if lang == Lang::En {
            if let Some(url) = &self.kokoro_url {
                let ts = now_ts()?;
                let out = std::env::temp_dir().join(format!("jarvis-{ts}.wav"));
                match self.kokoro_speak(url, clipped, &out) {
                    Ok(()) => return read_audio_file(&out),
                    Err(e) => tracing::info!("kokoro TTS failed ({e})"),
                }
            }
        }
        if self.has_piper(lang) {
            let voice = match lang {
                Lang::En => self.piper_en.as_ref(),
                Lang::Pl => self.piper_pl.as_ref(),
            };
            match self.piper_to_file(clipped, voice) {
                Ok(p) => return read_audio_file(&p),
                Err(e) => tracing::info!("piper TTS failed ({e})"),
            }
        }

        if self.openrouter_key.is_some() {
            tracing::info!("local TTS engines failed — OpenRouter already tried");
        }

        let ts = now_ts()?;
        let out = std::env::temp_dir().join(format!("jarvis-{ts}.wav"));
        host_speak(clipped, lang, &out)?;
        read_audio_file(&out)
    }

    fn openrouter_or_fish(&self, text: &str, lang: Lang) -> Result<(String, Vec<u8>)> {
        self.openrouter_tts(text, lang).or_else(|e| {
            tracing::warn!("openrouter TTS failed ({e}); retry {FISH_FREE}");
            self.openrouter_post(FISH_FREE, None, &fish_input(lang, text))
        })
    }

    pub fn speak(&self, text: &str, lang: Lang) -> Result<PathBuf> {
        let ts = now_ts()?;
        let out = std::env::temp_dir().join(format!("jarvis-{ts}.wav"));
        let (mime, bytes) = self.speak_bytes(text, lang)?;
        if mime == "audio/mpeg" {
            let mp3 = out.with_extension("mp3");
            std::fs::write(&mp3, bytes)?;
            return Ok(mp3);
        }
        std::fs::write(&out, bytes)?;
        Ok(out)
    }

    fn openrouter_tts(&self, text: &str, lang: Lang) -> Result<(String, Vec<u8>)> {
        let (model, voice, input) = self.openrouter_request(text, lang);
        self.openrouter_post(&model, voice.as_deref(), &input)
    }

    fn openrouter_post(&self, model: &str, voice: Option<&str>, input: &str) -> Result<(String, Vec<u8>)> {
        let key = self
            .openrouter_key
            .clone()
            .ok_or_else(|| anyhow::anyhow!("OPENROUTER_API_KEY missing — needed for cloud TTS"))?;
        let model = model.to_string();
        let voice = voice.map(str::to_string);
        let input = input.to_string();
        run_off_async(move || openrouter_post_inner(&key, &model, voice.as_deref(), &input))
    }

    fn openrouter_request(&self, text: &str, lang: Lang) -> (String, Option<String>, String) {
        // One cinematic voice family (Fish) for PL+EN — closer to film Jarvis than Flux/SAPI.
        let model = if self.openrouter_model.contains("flux") {
            FISH_FREE.into()
        } else {
            self.openrouter_model.clone()
        };
        if model.contains("fish-audio") {
            (model, None, fish_input(lang, text))
        } else if lang == Lang::En {
            let flux = self.openrouter_model_en.as_deref().unwrap_or(FLUX_FREE);
            if flux.contains("flux") {
                let voice = self
                    .openrouter_voice_en
                    .clone()
                    .unwrap_or_else(|| FLUX_VOICE_EN.into());
                (flux.to_string(), Some(voice), text.to_string())
            } else {
                (model, self.openrouter_voice_en.clone(), text.to_string())
            }
        } else {
            (model, self.openrouter_voice_en.clone(), text.to_string())
        }
    }

    fn piper_to_file(&self, text: &str, voice: Option<&PathBuf>) -> Result<PathBuf> {
        let ts = now_ts()?;
        let out = std::env::temp_dir().join(format!("jarvis-{ts}.wav"));
        self.piper_speak(text, voice, &out)?;
        Ok(out)
    }

    fn kokoro_speak(&self, base: &str, text: &str, out: &Path) -> Result<()> {
        let url = speech_endpoint(base);
        let text = text.to_string();
        let out = out.to_path_buf();
        run_off_async(move || {
            let body = serde_json::json!({
                "model": "kokoro",
                "voice": "bm_george",
                "input": text,
                "response_format": "wav",
            });
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()?;
            let resp = client.post(&url).json(&body).send()?.error_for_status()?;
            let bytes = resp.bytes()?;
            std::fs::write(&out, bytes)?;
            tracing::info!("tts kokoro → {}", out.display());
            Ok(())
        })
    }

    fn piper_speak(&self, text: &str, voice: Option<&PathBuf>, out: &Path) -> Result<()> {
        let Some(bin) = &self.piper else {
            bail!("PIPER_BIN not set");
        };
        let Some(voice) = voice else {
            bail!("piper voice model missing");
        };
        let mut cmd = Command::new(bin);
        cmd.arg("--model")
            .arg(voice)
            .arg("--output_file")
            .arg(out)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        hide_window(&mut cmd);
        let mut child = cmd.spawn().context("spawn piper")?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
            stdin.write_all(b"\n")?;
        }
        let status = child.wait()?;
        if !status.success() || !out.exists() {
            bail!("piper failed");
        }
        tracing::info!("tts piper → {}", out.display());
        Ok(())
    }
}

fn host_speak(text: &str, lang: Lang, out: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        return sapi_speak(text, lang, out);
    }
    #[cfg(not(windows))]
    {
        return espeak_speak(text, lang, out);
    }
}

#[cfg(windows)]
fn sapi_speak(text: &str, lang: Lang, out: &Path) -> Result<()> {
    let txt = out.with_extension("txt");
    let ps1 = out.with_extension("ps1");
    std::fs::write(&txt, text)?;
    let culture = match lang {
        Lang::Pl => "pl",
        Lang::En => "en-GB",
    };
    let fallback = match lang {
        Lang::Pl => "pl",
        Lang::En => "en",
    };
    let script = format!(
        r#"$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$s = New-Object System.Speech.Synthesis.SpeechSynthesizer
$want = '{culture}'
$fb = '{fallback}'
foreach ($v in $s.GetInstalledVoices()) {{
  $n = $v.VoiceInfo.Culture.Name
  if ($n.StartsWith($want, [StringComparison]::OrdinalIgnoreCase) -or $n.StartsWith($fb, [StringComparison]::OrdinalIgnoreCase)) {{
    $s.SelectVoice($v.VoiceInfo.Name)
    if ($n.StartsWith($want, [StringComparison]::OrdinalIgnoreCase)) {{ break }}
  }}
}}
$t = Get-Content -Raw -Encoding UTF8 -LiteralPath {txt}
$s.SetOutputToWaveFile({out})
$s.Speak($t)
$s.Dispose()
"#,
        culture = culture,
        fallback = fallback,
        txt = ps_literal(&txt),
        out = ps_literal(out),
    );
    std::fs::write(&ps1, script)?;
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
    ])
    .arg(&ps1)
    .stdout(Stdio::null())
    .stderr(Stdio::piped());
    hide_window(&mut cmd);
    let status = cmd.status().context("powershell SAPI")?;
    let _ = std::fs::remove_file(&txt);
    let _ = std::fs::remove_file(&ps1);
    if !status.success() || !out.exists() {
        bail!("Windows SAPI TTS failed");
    }
    tracing::info!("tts SAPI → {}", out.display());
    Ok(())
}

#[cfg(not(windows))]
fn espeak_speak(text: &str, lang: Lang, out: &Path) -> Result<()> {
    let voice = match lang {
        Lang::Pl => "pl",
        Lang::En => "en-gb",
    };
    let bin = ["espeak-ng", "espeak"]
        .into_iter()
        .find(|b| Command::new(b).arg("--version").output().is_ok())
        .ok_or_else(|| anyhow::anyhow!("espeak-ng not installed and PIPER_BIN unset"))?;
    let status = Command::new(bin)
        .args(["-v", voice, "-w"])
        .arg(out)
        .arg(text)
        .status()?;
    if !status.success() || !out.exists() {
        bail!("{bin} failed");
    }
    tracing::info!("tts {bin} → {}", out.display());
    Ok(())
}

fn run_off_async<T, F>(f: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("jarvis-tts-http".into())
        .spawn(move || {
            let _ = tx.send(f());
        })
        .context("spawn tts http")?;
    rx.recv().context("tts http thread")?
}

fn openrouter_post_inner(
    key: &str,
    model: &str,
    voice: Option<&str>,
    input: &str,
) -> Result<(String, Vec<u8>)> {
    let mut body = serde_json::json!({
        "model": model,
        "input": input,
        "response_format": "mp3",
    });
    if let Some(v) = voice {
        body["voice"] = serde_json::json!(v);
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()?;
    let resp = client
        .post(OPENROUTER_SPEECH)
        .bearer_auth(key)
        .header("HTTP-Referer", "https://github.com/local/jarvis")
        .header("X-Title", "Jarvis")
        .json(&body)
        .send()
        .context("openrouter tts http")?;
    let status = resp.status();
    if !status.is_success() {
        let err = resp.text().unwrap_or_default();
        bail!("openrouter tts {status}: {err}");
    }
    let ctype = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("audio/mpeg")
        .to_string();
    let bytes = resp.bytes()?.to_vec();
    let mime = if ctype.contains("wav") || bytes.starts_with(b"RIFF") {
        "audio/wav"
    } else {
        "audio/mpeg"
    };
    tracing::info!("tts openrouter {model} ({mime}, {} bytes)", bytes.len());
    Ok((mime.into(), bytes))
}

fn fish_input(lang: Lang, text: &str) -> String {
    match lang {
        Lang::En => format!(
            "[JARVIS, Iron Man holographic AI, calm British butler, warm precise baritone, slight digital sheen, cinematic, never a toy synthesizer] {text}"
        ),
        Lang::Pl => format!(
            "[JARVIS z Iron Man, spokojny brytyjski butler mówiący po polsku, ciepły precyzyjny baryton, lekko holograficzny, filmowy, nigdy jak syntezator] {text}"
        ),
    }
}

fn is_cloud() -> bool {
    std::env::var("JARVIS_KIND").ok().as_deref() == Some("cloud")
}

fn now_ts() -> Result<u128> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
}

fn read_audio_file(path: &Path) -> Result<(String, Vec<u8>)> {
    let bytes = std::fs::read(path).with_context(|| path.display().to_string())?;
    let _ = std::fs::remove_file(path);
    let mime = if bytes.starts_with(b"RIFF") {
        "audio/wav"
    } else {
        "audio/mpeg"
    };
    Ok((mime.into(), bytes))
}

fn clip_tts(text: &str) -> &str {
    let t = text.trim();
    if t.chars().count() <= MAX_CHARS {
        return t;
    }
    let end = t.char_indices().nth(MAX_CHARS).map(|(i, _)| i).unwrap_or(t.len());
    t.get(..end).unwrap_or(t)
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn env_path(key: &str) -> Option<PathBuf> {
    env_nonempty(key).map(PathBuf::from)
}

fn first_existing(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|p| p.is_file()).cloned()
}

fn find_onnx(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let search = [dir.to_path_buf(), dir.join("voices")];
    for d in search {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        let mut found: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension().and_then(|e| e.to_str()) == Some("onnx")
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with(prefix))
            })
            .collect();
        found.sort();
        if let Some(p) = found.into_iter().next() {
            return Some(p);
        }
    }
    None
}

fn speech_endpoint(base: &str) -> String {
    let b = base.trim().trim_end_matches('/');
    if b.ends_with("/audio/speech") {
        b.to_string()
    } else {
        format!("{b}/v1/audio/speech")
    }
}

fn ps_literal(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "''"))
}

fn hide_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = cmd;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_long_text() {
        let s = "ą".repeat(900);
        assert!(clip_tts(&s).chars().count() <= MAX_CHARS);
    }

    #[test]
    fn kokoro_endpoint() {
        assert_eq!(
            speech_endpoint("http://127.0.0.1:8880"),
            "http://127.0.0.1:8880/v1/audio/speech"
        );
    }

    #[test]
    fn no_local_engines_without_env() {
        let dir = std::env::temp_dir().join("jarvis-voice-empty");
        let _ = std::fs::create_dir_all(&dir);
        let v = Voice::from_env(&dir);
        assert!(!v.has_kokoro());
        assert!(!v.has_piper(Lang::Pl));
        assert!(!v.has_piper(Lang::En));
    }
}
