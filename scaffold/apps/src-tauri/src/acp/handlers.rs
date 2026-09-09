//! Client-side ACP methods. **This file is the security boundary.**
//!
//! Read docs/copilot/04-GOTCHAS.md section S in full before editing.
//!
//! Implemented here:
//!   fs/read_text_file, fs/write_text_file, session/request_permission,
//!   terminal/create, terminal/output, terminal/wait_for_exit,
//!   terminal/kill, terminal/release

use std::path::{Path, PathBuf};

use crate::agent::security::{
    is_blocked_command, is_network_command, normalize_path,
    validate_agent_path_with_permissions,
};

/// The single funnel every agent-supplied path must pass through.
///
/// GOTCHA S-2: `canonicalize` FAILS on paths that don't exist yet, and a move
/// destination usually doesn't exist yet. So: normalize lexically first (works
/// on non-existent paths), THEN canonicalize whatever ancestor prefix does
/// exist and re-validate that — otherwise a junction inside an allowed root
/// can point at C:\Windows and lexical validation won't notice (GOTCHA S-3).
///
/// GOTCHA S-4: normalize separators. Compare case-insensitively for allow-list
/// checks but preserve the original casing for display and for the wire.
pub fn guard_path(raw: &str, roots: &[PathBuf]) -> Result<PathBuf, String> {
    if !Path::new(raw).is_absolute() {
        return Err("ACP requires absolute paths".into());
    }
    let _lexical = normalize_path(raw);
    // TODO(claude) Phase 3:
    //   1. reject UNC (\\server\share), \\?\ and //./ device prefixes
    //   2. validate_agent_path_with_permissions(&lexical, roots)
    //   3. walk up to the nearest existing ancestor, canonicalize it,
    //      re-validate the canonical form against roots
    //   4. reject Windows reserved basenames: CON PRN AUX NUL COM1-9 LPT1-9
    todo!("guard_path")
}

/// GOTCHA S-1: `remove_file` / `remove_dir` must be unreachable from any
/// agent-facing surface. All deletion routes to operations::move_to_trash.
/// An LLM told to "clean up" will try `rm -rf`. Assume it.
pub fn guard_command(cmd: &str, args: &[String], sandbox_network: bool) -> Result<(), String> {
    if is_blocked_command(cmd, args) {
        return Err(format!("command '{cmd}' is blocked"));
    }
    if sandbox_network && is_network_command(cmd, args) {
        return Err(format!("command '{cmd}' blocked by network sandbox"));
    }
    Ok(())
}

// ─── fs/read_text_file ──────────────────────────────────────────────────────

pub async fn fs_read_text_file(
    _session_id: &str,
    _path: &str,
    _line: Option<u32>,   // GOTCHA A-11: 1-BASED
    _limit: Option<u32>,
) -> Result<String, String> {
    // TODO(claude) Phase 3: guard_path, then operations::read_text_file.
    // Apply `line`/`limit` as a 1-based window. Cap total bytes returned.
    todo!()
}

// ─── fs/write_text_file ─────────────────────────────────────────────────────

pub async fn fs_write_text_file(
    _session_id: &str,
    _path: &str,
    _content: &str,
) -> Result<(), String> {
    // TODO(claude) Phase 3:
    //   1. guard_path
    //   2. file_versions::create_version BEFORE overwriting (reversibility)
    //   3. write
    //   4. audit_log entry
    todo!()
}

// ─── session/request_permission ─────────────────────────────────────────────

/// GOTCHA S-6: this is a BLOCKING JSON-RPC request. The agent holds its turn
/// open until you answer. If the panel is closed or the user walks away, the
/// agent hangs forever. Therefore:
///   - a timeout is MANDATORY, and it must resolve to a REJECTION
///   - if the turn is cancelled while this is outstanding, respond with
///     `{"outcome":{"outcome":"cancelled"}}` — cancelling the turn does NOT
///     absolve you of answering the request
pub async fn session_request_permission(
    _session_id: &str,
    _request_id: u64,
    _tool_call: &serde_json::Value,
    _options: &[serde_json::Value],
) -> Result<serde_json::Value, String> {
    // TODO(claude) Phase 3:
    //   - Discuss mode: auto-reject if tool kind is edit/delete/move (Phase 7)
    //   - check persisted (session, tool, path-root) grants first
    //   - otherwise emit to the UI and await the oneshot with a timeout
    todo!()
}

// ─── terminal/* ─────────────────────────────────────────────────────────────

pub async fn terminal_create(
    _session_id: &str,
    _command: &str,
    _args: &[String],
    _env: &[(String, String)],
    _cwd: Option<&str>,
    _output_byte_limit: Option<usize>,
) -> Result<String, String> {
    // TODO(claude) Phase 3:
    //   - guard_command(command, args, sandbox_network)
    //   - guard_path(cwd) if present
    //   - spawn via pty.rs
    //   - return terminalId IMMEDIATELY; do not block on completion
    //   - register in AcpSession.terminals for forced cleanup
    todo!()
}

/// GOTCHA A-5: `outputByteLimit` semantics — when exceeded, the CLIENT
/// truncates from the BEGINNING of the output, and MUST cut at a character
/// boundary even if that retains slightly less than the limit. Naive byte
/// slicing on UTF-8 yields invalid strings and breaks JSON serialization.
///
/// `agent::truncate_to_char_boundary` exists but truncates the TAIL. Write the
/// front-truncating variant below and unit-test it with a 4-byte emoji
/// straddling the cut point.
pub fn truncate_front_to_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut idx = s.len() - max_bytes;
    while !s.is_char_boundary(idx) {
        idx += 1;
    }
    &s[idx..]
}

pub async fn terminal_output(_session_id: &str, _terminal_id: &str)
    -> Result<serde_json::Value, String> {
    // TODO(claude) Phase 3: return {output, truncated, exitStatus?}.
    // exitStatus present ONLY if the command has exited.
    // Unknown/released terminalId → error, never panic.
    todo!()
}

pub async fn terminal_wait_for_exit(_session_id: &str, _terminal_id: &str)
    -> Result<serde_json::Value, String> {
    // TODO(claude) Phase 3: return {exitCode, signal}; either may be null.
    todo!()
}

pub async fn terminal_kill(_session_id: &str, _terminal_id: &str) -> Result<(), String> {
    // TODO(claude) Phase 3: kill but DO NOT release. The terminal stays valid
    // for terminal/output and terminal/wait_for_exit, and the agent must still
    // call terminal/release.
    todo!()
}

pub async fn terminal_release(_session_id: &str, _terminal_id: &str) -> Result<(), String> {
    // TODO(claude) Phase 3: kill if running, free resources, invalidate the id.
    // If the terminal was embedded in a tool call, the UI SHOULD keep showing
    // its output after release (spec requirement).
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_truncation_respects_char_boundaries() {
        let s = "aaa🎉bbb"; // 🎉 is 4 bytes
        for limit in 1..s.len() {
            let out = truncate_front_to_char_boundary(s, limit);
            assert!(out.len() <= limit);
            assert!(std::str::from_utf8(out.as_bytes()).is_ok());
        }
    }

    // TODO(claude) Phase 3: port the full red-team suite from
    // docs/copilot/05-TESTING.md. Each case must ASSERT a rejection.
    #[test]
    #[ignore = "TODO(claude) Phase 3"]
    fn rejects_traversal_and_junctions() { todo!() }
}
