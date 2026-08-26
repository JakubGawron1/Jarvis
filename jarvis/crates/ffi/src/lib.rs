//! C ABI for Flutter (`pull_core` swaps this .so).
use jarvis_core::AgentHandle;
use jarvis_protocol::ClientMessage;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static RT: OnceLock<Runtime> = OnceLock::new();
static AGENT: OnceLock<AgentHandle> = OnceLock::new();

fn rt() -> &'static Runtime {
    RT.get_or_init(|| Runtime::new().expect("tokio"))
}

fn agent() -> &'static AgentHandle {
    AGENT.get_or_init(|| {
        let root = std::env::var("JARVIS_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        AgentHandle::spawn(root).expect("agent")
    })
}

/// Submit a JSON ClientMessage; returns JSON array of ServerMessage.
#[unsafe(no_mangle)]
pub extern "C" fn jarvis_submit(json: *const c_char) -> *mut c_char {
    let raw = unsafe { CStr::from_ptr(json) }.to_string_lossy().into_owned();
    let out = rt().block_on(async {
        match ClientMessage::parse(&raw) {
            Ok(msg) => {
                let replies = agent().handle(msg).await;
                serde_json::to_string(&replies).unwrap_or_else(|_| "[]".into())
            }
            Err(e) => format!(r#"[{{"type":"error","message":"{e}"}}]"#),
        }
    });
    CString::new(out).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn jarvis_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn jarvis_version() -> *mut c_char {
    CString::new(jarvis_protocol::CORE_VERSION)
        .unwrap()
        .into_raw()
}
