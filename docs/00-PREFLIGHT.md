# Phase 0 — Preflight (Windows build gate)

> _Byline: Claude Code · Fable 5.1 · 2026-09-09 (added fetch-only upstream remote step)_

**Stop condition: if this phase fails, do not proceed. Everything downstream is
worthless if the fork does not build and run on your machine.**

The repo's own `CLAUDE.md` states "Windows paths: this runs on Windows", and
there is a dedicated `windows_recycle_bin.rs` plus Windows registry shell
integration commands. Windows is supported. But the modern rewrite is at
`v1.0.0-alpha.1` (April 2026) with last push June 2026 — treat it as alpha.

## Toolchain

| Requirement | Notes |
|---|---|
| Node.js 18+ | |
| pnpm | **Never `npm install`** — repo rule, pnpm workspaces |
| Rust stable | repo pins via `rust-toolchain.toml` — let it govern |
| Tauri CLI v2 | |
| Visual Studio Build Tools | **"Desktop development with C++" workload** |
| WebView2 Runtime | preinstalled on Win11, verify on Win10 |
| Node ≥ 18 for agents | `claude-agent-acp` / `opencode` are Node CLIs |

## Steps

```bash
git clone https://github.com/kimlimjustin/xplorer.git xplorer-copilot
cd xplorer-copilot
git checkout next            # NOT main/master — next is the default & the rewrite
git checkout -b feat/acp-copilot

# Personal fork: upstream is fetch-only, never push to it
git remote rename origin upstream
git remote set-url --push upstream DISABLED
git remote add origin https://github.com/<you>/xplorer-copilot.git
pnpm install                 # not npm
pnpm run dev                 # Vite (5174) + Tauri window
```

Then, in order:

```bash
npx tsc --noEmit
pnpm run lint
npx vitest run
cd apps/src-tauri && cargo test
cd apps/src-tauri && cargo build
```

## Exit criteria — all must pass

- [ ] `pnpm run dev` opens a window and lists a real directory.
- [ ] Navigate into a folder, back out, and switch drives (`list_drives`).
- [ ] **Trash round-trip works**: delete a scratch file → appears via
      `get_trash_items` → `restore_trash_item` brings it back. This exercises
      `windows_recycle_bin.rs`, which is the highest-risk platform code.
- [ ] Dual pane opens (`use-split-layout`) and both panes navigate independently.
- [ ] `cargo build` completes with no errors.
- [ ] `npx tsc --noEmit` clean.

## Verify the existing MCP server before writing any ACP code

The binary already contains a JSON-RPC MCP server. Prove the plumbing works
first — it is a 10-minute check that de-risks the entire build.

```bash
# from apps/src-tauri, after cargo build
./target/debug/xplorer --mcp-server
```

Then paste on stdin (one JSON object per line):

```json
{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}
{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
```

Expect protocol version `2024-11-05`, server name `xplorer`, and exactly six
tools: `read_file`, `write_file`, `list_directory`, `search_files`,
`run_command`, `get_git_status`.

- [ ] `initialize` returns capabilities.
- [ ] `tools/list` returns the six tools.
- [ ] A `tools/call` on `list_directory` returns real entries.

## Verify both agents independently

Before wiring anything, confirm each agent starts and speaks ACP.

```bash
npm i -g @agentclientprotocol/claude-agent-acp   # latest 0.75.x
# or the older Zed adapter: npm i -g @zed-industries/claude-code-acp  (0.16.x)
npm i -g opencode
```

Drive each by hand:

```bash
claude-agent-acp
# stdin:
{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{"fs":{"readTextFile":true,"writeTextFile":true},"terminal":true},"clientInfo":{"name":"xplorer","title":"Xplorer","version":"0.1.0"}}}
```

```bash
opencode acp
# same initialize payload
```

- [ ] Both return `protocolVersion: 1`.
- [ ] Record each one's `agentCapabilities` verbatim into
      `docs/copilot/agent-capabilities.md`. **You will need this**: capabilities
      differ between agents and you MUST NOT call an unadvertised method.
- [ ] Note whether `loadSession`, `sessionCapabilities.resume`,
      `sessionCapabilities.close`, `sessionCapabilities.additionalDirectories`,
      `promptCapabilities.embeddedContext`, and `mcpCapabilities.http` are present.

`promptCapabilities.embeddedContext` is the critical one — without it you
cannot send `ContentBlock::Resource` and must fall back to `ResourceLink`.
