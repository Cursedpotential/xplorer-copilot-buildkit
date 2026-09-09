//! ACP (Agent Client Protocol) client for Xplorer.
//!
//! Xplorer is the ACP **Client**. An external process (claude-agent-acp or
//! `opencode acp`) is the ACP **Agent**. Transport is line-delimited
//! JSON-RPC 2.0 over the child's stdio.
//!
//! Register the Tauri commands at the bottom of this file in
//! `apps/src-tauri/src/main.rs`'s `generate_handler!` block.
//!
//! GOTCHAS THAT APPLY TO THIS MODULE:
//!   A-7  every Mutex guard must use .lock().unwrap_or_else(|e| e.into_inner())
//!   A-8  child process must die when the app dies (Windows Job Object)
//!   A-12 protocol version is 1; the `agent-client-protocol` crate is 2.x
//!   B-13 keep every file under 1000 lines

pub mod agents;
pub mod client;
pub mod events;
pub mod handlers;
pub mod protocol;
pub mod transport;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub use agents::{AgentProfile, AgentKind};
pub use client::AcpConnection;

/// Negotiated agent capabilities. Persist verbatim — every optional method
/// call must be gated on these.
///
/// GOTCHA A-1: anything omitted in `initialize` is UNSUPPORTED. Never call an
/// unadvertised method.
/// GOTCHA A-1: `load_session` is TOP-LEVEL in agentCapabilities, NOT inside
/// `session_capabilities`. The spec flags this inconsistency itself.
#[derive(Debug, Clone, Default)]
pub struct NegotiatedCapabilities {
    pub load_session: bool,
    pub prompt_image: bool,
    pub prompt_audio: bool,
    pub prompt_embedded_context: bool,
    pub mcp_http: bool,
    pub mcp_sse: bool,
    pub session_resume: bool,
    pub session_close: bool,
    pub session_delete: bool,
    pub session_list: bool,
    pub session_additional_directories: bool,
    pub auth_logout: bool,
}

impl NegotiatedCapabilities {
    // TODO(claude): parse from the `initialize` response. Treat every absent
    // field as false. Do NOT default anything to true.
    pub fn from_initialize_response(_v: &serde_json::Value) -> Self {
        todo!("Phase 1")
    }
}

/// Per-session state held by the client.
pub struct AcpSession {
    pub session_id: String,
    /// Active pane directory. Absolute. Session base for relative paths.
    pub cwd: std::path::PathBuf,
    /// Inactive pane directory, only populated if
    /// `session_additional_directories` was advertised.
    ///
    /// GOTCHA A-4: must be re-sent IN FULL on every session/load and
    /// session/resume. Omitting it activates NO roots — it does not restore.
    pub additional_directories: Vec<std::path::PathBuf>,
    /// GOTCHA A-6: terminals leak if the agent never calls terminal/release.
    /// Track them all and force-release on session close or agent death.
    pub terminals: HashMap<String, TerminalHandle>,
    /// Outstanding permission requests, keyed by JSON-RPC request id.
    ///
    /// GOTCHA S-6: these are BLOCKING calls. The agent is stalled until
    /// answered. GOTCHA S-7: on cancel these must be answered with the
    /// `cancelled` outcome, not dropped.
    pub pending_permissions: HashMap<u64, PendingPermission>,
    /// GOTCHA A-2: set while replaying session/load history so the UI replaces
    /// rather than appends.
    pub replaying: bool,
    pub mode: SessionMode,
}

pub struct TerminalHandle {
    pub terminal_id: String,
    pub pty_id: String,
    /// GOTCHA A-5: when exceeded, truncate from the FRONT, at a char boundary.
    pub output_byte_limit: Option<usize>,
}

pub struct PendingPermission {
    pub request_id: u64,
    pub tool_call_id: String,
    pub tool_kind: ToolKind,
    pub responder: tokio::sync::oneshot::Sender<PermissionOutcome>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// GOTCHA B-2: fixed enum. Do not invent kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Read, Edit, Delete, Move, Search, Execute, Think, Fetch, Other,
}

impl ToolKind {
    /// Destructive kinds are auto-rejected in Discuss mode and always prompt
    /// in Apply mode.
    pub fn is_destructive(self) -> bool {
        matches!(self, ToolKind::Edit | ToolKind::Delete | ToolKind::Move)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode { Discuss, Draft, Apply }

#[derive(Debug, Clone)]
pub enum PermissionOutcome {
    Selected { option_id: String },
    /// GOTCHA S-6: MUST be sent for an outstanding request when the turn is
    /// cancelled. This is not an error response.
    Cancelled,
}

/// Global connection registry.
///
/// GOTCHA A-7: guard with .lock().unwrap_or_else(|e| e.into_inner()).
pub static CONNECTIONS: std::sync::LazyLock<Mutex<HashMap<String, Arc<AcpConnection>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

// ─── Tauri commands (register these in main.rs) ─────────────────────────────

#[tauri::command]
pub async fn acp_connect(_profile: String) -> Result<String, String> {
    // TODO(claude) Phase 1: spawn the agent, run `initialize`, store
    // NegotiatedCapabilities, return a connection id.
    // On unsupported protocolVersion: close the connection and return an error
    // the UI can display. Do not limp along.
    todo!()
}

#[tauri::command]
pub async fn acp_new_session(
    _connection_id: String,
    _cwd: String,
    _additional_directories: Vec<String>,
) -> Result<String, String> {
    // TODO(claude) Phase 2.
    // - cwd MUST be absolute.
    // - mcpServers[0].command = std::env::current_exe(), args ["--mcp-server"]
    //   (GOTCHA A-9: never hardcode this path)
    // - only send additional_directories if the capability was advertised
    todo!()
}

#[tauri::command]
pub async fn acp_prompt(
    _session_id: String,
    _blocks: Vec<serde_json::Value>,
) -> Result<String, String> {
    // TODO(claude) Phase 4: send session/prompt, return the stopReason.
    todo!()
}

#[tauri::command]
pub async fn acp_cancel(_session_id: String) -> Result<(), String> {
    // TODO(claude) Phase 4. session/cancel is a NOTIFICATION.
    // GOTCHA S-7: keep the update sink alive afterward, answer any outstanding
    // permission request with Cancelled, wait for the session/prompt response
    // carrying StopReason::Cancelled, THEN clean up.
    todo!()
}

#[tauri::command]
pub async fn acp_respond_permission(
    _session_id: String,
    _request_id: u64,
    _option_id: String,
) -> Result<(), String> {
    // TODO(claude) Phase 3.
    // GOTCHA S-5: if the chosen option kind is allow_always/reject_always,
    // persist scoped to (session, tool, path-root) and expire on session
    // close. NEVER scope by tool name alone.
    todo!()
}

#[tauri::command]
pub async fn acp_set_mode(_session_id: String, _mode: String) -> Result<(), String> {
    // TODO(claude) Phase 7. Only expose modes present in availableModes.
    todo!()
}

#[tauri::command]
pub async fn acp_kill_switch(_session_id: String) -> Result<(), String> {
    // TODO(claude) Phase 9: revoke permissions, pty_kill_all, release all
    // terminals, close the session. Must work mid-turn and with an
    // outstanding permission request.
    todo!()
}
