# Implementation Phases

Each phase has a hard exit criterion. Do not start phase N+1 until phase N's
box is checked. Phases 1–4 are the critical path; 5–9 are product surface.

---

## Phase 1 — ACP transport skeleton

**Goal:** spawn an agent, complete `initialize`, hold the connection open.

Create `apps/src-tauri/src/acp/`:

- `mod.rs` — public surface + Tauri commands
- `transport.rs` — child process + line-delimited JSON-RPC read/write loop
- `protocol.rs` — serde types for ACP v1 (mirror `schema.json`)
- `client.rs` — connection state machine
- `agents.rs` — launch profiles

Add the ACP Rust crate rather than hand-rolling types:

```toml
# apps/src-tauri/Cargo.toml
agent-client-protocol = "2"   # crates.io, currently 2.1.0
```

> **Crate version vs protocol version.** The crate is at `2.x`; the *protocol*
> is `protocolVersion: 1`. These are unrelated numbers. Do not send `2`.

**Exit:** a `cargo test` integration test spawns `opencode acp`, sends
`initialize`, asserts `protocolVersion == 1`, and shuts down cleanly with no
orphaned process.

---

## Phase 2 — Session lifecycle

Implement `session/new` with `cwd` (absolute, = active pane directory) and an
`mcpServers` array containing Xplorer's own MCP server:

```json
{
  "name": "xplorer",
  "command": "C:/abs/path/to/xplorer.exe",
  "args": ["--mcp-server"],
  "env": []
}
```

Gate the optional lifecycle methods on advertised capabilities:

| Method | Gate |
|---|---|
| `session/load` | `agentCapabilities.loadSession === true` |
| `session/resume` | `sessionCapabilities.resume` present |
| `session/close` | `sessionCapabilities.close` present |
| `session/delete` | `sessionCapabilities.delete` present |
| `additionalDirectories` | `sessionCapabilities.additionalDirectories` present |

Use `additionalDirectories` for the **inactive pane's** directory when
supported — that is exactly the "dual-pane awareness" primitive. `cwd` stays
the active pane. The effective root set becomes `[cwd, ...additionalDirectories]`
and SHOULD bound the agent's filesystem operations.

**Exit:** session created against both Claude and opencode; `sessionId`
persisted; capability gating unit-tested with a fake agent that advertises
nothing (client must not call the optional methods).

---

## Phase 3 — Client-side methods (the security boundary)

This is where the build is won or lost. Implement in `acp/handlers.rs`:

### `fs/read_text_file`
Params: `sessionId`, `path` (absolute), optional `line` (1-based), `limit`.
- Guard path via `validate_agent_path_with_permissions`.
- Reuse `operations::read_text_file`.
- Honor `line`/`limit` — **1-based, not 0-based**.

### `fs/write_text_file`
Params: `sessionId`, `path` (absolute), `content`.
- Guard path.
- **Snapshot first**: call `file_versions::create_version` before overwrite.
- Log to `audit_log`.

### `session/request_permission`
Params: `sessionId`, `toolCall` (a `ToolCallUpdate`), `options` (`PermissionOption[]`).
- Option kinds: `allow_once`, `allow_always`, `reject_once`, `reject_always`.
- Respond `{"outcome":{"outcome":"selected","optionId":"..."}}`.
- **On cancellation you MUST respond `{"outcome":{"outcome":"cancelled"}}`** —
  not an error, not a timeout. Spec-mandated.
- `allow_always` / `reject_always` must persist into agent permissions
  (`update_agent_permissions`), scoped to session + tool name.

### `terminal/*` (5 methods)
`terminal/create` → `terminal/output` → `terminal/wait_for_exit` →
`terminal/kill` → `terminal/release`. Back with `pty.rs`.
- `terminal/create` returns `terminalId` **immediately**, does not block.
- Guard `command` with `is_blocked_command` + `is_network_command`.
- Guard `cwd` with the path validator.
- Honor `outputByteLimit`: truncate **from the beginning**, and **at a char
  boundary**. `agent::truncate_to_char_boundary` already exists — use it.
- Track terminal handles per session; force-release on session close.

**Exit:** a red-team test suite passes — see `docs/05-TESTING.md`. Traversal,
blocked commands, and out-of-root writes are all rejected, and the rejections
are asserted, not assumed.

---

## Phase 4 — session/update event sink

`session/update` is a **notification** (no response). Route each variant to a
Tauri event and a typed TS handler.

| `sessionUpdate` | UI target |
|---|---|
| `user_message_chunk` | echo in transcript |
| `agent_message_chunk` | streaming assistant text |
| `agent_thought_chunk` | collapsible "thinking" block |
| `tool_call` | new row in the operation queue |
| `tool_call_update` | patch that row by `toolCallId` |
| `plan` | the batch plan panel |
| `available_commands_update` | slash-command menu |
| `current_mode_update` | mode chip (Discuss/Draft/Apply) |
| `config_option_update` | settings panel |
| `session_info_update` | title / metadata |
| `usage_update` | tokens + cost footer |

**Exit:** a full prompt turn from a real agent streams end-to-end into the UI
and terminates with a `stopReason`.

---

## Phase 5 — Selection → prompt context

`apps/client/src/hooks/use-copilot-context.ts`:

- Read selection from the existing `use-cross-tab-selection` / pane state.
- Build `ContentBlock[]`:
  - one `text` block with the user instruction;
  - one block **per selected file**:
    - if `promptCapabilities.embeddedContext` → `ContentBlock::Resource`
      with `uri` (absolute, `file://`), `mimeType`, and inline `text`;
    - else → `ContentBlock::ResourceLink` (always supported baseline).
- Populate text for documents via `operations::extract_document_text`
  (PDF/DOCX/XLSX/PPTX). Skip binaries — send metadata only.
- Enforce a byte budget. Truncate at char boundaries and mark truncation.

Keep the file-chip UI idea from the existing `claude-code` extension (removable
chips above the input showing selected filenames). Discard its prompt
string-concatenation.

**Exit:** multi-selecting 5 mixed files and asking "what are these?" yields a
correct answer with no path strings glued into the prompt text.

---

## Phase 6 — Approval UI

- **Per-call**: inline approve/deny on the `tool_call` row, sourced from the
  `session/request_permission` options. Map `kind` → icon.
- **Batch**: render `plan` entries as an ordered checklist in the inactive
  pane; each entry has content, priority, status.
- **Diffs**: `ToolCallContent` of `type: "diff"` carries `path`, `oldText`
  (null for new files), `newText`. Render as a real diff view.
- **Terminals**: `ToolCallContent` of `type: "terminal"` carries `terminalId`;
  show live output, and **keep showing it after release**.
- **Follow-along**: `tool_call.locations[]` gives `{path, line}` — auto-reveal
  that file in the inactive pane as the agent works.
- **Stop button** → `session/cancel` notification.

**Exit:** you can watch a 10-file reorganization proposed, inspect it, approve
selectively, and see per-file status transitions.

---

## Phase 7 — Modes

Map `session/set_mode` to three product modes. Only expose modes the agent
actually advertises in `availableModes`; agents may also change mode themselves
and notify via `current_mode_update`.

| Product mode | Behavior |
|---|---|
| **Discuss** | read-only. Client auto-rejects any permission request for `edit`/`delete`/`move` tool kinds. |
| **Draft** | agent may propose plans; all mutations require explicit approval. |
| **Apply** | `allow_always` permitted for non-destructive kinds; destructive still prompts. |

**Exit:** Discuss mode provably cannot mutate the filesystem — asserted by test.

---

## Phase 8 — Extend the MCP tool surface

Grow `mcp_host.rs` from 6 to the full verb set in `docs/03-TOOL-CATALOG.md`.
Every new tool: guard args → call the existing `operations::*` command → log to
audit → return structured JSON.

**Exit:** `tools/list` returns the full set; the agent successfully organizes a
scratch directory using `preview_organization` then `execute_organization`.

---

## Phase 9 — Reversibility & audit surface

- Bind Ctrl+Z to `undo_operation`; expose `get_undo_history` in the panel.
- Enable `file_versions` on any directory the agent has touched.
- Surface `get_audit_log` as a "Copilot activity" panel with export.
- Add a **kill switch**: one control that revokes agent permissions, kills all
  PTYs (`pty_kill_all`), and closes the ACP session.

**Exit:** every mutation performed in phases 1–8 is visible in the audit log
and individually reversible.

---

## Phase 10 — Backend switcher

Expose Claude vs opencode as a dropdown. Because both are ACP, this is a
launch-profile swap (`scaffold/apps/src-tauri/src/acp/agents.rs`). Show
`usage_update` tokens/cost in the footer.

**Exit:** switching backends mid-app starts a fresh session against the other
agent with identical UI behavior.
