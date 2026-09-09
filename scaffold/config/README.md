# Agent launch configs

## `opencode.json`

Denies every opencode built-in that can touch the filesystem or shell
(`read`, `write`, `edit`, `multiedit`, `patch`, `bash`, `webfetch`) and routes
everything through the `xplorer` MCP server instead.

**Gotcha A-13:** opencode's default is *all tools enabled, no permission
required*. Without this file the agent bypasses every guard in
`acp/handlers.rs` and writes straight to disk. Permission values are
`ask` | `allow` | `deny`, and wildcards are supported.

Read-only verbs are `allow` (no interruption). Every mutating verb is `ask`,
which surfaces as a `session/request_permission` call your approval UI answers.

Replace the `command` path with the value from `std::env::current_exe()`.
Under `pnpm dev` that resolves into `target/debug/` — different from
production. Never hardcode either.

## Claude

`@agentclientprotocol/claude-agent-acp` (0.75.x) is the current adapter;
`@zed-industries/claude-code-acp` (0.16.x) is the older Zed one. Constrain its
tool set through the adapter's allowed-tools/settings mechanism so the same
inversion holds: no direct filesystem access, everything through Xplorer's MCP
verbs.

**Gotcha B-15:** both are `0.x` — breaking changes are permitted between minor
versions. Pin exact versions in the lockfile and re-run the Phase 0 handshake
check after any bump.

## Capability recording

Phase 0 requires writing each agent's `agentCapabilities` verbatim into
`docs/copilot/agent-capabilities.md`. They differ between agents, and
**anything omitted is UNSUPPORTED** — you must never call an unadvertised
method (Gotcha A-1). `promptCapabilities.embeddedContext` is the one that most
changes behavior: without it, Phase 5 must fall back to `ResourceLink` and
cannot inline extracted document text.
