# CLAUDE.copilot.md — ACP Copilot workstream

> Append `@CLAUDE.copilot.md` to the repo's existing `CLAUDE.md` rather than
> overwriting it. The upstream `CLAUDE.md` contains binding conventions.
>
> _Byline: Claude Code · Fable 5.1 · 2026-09-09 (renamed from the kit's `CLAUDE.md`; git policy section added)_

## Git policy (owner ruling 2026-09-09)

This fork is a **personal tool**. Nothing is contributed back upstream.

- `upstream` = `kimlimjustin/xplorer`, **fetch-only**. Pull updates from it;
  **never push to it under any circumstances.** Keep its push URL disabled.
- `origin` = the owner's own fork. Push feature branches there only.
- Base branch is upstream `next`, not `main`.

## Mission

Add a **dual-pane-aware AI copilot** to this Xplorer fork. The copilot is an
external agent process (Claude or opencode) speaking **Agent Client Protocol
(ACP) v1** over JSON-RPC on stdio. Xplorer is the **ACP Client**. The agent is
the **ACP Agent**. Xplorer additionally exposes file-manager verbs to the agent
as an **MCP server**.

Non-goal: do not extend or improve the existing Rust `agent/` module's own LLM
loop. Treat `agent/security.rs` as a **library** to reuse and `agent/tools.rs`
as a **vocabulary reference**. The shipped agent loop is being superseded.

## Absolute rules

1. **Never bypass `agent::security`.** Every path that arrives from the agent
   (via `fs/write_text_file`, `fs/read_text_file`, an MCP tool arg, or a
   `terminal/create` cwd) MUST pass `validate_agent_path_with_permissions`
   before use. Every command string MUST pass `is_blocked_command` and, when
   the internet sandbox is on, `is_network_command`.
2. **Never hard-delete.** All agent-initiated deletion routes to
   `operations::move_to_trash`. `remove_file` / `remove_dir` are not reachable
   from any agent-facing surface. No exceptions, not even behind a flag.
3. **No `invoke()` in components.** Add commands via a `packages/sdk` service,
   surfaced through `apps/client/src/lib/tauri-api.ts`. This is an existing
   repo rule and it applies to all copilot code.
4. **Never scrape a PTY to talk to an agent.** The existing
   `packages/extensions/claude-code` extension does this. It is the anti-pattern
   this whole workstream replaces. Do not copy from it except its file-chip UI.
5. **Do not implement the copilot as a sandboxed extension.** The extension
   sandbox blocks `__TAURI__`, `fetch`, and process spawning, and `XplorerAPI`
   has no subprocess namespace. The copilot is native Rust + first-party React.
6. **Every mutation is preview-then-commit.** An agent tool that changes the
   filesystem must be expressible as a plan the user approves. Use
   `session/request_permission` for per-call approval and the `plan` session
   update for batch approval.
7. **Absolute paths only, always.** ACP requires absolute paths on the wire.
   Normalize at the boundary, never mid-pipeline.

## Verification loop (run after every change)

```bash
npx tsc --noEmit                  # must pass
npx vitest run <changed-test>     # affected tests only
pnpm run lint                     # zero errors
cd apps/src-tauri && cargo test   # for Rust changes
cd apps/src-tauri && cargo clippy -- -D warnings
```

Do not run the full suite unless asked. Single test files for speed.

## Where code goes

| Concern | Location |
|---|---|
| ACP client (JSON-RPC, transport, session state) | `apps/src-tauri/src/acp/` (new module) |
| ACP Tauri commands | register in `apps/src-tauri/src/main.rs` |
| MCP tool surface for the agent | `apps/src-tauri/src/mcp_host.rs` (extend) + `src/mcp/` |
| Path/command guards | reuse `apps/src-tauri/src/agent/security.rs` |
| TS service layer | `packages/sdk/src/services/copilot.ts` |
| Facade | `apps/client/src/lib/tauri-api.ts` |
| Copilot UI | `apps/client/src/components/copilot/` |
| Selection→context hook | `apps/client/src/hooks/use-copilot-context.ts` |

## Repo conventions you must not break

- Arrow functions only. No `function` keyword.
- Files under 1000 lines. Extract before you cross it.
- TS strict. `unknown` + narrowing, never `any`.
- `===` always (`== null` allowed).
- `console.warn` / `console.error` only. No `console.log`.
- PascalCase components, `use-kebab-case.ts` hooks.
- Rust: `chrono` for time. Guard every Mutex with
  `.lock().unwrap_or_else(|e| e.into_inner())` — poisoning is a live risk here.
- Do not hand-edit `Cargo.lock`.
- New user-facing strings go in **all four** locales: `en`, `zh`, `ja`, `id`.
- Extensions use JSX + inline styles + CSS vars. Never Tailwind in extensions.

## Commits

Conventional style (`feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`).
Concise, focused on *why*. No `Co-Authored-By` tags.
