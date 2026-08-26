use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use jarvis_core::AgentHandle;
use jarvis_protocol::{ClientMessage, ServerMessage, CORE_VERSION, DEFAULT_BIND};
use std::path::PathBuf;
use std::sync::Arc;
use sysinfo::System;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

struct AppState {
    agent: AgentHandle,
    tx: broadcast::Sender<String>,
}

fn main() -> anyhow::Result<()> {
    load_dotenv();
    async_main()
}

#[tokio::main]
async fn async_main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "jarvis_daemon=info,jarvis_core=info".into()),
        )
        .init();

    load_dotenv();
    let root = repo_root();
    let agent = AgentHandle::spawn(root)?;
    let bind = bind_addr();
    let token = std::env::var("JARVIS_PAIRING_TOKEN").unwrap_or_default();

    let (tx, _) = broadcast::channel(64);
    let state = Arc::new(AppState { agent, tx });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(health))
        .route("/stats", get(stats))
        .layer(CorsLayer::permissive())
        .with_state(state);

    tracing::info!("jarvisd {CORE_VERSION} listening on {bind}");
    if !token.is_empty() {
        tracing::info!("pairing token required");
    }
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true, "version": CORE_VERSION }))
}

async fn stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut sys = System::new_all();
    sys.refresh_all();
    let cpu = sys.global_cpu_usage();
    let ram_used = sys.used_memory();
    let ram_total = sys.total_memory();
    Json(ServerMessage::Stats {
        cpu,
        ram_used,
        ram_total,
        model: state.agent.model_name.clone(),
        core_version: CORE_VERSION.into(),
    })
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let token = std::env::var("JARVIS_PAIRING_TOKEN").unwrap_or_default();
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    {
        for m in state.agent.presence_hello() {
            let _ = sender.send(Message::Text(m.to_json().into())).await;
        }
    }

    loop {
        tokio::select! {
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Text(t))) => {
                        if !token.is_empty() {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
                                if v.get("token").and_then(|x| x.as_str()) != Some(token.as_str())
                                    && v.get("pairing_token").and_then(|x| x.as_str()) != Some(token.as_str())
                                {
                                    let err = ServerMessage::Error { message: "unauthorized".into() };
                                    let _ = sender.send(Message::Text(err.to_json().into())).await;
                                    continue;
                                }
                            }
                        }
                        match ClientMessage::parse(&t) {
                            Ok(msg) => {
                                let replies = state.agent.handle(msg).await;
                                for r in replies {
                                    let json = r.to_json();
                                    if fanout_all(&r) {
                                        // Presence / mesh — every HUD. This socket is already on `rx`.
                                        if state.tx.send(json.clone()).is_err() {
                                            if sender.send(Message::Text(json.into())).await.is_err() {
                                                return;
                                            }
                                        }
                                    } else if sender.send(Message::Text(json.into())).await.is_err() {
                                        // Reply / speech / visual — only the device that asked.
                                        return;
                                    }
                                }
                            }
                            Err(e) => {
                                let err = ServerMessage::Error { message: e.to_string() };
                                let _ = sender.send(Message::Text(err.to_json().into())).await;
                            }
                        }
                    }
                    Some(Ok(Message::Ping(p))) => {
                        let _ = sender.send(Message::Pong(p)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => return,
                    Some(Err(_)) => return,
                    _ => {}
                }
            }
            Ok(broadcasted) = rx.recv() => {
                if sender.send(Message::Text(broadcasted.into())).await.is_err() {
                    return;
                }
            }
        }
    }
}

fn repo_root() -> PathBuf {
    if let Ok(p) = std::env::var("JARVIS_ROOT") {
        return PathBuf::from(p);
    }
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for _ in 0..6 {
        if dir.join("skills/persona.md").exists() {
            return dir;
        }
        if !dir.pop() {
            break;
        }
    }
    PathBuf::from("..")
}

/// Bind order: `JARVIS_BIND` → `PORT` (Render) → `0.0.0.0:7420` in cloud → localhost default.
fn bind_addr() -> String {
    bind_addr_from(
        std::env::var("JARVIS_BIND").ok().as_deref(),
        std::env::var("PORT").ok().as_deref(),
        std::env::var("JARVIS_KIND").ok().as_deref(),
    )
}

fn bind_addr_from(jarvis_bind: Option<&str>, port: Option<&str>, kind: Option<&str>) -> String {
    // Copied desktop `.env` often has JARVIS_BIND=127.0.0.1:7420. On Render that
    // makes health checks hit the wrong socket → `x-render-routing: no-server`.
    if kind == Some("cloud") {
        if let Some(port) = port {
            if !port.trim().is_empty() {
                return format!("0.0.0.0:{port}");
            }
        }
        return "0.0.0.0:7420".into();
    }
    if let Some(bind) = jarvis_bind {
        if !bind.trim().is_empty() {
            return bind.to_string();
        }
    }
    if let Some(port) = port {
        if !port.trim().is_empty() {
            return format!("0.0.0.0:{port}");
        }
    }
    DEFAULT_BIND.into()
}

fn fanout_all(msg: &ServerMessage) -> bool {
    matches!(
        msg,
        ServerMessage::Presence { .. }
            | ServerMessage::DeviceHello { .. }
            | ServerMessage::DeviceLost { .. }
            | ServerMessage::Stats { .. }
            | ServerMessage::CoreWaking {}
            | ServerMessage::CoreUpdate { .. }
            | ServerMessage::HandoffReady { .. }
            | ServerMessage::Pong {}
    )
}

fn load_dotenv() {
    let candidates = [PathBuf::from(".env"), repo_root().join(".env")];
    for p in candidates {
        if let Ok(s) = std::fs::read_to_string(&p) {
            for line in s.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    if std::env::var(k).is_err() {
                        // SAFETY: called from `main` before the Tokio runtime exists.
                        unsafe { std::env::set_var(k.trim(), v.trim()) };
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_cloud_ignores_localhost_jarvis_bind() {
        assert_eq!(
            bind_addr_from(Some("127.0.0.1:7420"), Some("10000"), Some("cloud")),
            "0.0.0.0:10000"
        );
    }

    #[test]
    fn bind_uses_render_port_when_unset() {
        assert_eq!(
            bind_addr_from(None, Some("10000"), Some("cloud")),
            "0.0.0.0:10000"
        );
    }

    #[test]
    fn bind_cloud_defaults_all_interfaces() {
        assert_eq!(bind_addr_from(None, None, Some("cloud")), "0.0.0.0:7420");
    }

    #[test]
    fn bind_desktop_defaults_localhost() {
        assert_eq!(bind_addr_from(None, None, None), DEFAULT_BIND);
    }
}
