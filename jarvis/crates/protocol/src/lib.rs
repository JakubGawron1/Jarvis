use serde::{Deserialize, Serialize};

pub const DEFAULT_BIND: &str = "127.0.0.1:7420";
pub const PROTOCOL_VERSION: &str = "0.1.0";
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Pl,
    En,
}

impl Lang {
    pub fn as_str(self) -> &'static str {
        match self {
            Lang::Pl => "pl",
            Lang::En => "en",
        }
    }
}

/// Heuristic: Polish diacritics or common Polish function words.
pub fn detect_lang(text: &str) -> Lang {
    let lower = text.to_lowercase();
    let polish_chars = lower.chars().any(|c| "ąćęłńóśźż".contains(c));
    let polish_words = [
        " nie ", " czy ", " jest ", " proszę ", " otwórz ", " zrób ", " jaki ",
        " kalendarz", " notatk", " przełącz", " dziękuję", " może ", " będę ",
    ];
    let padded = format!(" {lower} ");
    if polish_chars || polish_words.iter().any(|w| padded.contains(w)) {
        Lang::Pl
    } else {
        Lang::En
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Windows,
    LinuxDesktop,
    Android,
    FlutterLinux,
    Cloud,
    JarvisLinux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DistroBoot {
    Qemu,
    Vbox,
    Metal,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceCaps {
    pub llm_local: bool,
    pub llm_online: bool,
    pub tts: bool,
    pub stt: bool,
    pub tools_os: bool,
    pub rewrite_core: bool,
    pub pull_core: bool,
    pub mic: bool,
    pub speaker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
    pub boot: Option<DistroBoot>,
    pub caps: DeviceCaps,
    pub core_version: String,
    pub battery: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub transcript: Vec<ChatTurn>,
    pub lang: Lang,
    pub open_jobs: Vec<String>,
    pub io_device: String,
    pub leader: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTurn {
    pub role: String,
    pub content: String,
    pub lang: Lang,
}

/// Generic HUD visual: 3D hologram, slides, diagram, or procedural video.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualSpec {
    pub kind: VisualKind,
    pub title: String,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub scene3d: Option<Scene3d>,
    #[serde(default)]
    pub slides: Option<Vec<Slide>>,
    #[serde(default)]
    pub diagram: Option<Diagram>,
    #[serde(default)]
    pub video: Option<VideoClip>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualKind {
    Scene3d,
    Slides,
    Diagram,
    Video,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene3d {
    #[serde(default = "default_cam_z")]
    pub camera_z: f32,
    #[serde(default)]
    pub bodies: Vec<Body3d>,
    #[serde(default)]
    pub links: Vec<(usize, usize)>,
    #[serde(default)]
    pub particles: u32,
    #[serde(default)]
    pub neural: bool,
}

fn default_cam_z() -> f32 {
    8.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body3d {
    pub id: String,
    #[serde(default = "default_shape")]
    pub shape: String,
    #[serde(default = "default_radius")]
    pub radius: f32,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub glow: bool,
    #[serde(default)]
    pub orbit: Option<Orbit>,
    #[serde(default)]
    pub label: Option<String>,
}

fn default_shape() -> String {
    "sphere".into()
}
fn default_radius() -> f32 {
    0.25
}
fn default_color() -> String {
    "#ff8c3a".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orbit {
    #[serde(default)]
    pub radius: f32,
    #[serde(default = "default_speed")]
    pub speed: f32,
    #[serde(default)]
    pub tilt: f32,
}

fn default_speed() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slide {
    pub title: String,
    #[serde(default)]
    pub bullets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagram {
    pub nodes: Vec<String>,
    #[serde(default)]
    pub edges: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoClip {
    #[serde(default = "default_duration")]
    pub duration_sec: f32,
    #[serde(default)]
    pub caption: Option<String>,
}

fn default_duration() -> f32 {
    8.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Text {
        id: String,
        content: String,
        #[serde(default)]
        lang: Option<Lang>,
        /// Client that typed this — I/O follows the speaker.
        #[serde(default)]
        device_id: Option<String>,
    },
    Utterance {
        id: String,
        #[serde(default)]
        transcript: Option<String>,
        #[serde(default)]
        audio_b64: Option<String>,
        #[serde(default)]
        device_id: Option<String>,
    },
    Confirm {
        id: String,
        accepted: bool,
    },
    Hello {
        device: DeviceInfo,
    },
    HandoffRequest {
        target_device: String,
    },
    PullCore {},
    Ping {},
    DismissVisual {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Reply {
        id: String,
        content: String,
        lang: Lang,
    },
    Confirm {
        id: String,
        prompt: String,
        lang: Lang,
    },
    TaskProgress {
        job_id: String,
        status: String,
        detail: String,
    },
    DeviceHello {
        device: DeviceInfo,
    },
    DeviceLost {
        device_id: String,
    },
    Presence {
        io_device: String,
        leader: String,
        devices: Vec<DeviceInfo>,
    },
    HandoffReady {
        snapshot: SessionSnapshot,
    },
    JobDeferred {
        job_id: String,
        until: String,
        message: String,
    },
    CoreWaking {},
    CoreUpdate {
        version: String,
    },
    Stats {
        cpu: f32,
        ram_used: u64,
        ram_total: u64,
        model: String,
        core_version: String,
    },
    Error {
        message: String,
    },
    Pong {},
    Visual {
        id: String,
        spec: VisualSpec,
        lang: Lang,
    },
    /// WAV (or other) speech for the matching reply `id`. Text is always sent first.
    Speech {
        id: String,
        mime: String,
        audio_b64: String,
    },
}

impl ClientMessage {
    pub fn parse(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

impl ServerMessage {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"type":"error","message":"serialize failed"}"#.to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_polish() {
        assert_eq!(detect_lang("otwórz notatnik"), Lang::Pl);
        assert_eq!(detect_lang("jaki mam kalendarz"), Lang::Pl);
    }

    #[test]
    fn detects_english() {
        assert_eq!(detect_lang("open notepad please"), Lang::En);
    }

    #[test]
    fn roundtrip_text_message() {
        let m = ClientMessage::Text {
            id: "1".into(),
            content: "hi".into(),
            lang: Some(Lang::En),
            device_id: None,
        };
        let s = serde_json::to_string(&m).unwrap();
        let back = ClientMessage::parse(&s).unwrap();
        match back {
            ClientMessage::Text { content, .. } => assert_eq!(content, "hi"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn visual_message_roundtrip() {
        let m = ServerMessage::Visual {
            id: "1".into(),
            spec: VisualSpec {
                kind: VisualKind::Scene3d,
                title: "Atom".into(),
                subtitle: None,
                scene3d: Some(Scene3d {
                    camera_z: 7.0,
                    bodies: vec![],
                    links: vec![],
                    particles: 10,
                    neural: false,
                }),
                slides: None,
                diagram: None,
                video: None,
            },
            lang: Lang::Pl,
        };
        let s = m.to_json();
        assert!(s.contains("\"type\":\"visual\""));
        assert!(s.contains("scene3d"));
    }

    #[test]
    fn speech_message_roundtrip() {
        let m = ServerMessage::Speech {
            id: "1".into(),
            mime: "audio/wav".into(),
            audio_b64: "AAAA".into(),
        };
        let s = m.to_json();
        assert!(s.contains("\"type\":\"speech\""));
        let back: ServerMessage = serde_json::from_str(&s).unwrap();
        match back {
            ServerMessage::Speech { audio_b64, .. } => assert_eq!(audio_b64, "AAAA"),
            _ => panic!("wrong variant"),
        }
    }
}
