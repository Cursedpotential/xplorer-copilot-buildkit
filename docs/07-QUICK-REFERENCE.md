# One-page quick reference

## The four numbers
| Identifier | Value |
|---|---|
| ACP protocol version (wire) | `1` |
| `agent-client-protocol` crate | `2.x` |
| MCP protocol version (`mcp_server.rs`) | `"2024-11-05"` |
| `@agentclientprotocol/claude-agent-acp` | `0.75.x` |

## Methods you implement (Client side)
`fs/read_text_file` · `fs/write_text_file` · `session/request_permission` ·
`session/update` (notification, no response) · `terminal/create` ·
`terminal/output` · `terminal/wait_for_exit` · `terminal/kill` ·
`terminal/release`

## Methods you call (Agent side)
Baseline: `initialize` · `session/new` · `session/prompt` · `session/cancel`
Gated: `authenticate` · `session/load` · `session/resume` · `session/close` ·
`session/delete` · `session/list` · `session/set_mode` ·
`session/set_config_option` · `logout`

## Enums
- **ToolKind**: read, edit, delete, move, search, execute, think, fetch, other
- **ToolCallStatus**: pending, in_progress, completed, failed
- **PermissionOptionKind**: allow_once, allow_always, reject_once, reject_always
- **ToolCallContent**: content, diff, terminal
- **ContentBlock**: text, resource_link (baseline) / resource, image, audio (gated)

## The 8 rules that matter most
1. All deletion → `move_to_trash`. `remove_file`/`remove_dir` unreachable.
2. Lexical normalize first (destinations don't exist yet), then canonicalize
   the existing prefix (junctions).
3. Omitted capability = unsupported. Never call an unadvertised method.
4. `allow_always` scoped to (session, tool, path-root), expires on close.
5. Permission requests block the agent — timeout must resolve to rejection.
6. Cancel isn't a stop: keep the sink alive, answer outstanding requests with
   `cancelled`, wait for the `StopReason`.
7. `tool_call_update` is a sparse patch — merge, never replace.
8. Re-key path-scoped tags/notes inside every move transaction.

## Verification loop
```
npx tsc --noEmit && pnpm run lint && cargo clippy -- -D warnings
npx vitest run <changed-test> ; cargo test
```

## Repo rules that fail review
pnpm not npm · no `invoke()` in components · arrow functions only · no
`console.log` · files <1000 lines · four locale files · Mutex guard pattern ·
no Tailwind in extensions · don't edit `Cargo.lock`
