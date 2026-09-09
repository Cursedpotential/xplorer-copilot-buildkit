# MCP tool surface

`mcp_host.rs` already exposes six tools (`read_file`, `write_file`,
`list_directory`, `search_files`, `run_command`, `get_git_status`) over
JSON-RPC when the binary is launched with `--mcp-server`. Phase 8 grows this to
the catalog in `docs/copilot/03-TOOL-CATALOG.md`.

## Hard rules

1. **Never register `remove_file` or `remove_dir`.** All deletion goes through
   `operations::move_to_trash`. Add a unit test asserting `tools/list` contains
   none of the names on the "Never expose" list. (Gotcha S-1)
2. **Every path arg goes through `acp::handlers::guard_path`.** (Gotchas S-2, S-3, S-4)
3. **`run_command` must route through `is_blocked_command` + `is_network_command`.**
   It currently does not. Harden it in Phase 8.
4. **Re-key path-scoped metadata inside move/rename transactions.** Tags,
   notes, and custom metadata are keyed by PATH, and this copilot's entire job
   is moving files. Every unintercepted move silently orphans user data.
   (Gotcha B-16 — this is the highest-value bug in the whole build.)
5. **`--mcp-server` must be truly headless.** No Tauri window, nothing on
   stdout but JSON-RPC. Any stray `println!`, stdout `tracing` writer, or
   startup banner corrupts the stream and the agent fails to parse. All logs to
   stderr or a file. (Gotcha A-14)
6. **Decide the SQLite concurrency story.** The GUI instance and the
   `--mcp-server` instance both want the storage layer → `database is locked`.
   WAL + busy_timeout, or proxy mutations back to the GUI instance. Document
   the choice in `docs/copilot/decisions.md`. (Gotcha A-15)

## Per-tool shape

```
validate args (guard_path / guard_command)
  → delegate to the existing operations::* command
  → write an audit_log entry
  → return structured JSON
```

Never panic. Return `McpToolResult::err` with a human-readable reason.

Mutating tools return `{"ok":bool,"undo_token":string,"audit_id":int}`.
Listing tools accept `limit`/`offset` and return
`{"items":[],"total":n,"truncated":bool}`.

## Version identifiers — do not mix them up (Gotcha A-12)

| Thing | Value |
|---|---|
| MCP protocol version in `mcp_server.rs` | `"2024-11-05"` (date string) |
| ACP protocol version on the wire | `1` (integer) |
| `agent-client-protocol` crate | `2.x` |
