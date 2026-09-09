# Build Guide — Xplorer ACP Copilot

## The one-paragraph version

Fork `kimlimjustin/xplorer` at branch `next`. Add a new Rust module
`apps/src-tauri/src/acp/` that makes Xplorer an **ACP Client**: it spawns
`claude-agent-acp` or `opencode acp` as a child process, speaks JSON-RPC 2.0
over that child's stdio, and implements the four Client-side methods
(`fs/read_text_file`, `fs/write_text_file`, `session/request_permission`,
`terminal/*`) plus the `session/update` notification sink. Extend
`mcp_host.rs` from its current 6 tools to a full file-manager verb set and hand
that MCP server to the agent at `session/new`. Every path and command from the
agent is filtered through the existing `agent/security.rs`. The React side gets
a copilot panel that turns the dual-pane file selection into ACP
`ContentBlock[]` and renders `plan` / `tool_call` updates as an approval queue.

## Why this shape

Four properties fall out of ACP for free, and each one is a requirement you'd
otherwise have to build:

| Requirement | ACP mechanism |
|---|---|
| "Copilot immediately sees my selection" | `session/prompt` accepts `ContentBlock::Resource` with inline text — one block per selected file |
| "Discuss before doing" | `session/request_permission` is a **baseline Client method**; the agent must ask you |
| "Show me the batch plan" | the `plan` session update carries entries with content/priority/status |
| "Switch between Claude and opencode" | both are ACP agents; swapping is a subprocess-command change |

## Architecture

```
┌──────────────────────────── Xplorer (Tauri 2) ───────────────────────────┐
│                                                                          │
│  React (apps/client)                                                     │
│   ├─ Dual pane (use-split-layout, use-pane-sync, use-cross-tab-selection)│
│   ├─ CopilotPanel ──── renders plan + tool_call queue + approvals        │
│   └─ use-copilot-context ── selection → ContentBlock[]                   │
│            │  (via packages/sdk → tauri-api facade, never raw invoke)    │
│  ══════════╪═══════════════ Tauri IPC ═══════════════════════════════    │
│            ▼                                                             │
│  Rust (apps/src-tauri)                                                   │
│   ├─ acp/            ← NEW. ACP Client: transport, session, handlers     │
│   │    ├─ transport.rs   framed JSON-RPC over child stdio                │
│   │    ├─ client.rs      initialize / session lifecycle / prompt         │
│   │    ├─ handlers.rs    fs/*, terminal/*, request_permission            │
│   │    ├─ events.rs      session/update → Tauri emit                     │
│   │    └─ agents.rs      subprocess launch profiles (claude | opencode)  │
│   ├─ mcp_host.rs     ← EXTEND. 6 tools → full file-manager verb set      │
│   ├─ agent/security.rs   ← REUSE AS-IS. the guard layer                  │
│   ├─ operations/     ← REUSE. move/copy/rename/trash/bulk_rename         │
│   ├─ pty.rs          ← REUSE. backs terminal/*                           │
│   └─ audit_log.rs    ← REUSE. every agent mutation gets logged           │
└──────────────────────────────────────────────────────────────────────────┘
                    │ stdio JSON-RPC (ACP v1)      │ stdio JSON-RPC (MCP)
                    ▼                               ▼
        ┌───────────────────────┐        ┌──────────────────────────┐
        │ claude-agent-acp      │───────▶│ xplorer --mcp-server     │
        │   or  opencode acp    │  MCP   │ (file-manager verbs)     │
        └───────────────────────┘        └──────────────────────────┘
```

Note the loop: the agent reaches Xplorer's file-manager verbs **through MCP**,
and Xplorer reaches the agent **through ACP**. Both are stdio JSON-RPC. The
MCP server is the same binary launched with `--mcp-server`.

## The inversion that makes it safe

Both candidate agents ship their own filesystem and shell tools. Those must be
**turned off** so the agent can only act through Xplorer's permissioned verbs:

- **opencode**: `opencode.json` `permission` block + disable `read`, `write`,
  `edit`, `multiedit`, `bash`, `patch`. See `scaffold/config/opencode.json`.
- **Claude**: restrict via `--allowedTools` / settings passed to
  `claude-agent-acp`, and rely on `session/request_permission` for the rest.

This is the pattern `josephschmitt/opencode-acp` uses, and it is the single
most important safety decision in the build.

## Read next

1. `docs/00-PREFLIGHT.md` — do not skip. If Windows build fails, stop.
2. `docs/04-GOTCHAS.md` — read before writing code, not after.
3. `docs/01-PHASES.md` — the actual work.
