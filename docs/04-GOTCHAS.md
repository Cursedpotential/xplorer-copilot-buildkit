# Gotchas — read this before writing code

Ranked by how much damage they cause. **S = will destroy user data or silently
break security. A = will cost you a day. B = will cost you an hour.**

---

## S-1. `remove_file` / `remove_dir` must be unreachable from the agent

Xplorer registers both. They are permanent. The agent-facing surface must route
**all** deletion through `operations::move_to_trash`.

- Do not expose `remove_file`/`remove_dir` as MCP tools.
- Do not let `terminal/create` run `del`, `rm`, `Remove-Item`, `rmdir`,
  `format`, `diskpart` — extend the blocklist in `security.rs` and assert it.
- An LLM asked to "clean up" will absolutely try `rm -rf`. Assume it.

## S-2. Path traversal on *not-yet-existing* destinations

`std::fs::canonicalize` fails on paths that don't exist yet. A move
destination usually doesn't exist yet. If you canonicalize-or-bail, you'll
either break every move or silently skip validation.

`agent::security::normalize_path` exists precisely for this: it resolves `.`
and `..` lexically without requiring existence. Use it. Then run
`validate_agent_path_with_permissions`.

Test vectors that must all be rejected:
```
C:/Users/matt/Documents/../../Windows/System32/drivers/etc/hosts
\\?\C:\Windows\System32
//./C:/Windows
C:/Users/matt/Documents/link-to-system32/...   (junction/symlink)
\\server\share\...                             (UNC)
```

## S-3. Windows junctions and symlinks defeat lexical normalization

Lexical normalization does not follow reparse points. A junction inside an
allowed root can point at `C:\Windows`. After lexical validation, if the parent
exists, **canonicalize and re-validate**. Two-stage: lexical first (so
non-existent paths work), then real-path check on whatever prefix does exist.

## S-4. Windows path separators cross the JSON-RPC boundary

ACP mandates absolute paths. On Windows they contain `\`, which must be escaped
in JSON, and agents written on macOS/Linux frequently mishandle them.

- Normalize to forward slashes on the wire; the repo's own `CLAUDE.md` already
  says "use forward slashes in JS/TS, but Rust paths use `\\`".
- Drive letters: `C:/Users/...` is accepted by Rust `Path`. `file://` URIs need
  `file:///C:/Users/...` — **three** slashes.
- Never string-concatenate paths. `PathBuf::join` only.
- Case-insensitive but case-preserving: compare canonically lowercased for
  allow-list checks, but preserve the original for display and for the wire.

## S-5. `allow_always` is a persistent grant — scope it or regret it

If you persist `allow_always` per *tool name* only, "always allow move_file"
becomes a blanket license to move anything anywhere for the rest of the app's
life. Scope every remembered grant to **(session, tool, path-root)** at minimum,
and expire it on session close. Show remembered grants in the UI with revoke.

## S-6. The permission request is a *blocking* JSON-RPC call

`session/request_permission` is a request, not a notification. The agent is
stalled until you answer. Consequences:

- If your UI never answers (panel closed, window minimized, user walked away),
  the agent hangs forever holding a turn open.
- You need a timeout policy. Timeout must resolve to a **rejection**, not a
  dropped request.
- If the user hits Stop while a permission request is outstanding, you MUST
  answer that outstanding request with `{"outcome":{"outcome":"cancelled"}}`.
  Cancelling the turn does not absolve you of answering.

## S-7. Cancellation is not a hard stop

`session/cancel` is a notification. The spec says clients **SHOULD keep
accepting `tool_call` updates after cancelling**, because the agent may emit
final updates before it responds to `session/prompt` with
`StopReason::Cancelled`. If you tear down state on cancel, late updates will
arrive for a session you've forgotten and you'll either panic or leak.

Correct sequence: send `session/cancel` → keep the sink alive → answer any
outstanding permission request with `cancelled` → wait for the `session/prompt`
response carrying `Cancelled` → *then* clean up.

## S-8. Do not build the copilot as a sandboxed extension

The extension sandbox blocks `fetch`, `XMLHttpRequest`, `WebSocket`,
`localStorage`, `sessionStorage`, `indexedDB`, `eval`, and `__TAURI__` /
`__TAURI_INTERNALS__` / `__TAURI_IPC__`. `XplorerAPI` has **no subprocess
namespace** — there is no sanctioned way to spawn `opencode acp`.

The shipped `claude-code` extension bypasses this by `await import`ing
`@tauri-apps/api/core` and calling `invoke('pty_spawn')` directly, which means
the sandbox is not uniformly enforced. Do not build on an inconsistently
enforced boundary. Go native.

---

## A-1. Capability negotiation is mandatory, and omission means unsupported

The spec is explicit: **capabilities omitted in `initialize` MUST be treated as
UNSUPPORTED.** Not "probably fine", not "try it and see".

- Claude and opencode advertise **different** capability sets. Store what each
  returned.
- `promptCapabilities.embeddedContext` false → you cannot send
  `ContentBlock::Resource`. Fall back to `ResourceLink`.
- `promptCapabilities.image` false → don't send thumbnails as image blocks.
- Baseline you can always rely on: `ContentBlock::Text` and
  `ContentBlock::ResourceLink` in prompts; `session/new`, `session/prompt`,
  `session/cancel`, `session/update` on the agent side.
- `loadSession` lives at the **top level** of `agentCapabilities`, not inside
  `sessionCapabilities` — the spec flags this inconsistency explicitly. Getting
  it wrong means you'll read `undefined` and disable a working feature.

## A-2. `session/load` replays the whole conversation before responding

The agent MUST stream the entire history as `session/update` notifications and
only then respond to the `session/load` request. If your UI appends everything
it receives, restoring a session will duplicate the transcript. Enter a
"replaying" state, buffer or replace rather than append, and exit it on the
`session/load` response.

Also: replayed chunks may carry `messageId`. Treat it as opaque; dedupe on it.

## A-3. `session/resume` is the opposite and must not replay

If you implement both, don't share the handler. `resume` restores context and
returns without replay. Choosing `load` when you wanted `resume` produces a
several-thousand-line transcript flood on every reconnect.

## A-4. `additionalDirectories` must be re-sent in full every time

On `session/load` and `session/resume` you must send the **complete intended**
additional-root list again. Omitting it or sending `[]` does **not** restore
previously stored roots — it activates none. If your dual-pane copilot relies
on the inactive pane being a root, and you reconnect without re-sending, the
agent silently loses access and starts refusing operations for no visible reason.

Also: `cwd` in the request must match the session's original `cwd`.

## A-5. Terminal output truncation is from the *front*, at char boundaries

`outputByteLimit` semantics: when exceeded, the **Client** truncates from the
beginning of the output. And it MUST truncate at a character boundary even if
that means retaining slightly less than the limit. Naive byte slicing on UTF-8
produces invalid strings and will break JSON serialization.
`agent::truncate_to_char_boundary` already handles this — but note it truncates
the *tail*; you need a front-truncating variant. Write it and unit-test it with
multi-byte input.

## A-6. Terminals leak if you don't release them

The **Agent** is responsible for calling `terminal/release`, but a crashed or
buggy agent won't. You'll accumulate live PTYs. Track every `terminalId` per
session, and on session close / agent death call `pty_kill_all` and release
everything. Also: after `terminal/kill` the terminal is still valid — output
and exit status remain readable, and release is still required.

## A-7. Mutex poisoning is a live hazard in this codebase

The repo's `CLAUDE.md` calls it out by name. Every `Mutex` guard must be
`.lock().unwrap_or_else(|e| e.into_inner())`. The ACP client holds session
state in shared mutable structures across an async read loop and Tauri command
handlers — this is exactly where a panic in one task poisons a lock and
cascades into total copilot failure.

## A-8. Orphaned agent processes

If Xplorer exits while a child agent is running, on Windows the child often
survives (no process-group kill by default). You'll leak a Node process per run
and eventually the user notices `node.exe` eating RAM.

- Use a Windows Job Object, or explicitly kill on `WindowEvent::Destroyed`.
- Also handle the reverse: agent dies mid-turn. Detect EOF on stdout, surface
  "agent exited unexpectedly", answer outstanding requests, don't hang the UI.

## A-9. The MCP server path must be absolute — and it's your own binary

`session/new` `mcpServers[].command` requires the **absolute path to the
executable**. You're pointing it at yourself with `--mcp-server`. Use
`std::env::current_exe()`. In `pnpm dev` this resolves to the debug build under
`target/debug/`, which is fine, but the path differs from production — do not
hardcode either.

## A-10. All Agents MUST support stdio MCP transport; HTTP is optional

Use stdio for the Xplorer MCP server. Only use HTTP if
`mcpCapabilities.http` is advertised. (The spec notes new agents *should*
support HTTP, and that SSE is deprecated by the MCP spec — don't build on SSE.)

## A-11. Line numbers are 1-based

`fs/read_text_file`'s `line` param and `ToolCallLocation.line` are 1-based. Your
editor component and your Rust slicing are almost certainly 0-based. Off-by-one
here shows up as the agent reading the wrong region and producing confidently
wrong edits.

## A-12. Two different `protocolVersion` numbers

ACP protocol version is the integer `1`. The Rust crate
`agent-client-protocol` is at `2.1.0`. The MCP protocol version in
`mcp_server.rs` is the date string `"2024-11-05"`. Three unrelated version
identifiers in one build. Mixing them produces immediate handshake failure with
an unhelpful error.

Version negotiation rule: if the agent responds with a version you don't
support, **close the connection and tell the user** — don't limp along.

## A-13. Don't let the agent keep its own file tools

opencode ships `read`, `write`, `edit`, `multiedit`, `bash`, `patch` and by
default **all tools are enabled and need no permission**. If you don't disable
them, the agent will bypass every guard you wrote and edit the disk directly.
Use `opencode.json` `permission` (supports wildcards; `ask`/`allow`/`deny`) and
disable the built-ins. See `scaffold/config/opencode.json`.

Claude similarly needs its tool set constrained through the adapter.

## A-14. `--mcp-server` mode must be truly headless

When launched with `--mcp-server`, the binary must not initialize a Tauri
window, must not touch the same settings/DB files in write mode as the GUI
instance, and must write **nothing** to stdout except JSON-RPC. Any stray
`println!`, `tracing` writer on stdout, or Tauri startup banner corrupts the
stream and the agent will fail to parse. Route all logs to stderr or a file.

## A-15. Two instances of the same binary, one SQLite file

The GUI instance and the `--mcp-server` instance both want the storage layer
(tags, notes, audit log, search index). Concurrent SQLite writers → `database
is locked`. Options: WAL mode + busy timeout, or have the MCP instance proxy
mutations back to the GUI instance over a local channel. Decide deliberately;
don't discover it in production.

---

## B-1. `session/prompt` prefers `Resource` over `ResourceLink`

The spec says `ContentBlock::Resource` is preferred when available because it
avoids extra round-trips and lets you include context the agent can't reach
itself. Use `Resource` whenever `embeddedContext` is advertised — it's also how
you feed extracted PDF/DOCX text the agent has no parser for.

## B-2. Tool kinds are a fixed enum — use them for icons

`read`, `edit`, `delete`, `move`, `search`, `execute`, `think`, `fetch`,
`other` (default). Map to icons and to your mode-based auto-reject rules.
Don't invent kinds.

## B-3. Tool call statuses and the `pending` ambiguity

`pending`, `in_progress`, `completed`, `failed`. `pending` means *either*
"input still streaming" *or* "awaiting approval" — the same status covers both.
Your UI must distinguish them from context (is there an outstanding permission
request for this `toolCallId`?), or users will think it's hung.

## B-4. `tool_call_update` is a sparse patch

Every field except `toolCallId` is optional; only changed fields are sent.
Merge, never replace. Replacing wipes `title`/`kind` and your row goes blank.

## B-5. Diff `oldText` is null for new files

`ToolCallContent::Diff` has `oldText: string | null`. Null means creation. Your
diff renderer must handle null as "empty side", not crash.

## B-6. `_meta` and `_`-prefixed methods are the sanctioned extension point

Need "target pane" or "current selection" concepts ACP doesn't model? Put them
in `_meta`, and name custom methods with a leading `_`. Implementations MUST NOT
assume anything about `_meta` values. Custom capabilities are advertised through
`_meta` at initialization. Don't fork the protocol types.

## B-7. Register commands in `main.rs`, logic in `operations/`

Existing repo structure. `main.rs` is already ~645 lines with ~300 registered
commands in one `generate_handler!` block. Adding to that block is correct;
putting logic there is not.

## B-8. Never call `invoke()` from a component

Repo rule: `apps/client/src/lib/tauri-api.ts` delegates to `@xplorer/sdk`. Add
a `packages/sdk/src/services/copilot.ts`. There are already 24 service files
following this pattern — match them.

## B-9. The test setup mocks `invoke`

`apps/client/src/__tests__/setup.ts` mocks `@tauri-apps/api/core`. **Every new
Tauri command needs a mock entry**, or tests fail with confusing undefined
errors. Same for new `lucide-react` icons — they're mocked too.

## B-10. Four locale files, always

`en`, `zh`, `ja`, `id` in `apps/client/src/locales/`. Adding a string to only
`en.json` is a lint/review failure. Copilot UI is string-heavy — budget for it.

## B-11. No Tailwind inside extensions

If you do add any extension-side surface, it must use JSX + inline styles +
`--xp-*` CSS variables. Tailwind classes don't exist in the extension sandbox.

## B-12. `console.log` is banned

Only `console.warn` and `console.error`. A streaming agent transcript is
exactly the kind of code where you'll reflexively add `console.log`. Lint will
reject it.

## B-13. 1000-line file ceiling

Repo convention. An ACP client and a streaming chat panel both blow past this
naturally. Plan the module split up front (see the file layout in
`BUILD_GUIDE.md`) rather than refactoring under duress.

## B-14. `usage_update` carries real cost data

Tokens used, context size, and cumulative cost in ISO 4217 currency. Surface it
— it's free spend tracking and it's already in the protocol.

## B-15. Agent versions move fast

`@agentclientprotocol/claude-agent-acp` is at `0.75.x`, the older
`@zed-industries/claude-code-acp` at `0.16.x`, and the Rust crate at `2.1.0`
(updated Sept 2026). Pin exact versions in a lockfile and re-verify the
handshake after any bump. `0.x` means breaking changes are permitted.

## B-16. Xplorer's metadata is path-keyed, not content-keyed

Tags, notes, annotations, and custom metadata are keyed by path. **The copilot's
whole job is moving files.** Every agent move silently orphans that metadata.

Mitigations, cheapest first: (a) intercept every move in the MCP layer and
re-key the storage rows in the same transaction; (b) add a BLAKE3/hash column
via the existing `compute_file_hash` and key metadata on content instead. Do
(a) at minimum, in Phase 8, or the product actively destroys user data as it
"organizes".
