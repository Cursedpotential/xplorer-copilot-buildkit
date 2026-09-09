# Task prompts for Claude Code

One block per phase. Paste verbatim. Each assumes `CLAUDE.copilot.md` is
imported from the repo's `CLAUDE.md` and `docs/copilot/` is present.

---

## Phase 0

```
Read docs/copilot/00-PREFLIGHT.md and execute it.

Do not write any application code in this phase. Your job is verification.

1. Confirm the toolchain, then run: pnpm install, npx tsc --noEmit,
   pnpm run lint, npx vitest run, cargo test, cargo build.
2. Launch ./target/debug/xplorer --mcp-server and drive it by hand with the
   two JSON-RPC lines in the doc. Record the actual response bodies.
3. Launch claude-agent-acp and opencode acp separately. Send the initialize
   payload to each. Record each agent's full agentCapabilities object verbatim
   into a new file docs/copilot/agent-capabilities.md.
4. Fill in every checkbox in 00-PREFLIGHT.md with PASS/FAIL plus evidence.

If any exit criterion fails, STOP and report. Do not attempt fixes beyond
toolchain installation.
```

---

## Phase 1

```
Implement Phase 1 from docs/copilot/01-PHASES.md: the ACP transport skeleton.

Read docs/copilot/02-ACP-REFERENCE.md and docs/copilot/04-GOTCHAS.md first.
Gotchas S-8, A-7, A-8, A-12, B-13 apply directly to this phase.

Create apps/src-tauri/src/acp/ with mod.rs, transport.rs, protocol.rs,
client.rs, agents.rs. Add the agent-client-protocol crate (2.x) to
Cargo.toml — do not hand-roll the protocol types, and do not confuse the
crate version (2.x) with the protocol version (1).

Use the stubs in apps/src-tauri/src/acp/ as your starting point. Every
TODO(claude) marker must be resolved or explicitly deferred with a reason.

Requirements:
- Line-delimited JSON-RPC over the child's stdin/stdout. stderr → tracing.
- Every Mutex guard uses .lock().unwrap_or_else(|e| e.into_inner()).
- Child process is killed on app exit AND on EOF-detected agent death.
  On Windows use a Job Object or an explicit kill in WindowEvent::Destroyed.
- Store the negotiated capabilities in session state. If the agent returns an
  unsupported protocolVersion, close the connection and surface an error.

Also build the fake-agent test harness described in docs/copilot/05-TESTING.md.
It must support the "advertises zero optional capabilities" and "returns an
unsupported protocolVersion" scripts at minimum.

Exit criterion: a cargo test that spawns the fake agent, completes initialize,
asserts protocolVersion == 1, and shuts down with no orphaned process.

Verify with: cargo test, cargo clippy -- -D warnings.
```

---

## Phase 2

```
Implement Phase 2 from docs/copilot/01-PHASES.md: session lifecycle.

Gotchas A-1, A-2, A-3, A-4, A-9, A-10, A-14, A-15 apply.

- session/new with absolute cwd (active pane) and mcpServers pointing at
  std::env::current_exe() with args ["--mcp-server"]. Never hardcode the path.
- Use additionalDirectories for the inactive pane's directory, but ONLY if
  sessionCapabilities.additionalDirectories was advertised.
- Gate session/load, session/resume, session/close, session/delete strictly on
  advertised capabilities. Remember loadSession is TOP-LEVEL in
  agentCapabilities, not inside sessionCapabilities.
- session/load and session/resume must NOT share a handler. load replays
  history via session/update then responds; resume must not replay.
- On load/resume, re-send the complete additionalDirectories list. Omitting it
  activates no roots.
- Make --mcp-server mode truly headless: no Tauri window, nothing on stdout
  except JSON-RPC, all logging to stderr. Verify by piping stdout to a JSON
  parser and asserting every line parses.
- Decide and document the SQLite concurrency strategy for two instances of the
  binary (gotcha A-15). WAL + busy_timeout, or proxying. Write the decision
  into docs/copilot/decisions.md.

Test with the fake agent advertising (a) nothing optional, (b) everything.
Assert the client never calls an unadvertised method in case (a).
```

---

## Phase 3

```
Implement Phase 3 from docs/copilot/01-PHASES.md: the client-side methods.
This is the security boundary. Read ALL of docs/copilot/04-GOTCHAS.md
section S before starting.

Implement in apps/src-tauri/src/acp/handlers.rs:
  fs/read_text_file, fs/write_text_file, session/request_permission,
  terminal/create, terminal/output, terminal/wait_for_exit, terminal/kill,
  terminal/release.

Non-negotiable:
1. EVERY path from the agent goes through agent::security::normalize_path then
   validate_agent_path_with_permissions. Move destinations do not exist yet, so
   lexical normalization first, then canonicalize-and-revalidate whatever
   prefix does exist (defeats junctions/symlinks — gotcha S-3).
2. EVERY command string goes through is_blocked_command and, when the sandbox
   is on, is_network_command.
3. fs/write_text_file calls file_versions::create_version BEFORE overwriting.
4. Every mutation writes an audit_log entry.
5. session/request_permission is a BLOCKING call. Implement a timeout that
   resolves to rejection. If the turn is cancelled while a request is
   outstanding, respond {"outcome":{"outcome":"cancelled"}} — this is mandatory
   and is not the same as erroring.
6. allow_always / reject_always persist scoped to (session, tool, path-root)
   and expire on session close. Never scope by tool name alone.
7. outputByteLimit truncates from the FRONT at a character boundary. Write a
   front-truncating variant of agent::truncate_to_char_boundary and unit-test
   it with 4-byte emoji straddling the cut.
8. Track every terminalId per session. Release all + pty_kill_all on session
   close or agent death.

Then write the full red-team suite from docs/copilot/05-TESTING.md. Every case
must assert a rejection with a surfaced reason. This suite passing IS the exit
criterion — do not declare the phase complete on the basis of the handlers
compiling.
```

---

## Phase 4

```
Implement Phase 4: the session/update event sink.

session/update is a NOTIFICATION — never send a response to it.

Route all eleven variants to Tauri events and typed TS handlers per the table
in docs/copilot/01-PHASES.md. Then add packages/sdk/src/services/copilot.ts and
surface it through apps/client/src/lib/tauri-api.ts. Do NOT call invoke() from
any component (gotcha B-8).

Reducer requirements:
- tool_call_update is a SPARSE PATCH keyed by toolCallId. Merge, never replace
  (gotcha B-4).
- Keep accepting tool_call updates after session/cancel is sent (gotcha S-7).
- Ignore updates for unknown sessionId/toolCallId without panicking.
- Enter a "replaying" state during session/load so the transcript is replaced
  rather than appended (gotcha A-2). Dedupe on messageId.

Add every new Tauri command to the invoke mock in
apps/client/src/__tests__/setup.ts and every new lucide-react icon to its mock
(gotcha B-9). console.warn/console.error only (B-12).

Exit criterion: a real prompt turn from claude-agent-acp streams end-to-end
into the UI and terminates with a stopReason.
```

---

## Phase 5

```
Implement Phase 5: selection → prompt context.

Create apps/client/src/hooks/use-copilot-context.ts. Read the existing
use-cross-tab-selection.ts, use-pane-sync.ts, and use-split-layout.ts and
build on them — do not invent a parallel selection model.

Build ContentBlock[]:
- one text block for the user instruction
- one block per selected file:
    if promptCapabilities.embeddedContext → ContentBlock::Resource with
      absolute file:///C:/... uri (THREE slashes), mimeType, and inline text
    else → ContentBlock::ResourceLink (baseline, always safe)
- populate text for PDF/DOCX/XLSX/PPTX via operations::extract_document_text
- binaries: metadata only, never bytes
- enforce a total byte budget; truncate at char boundaries; mark truncation
  visibly in the UI

Never concatenate paths into the prompt text string. The whole point is
structured context.

Build the file-chip UI (removable chips above the input). You may reference
packages/extensions/claude-code for the chip visuals ONLY — ignore its PTY
scraping and its prompt string building, which are the anti-patterns this
workstream replaces (gotcha S-8).

Add all new strings to en/zh/ja/id locale files (gotcha B-10).

Exit criterion: multi-select 5 mixed files, ask "what are these?", get a
correct answer, and confirm via the raw JSON-RPC log that no paths were glued
into prompt text.
```

---

## Phase 6

```
Implement Phase 6: the approval UI, in apps/client/src/components/copilot/.

- Per-call approve/deny inline on the tool_call row, options sourced from the
  live session/request_permission payload. Map PermissionOptionKind to icons.
- Batch plan panel rendering `plan` update entries (content, priority, status)
  as an ordered checklist in the inactive pane.
- Diff renderer for ToolCallContent type "diff". oldText is null for new files
  — render null as an empty side, do not crash (gotcha B-5).
- Terminal renderer for ToolCallContent type "terminal": live output, and KEEP
  DISPLAYING after terminal/release (spec requirement).
- Follow-along: use tool_call.locations[] {path, line} to auto-reveal in the
  inactive pane. line is 1-BASED (gotcha A-11).
- Distinguish "pending because input is streaming" from "pending because it's
  awaiting your approval" — same status value, different UI (gotcha B-3).
- Stop button sends the session/cancel notification.

Keep every file under 1000 lines (B-13). Tailwind + --xp-* CSS variables, and
match the existing theme classes.
```

---

## Phase 7

```
Implement Phase 7: modes.

Map product modes to session/set_mode, but ONLY expose modes the agent actually
advertises in availableModes. Handle agent-initiated mode changes arriving as
current_mode_update. session/set_mode is callable while idle or generating.

Discuss  → read-only. Client AUTO-REJECTS any permission request whose tool
           kind is edit, delete, or move.
Draft    → plans allowed; every mutation needs explicit approval.
Apply    → allow_always permitted for non-destructive kinds only; destructive
           kinds always prompt regardless.

Exit criterion: a test that puts the session in Discuss mode, has the fake
agent attempt an edit, a delete, and a move, and asserts the filesystem is
byte-identical afterward. Prove it, don't assume it.
```

---

## Phase 8

```
Implement Phase 8: extend the MCP tool surface per docs/copilot/03-TOOL-CATALOG.md.

Grow mcp_host.rs / src/mcp/ from 6 tools to the full catalog. For each tool:
validate args → delegate to the existing operations::* command → write an audit
entry → return structured JSON. Never panic; return McpToolResult::err.

Enforce the "Never expose" list in the catalog. Add a unit test asserting that
tools/list contains NONE of those names — remove_file and remove_dir especially
(gotcha S-1).

CRITICAL, do not skip: Xplorer's tags, notes, and custom metadata are keyed by
PATH, and this copilot's entire purpose is moving files. Every agent-initiated
move currently orphans that metadata (gotcha B-16). Intercept every move/rename
in the MCP layer and re-key the storage rows in the SAME transaction as the
move. Add the fixture/tagged/ test from docs/copilot/05-TESTING.md and assert
tags and notes survive an agent-driven reorganization.

Also harden the pre-existing run_command tool — it must route through
is_blocked_command and is_network_command.

Exit criterion: the agent organizes fixture/photos/ using preview_organization
then execute_organization, and the tagged-metadata test passes.
```

---

## Phase 9

```
Implement Phase 9: reversibility and audit surface.

- Ctrl+Z → operations::undo_redo::undo_operation; render get_undo_history in
  the copilot panel.
- Enable file_versions on any directory the agent has touched.
- "Copilot activity" panel over get_audit_log, with export.
- KILL SWITCH: one control that revokes agent permissions, calls pty_kill_all,
  releases all terminals, and closes the ACP session. It must work even if the
  agent is mid-turn and even if a permission request is outstanding — answer
  that outstanding request with the cancelled outcome (gotcha S-6, S-7).

Exit criterion: every mutation from phases 1-8 appears in the audit log and is
individually reversible. Verify by scripted replay, not by inspection.
```

---

## Phase 10

```
Implement Phase 10: the backend switcher.

Expose Claude vs opencode as a dropdown driven by the launch profiles in
apps/src-tauri/src/acp/agents.rs. Switching starts a fresh session against the
other agent; do not attempt to migrate session state across backends.

Wire scaffold/config/opencode.json into the opencode profile. Both profiles
MUST disable the agent's own built-in filesystem and shell tools (read, write,
edit, multiedit, bash, patch) so the agent can only act through Xplorer's
permissioned MCP verbs (gotcha A-13). Add a test asserting the agent has no
usable direct-write path.

Surface usage_update tokens and cost (ISO 4217) in the panel footer (B-14).

Pin exact agent versions in the lockfile and document them in
docs/copilot/agent-capabilities.md. These are 0.x packages; re-verify the
handshake after every bump (B-15).
```
