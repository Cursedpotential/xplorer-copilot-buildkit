//! `session/update` notification sink.
//!
//! GOTCHA: session/update is a NOTIFICATION. Never send a response.
//! GOTCHA S-7: keep accepting `tool_call` updates AFTER sending
//!             session/cancel — the agent may emit final updates before it
//!             responds with StopReason::Cancelled.
//! GOTCHA B-4: `tool_call_update` is a SPARSE PATCH keyed by toolCallId.
//!             Merge; never replace, or you wipe title/kind and the row blanks.
//! GOTCHA A-2: during session/load replay, the agent streams the ENTIRE
//!             history before responding. Buffer/replace instead of appending
//!             or you duplicate the transcript. Dedupe on messageId.

/// Every variant maps to a Tauri event name consumed by
/// apps/client/src/hooks/use-copilot-session.ts.
pub const EVENT_PREFIX: &str = "acp://update/";

pub enum SessionUpdate {
    UserMessageChunk,
    AgentMessageChunk,
    AgentThoughtChunk,
    ToolCall,
    ToolCallUpdate,
    Plan,
    AvailableCommandsUpdate,
    CurrentModeUpdate,
    ConfigOptionUpdate,
    SessionInfoUpdate,
    /// GOTCHA B-14: carries tokens, context size, and cumulative cost in
    /// ISO 4217. Free spend tracking — surface it.
    UsageUpdate,
}

pub fn dispatch(_app: &tauri::AppHandle, _session_id: &str, _update: &serde_json::Value) {
    // TODO(claude) Phase 4:
    //   - match on the `sessionUpdate` discriminator
    //   - IGNORE unknown sessionId / unknown toolCallId without panicking
    //   - emit `acp://update/<variant>` with the raw payload
    todo!()
}
