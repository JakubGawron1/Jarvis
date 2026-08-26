use crate::{fanout_kind, AppState};
use futures_util::{SinkExt, StreamExt};
use jarvis_protocol::{ClientMessage, ServerMessage};
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

const RENDER_WS: &str = "wss://jarvis-core-n12s.onrender.com/ws";
const CLOUD_PAIRING: &str = "uMrUM1mJIQFOmGPwMVekLpsjBTwV9QcO1lsX/im7l5I=";

pub fn spawn(state: Arc<AppState>) {
    if std::env::var("JARVIS_KIND").ok().as_deref() == Some("cloud") {
        return;
    }
    let peers = peer_urls();
    if peers.is_empty() {
        tracing::info!("mesh peers disabled");
        return;
    }
    for url in peers {
        let state = state.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = run_peer(state.clone(), &url).await {
                    tracing::warn!("mesh peer {url}: {e}");
                }
                tokio::time::sleep(Duration::from_secs(8)).await;
            }
        });
    }
}

fn peer_urls() -> Vec<String> {
    let raw = std::env::var("JARVIS_MESH_PEERS").unwrap_or_default();
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case("off") || raw == "none" || raw == "-" {
        return vec![];
    }
    if raw.is_empty() {
        return vec![RENDER_WS.into()];
    }
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn pairing_token() -> String {
    std::env::var("JARVIS_CLOUD_PAIRING_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("JARVIS_PAIRING_TOKEN").ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| CLOUD_PAIRING.into())
}

fn with_token(mut v: serde_json::Value) -> String {
    let token = pairing_token();
    if !token.is_empty() {
        v.as_object_mut()
            .map(|o| o.insert("token".into(), serde_json::Value::String(token)));
    }
    v.to_string()
}

fn hash_frame(s: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

pub async fn remember(recent: &Mutex<VecDeque<u64>>, frame: &str) -> bool {
    let h = hash_frame(frame);
    let mut q = recent.lock().await;
    if q.contains(&h) {
        return false;
    }
    q.push_back(h);
    while q.len() > 48 {
        q.pop_front();
    }
    true
}

async fn run_peer(state: Arc<AppState>, url: &str) -> anyhow::Result<()> {
    tracing::info!("mesh peer connecting {url}");
    let (ws, _) = tokio_tungstenite::connect_async(url).await?;
    let (mut sink, mut stream) = ws.split();
    let mut rx = state.tx.subscribe();

    let hello = ClientMessage::Hello {
        device: state.agent.local_device.clone(),
    };
    sink.send(Message::Text(with_token(serde_json::from_str(&hello.to_json())?).into()))
        .await?;
    send_mesh_sync(&state, &mut sink).await?;

    let mut tick = tokio::time::interval(Duration::from_secs(6));
    loop {
        tokio::select! {
            frame = stream.next() => {
                match frame {
                    Some(Ok(Message::Text(t))) => {
                        ingest_from_peer(&state, &t).await;
                    }
                    Some(Ok(Message::Ping(p))) => {
                        let _ = sink.send(Message::Pong(p)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => anyhow::bail!("peer closed"),
                    Some(Err(e)) => anyhow::bail!(e),
                    _ => {}
                }
            }
            Ok(local) = rx.recv() => {
                if !remember(&state.recent, &local).await {
                    continue;
                }
                if should_relay(&local) {
                    let relay = ClientMessage::Relay { frame: local };
                    let body = with_token(serde_json::from_str(&relay.to_json())?);
                    sink.send(Message::Text(body.into())).await?;
                }
            }
            _ = tick.tick() => {
                send_mesh_sync(&state, &mut sink).await?;
            }
        }
    }
}

async fn send_mesh_sync(
    state: &AppState,
    sink: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        Message,
    >,
) -> anyhow::Result<()> {
    let (io, leader, devices) = state.agent.owned_mesh().await;
    let msg = ClientMessage::MeshSync {
        core_id: state.agent.local_device.id.clone(),
        devices,
        io_device: io,
        leader,
    };
    let body = with_token(serde_json::from_str(&msg.to_json())?);
    sink.send(Message::Text(body.into())).await?;
    Ok(())
}

fn should_relay(json: &str) -> bool {
    json.contains("\"type\":\"visual\"")
        || json.contains("\"type\":\"handoff_ready\"")
        || json.contains("\"type\":\"presence\"")
        || json.contains("\"type\":\"device_hello\"")
        || json.contains("\"type\":\"reply\"")
        || json.contains("\"type\":\"speech\"")
}

async fn ingest_from_peer(state: &AppState, json: &str) {
    if !remember(&state.recent, json).await {
        return;
    }
    let Ok(msg) = serde_json::from_str::<ServerMessage>(json) else {
        return;
    };
    match msg {
        ServerMessage::Presence { devices, .. } | ServerMessage::MeshSync { devices, .. } => {
            let ours = state.agent.local_device.id.clone();
            let remote: Vec<_> = devices
                .into_iter()
                .filter(|d| d.id != ours && d.core_id.as_deref() != Some(ours.as_str()))
                .collect();
            if remote.is_empty() {
                return;
            }
            let core_id = remote
                .iter()
                .find_map(|d| d.core_id.clone())
                .unwrap_or_else(|| "cloud".into());
            let replies = state
                .agent
                .handle(ClientMessage::MeshSync {
                    core_id,
                    devices: remote,
                    io_device: String::new(),
                    leader: String::new(),
                })
                .await;
            for r in replies {
                if fanout_kind(&r) {
                    let _ = state.tx.send(r.to_json());
                }
            }
        }
        ServerMessage::PeerForward { device_id, json } => {
            let routes = state.routes.lock().await;
            if let Some(tx) = routes.get(&device_id) {
                let _ = tx.send(json);
            }
        }
        ServerMessage::Visual { .. }
        | ServerMessage::HandoffReady { .. }
        | ServerMessage::Reply { .. }
        | ServerMessage::Speech { .. }
        | ServerMessage::DeviceHello { .. } => {
            let _ = state.tx.send(json.to_string());
        }
        _ => {}
    }
}
