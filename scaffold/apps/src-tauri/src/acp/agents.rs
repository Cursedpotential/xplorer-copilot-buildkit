//! Agent launch profiles.
//!
//! Both backends are ACP agents, so switching is a subprocess-command swap.
//!
//! GOTCHA A-13: both ship their own filesystem and shell tools, and opencode
//! enables ALL tools with NO permission required by default. If you don't
//! disable them the agent bypasses every guard in handlers.rs and writes to
//! disk directly. Disabling the built-ins is the single most important safety
//! decision in this build.
//!
//! GOTCHA B-15: these are 0.x packages. Pin exact versions and re-verify the
//! handshake after any bump.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind { Claude, Opencode }

#[derive(Debug, Clone)]
pub struct AgentProfile {
    pub kind: AgentKind,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    /// Config file handing the agent its restricted tool set.
    pub config_path: Option<PathBuf>,
}

impl AgentProfile {
    /// `@agentclientprotocol/claude-agent-acp` (0.75.x at time of writing).
    /// The older Zed adapter is `@zed-industries/claude-code-acp` (0.16.x).
    pub fn claude() -> Self {
        Self {
            kind: AgentKind::Claude,
            command: "claude-agent-acp".into(),
            // TODO(claude) Phase 10: constrain the tool set through the
            // adapter's allowed-tools/settings mechanism so the agent cannot
            // read or write the filesystem except via Xplorer's MCP verbs.
            args: vec![],
            env: vec![],
            config_path: None,
        }
    }

    /// `opencode acp`. Restricted via scaffold/config/opencode.json.
    pub fn opencode() -> Self {
        Self {
            kind: AgentKind::Opencode,
            command: "opencode".into(),
            args: vec!["acp".into()],
            env: vec![],
            // TODO(claude) Phase 10: point at the shipped opencode.json which
            // denies read/write/edit/multiedit/bash/patch.
            config_path: None,
        }
    }

    /// The MCP server entry handed to the agent at session/new.
    ///
    /// GOTCHA A-9: must be the ABSOLUTE path to this very executable. Use
    /// current_exe() — under `pnpm dev` it resolves into target/debug, which
    /// differs from production. Never hardcode either.
    ///
    /// GOTCHA A-10: stdio is mandatory for all agents; HTTP only if
    /// mcpCapabilities.http was advertised; SSE is deprecated — don't use it.
    pub fn xplorer_mcp_server() -> Result<serde_json::Value, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "name": "xplorer",
            "command": exe.to_string_lossy(),
            "args": ["--mcp-server"],
            "env": []
        }))
    }
}

/// GOTCHA A-8: on Windows a child usually SURVIVES parent exit — no
/// process-group kill by default. You will leak a node.exe per run.
/// Attach the child to a Job Object, or kill explicitly in
/// WindowEvent::Destroyed. Also handle the reverse: agent dies mid-turn, so
/// detect EOF on stdout, surface "agent exited unexpectedly", answer any
/// outstanding permission requests, and never hang the UI.
pub fn spawn_supervised(_profile: &AgentProfile) -> Result<(), String> {
    todo!("Phase 1")
}
