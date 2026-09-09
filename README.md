# Xplorer Copilot Build Kit

> _Byline: Claude Code · Fable 5.1 · 2026-09-09 (README revised: CLAUDE file split + ownership note)_

A complete, agent-executable build plan for turning a fork of **Xplorer**
(Tauri 2 + Rust + React file manager) into a dual-pane file manager with an
**ACP-based AI copilot** that can be driven by **Claude** (`@agentclientprotocol/claude-agent-acp`)
or **opencode** (`opencode acp`).

## What's in here

| Path | Purpose |
|---|---|
| `CLAUDE.md` | Guidance for Claude Code working **in this kit** (not for the fork). |
| `CLAUDE.copilot.md` | **Drop this at your fork root.** Operating rules for Claude Code inside the fork. |
| `BUILD_GUIDE.md` | The master build plan. Read this first. |
| `docs/00-PREFLIGHT.md` | Windows build gate. Do this before writing code. |
| `docs/01-PHASES.md` | Phase-by-phase implementation plan with exit criteria. |
| `docs/02-ACP-REFERENCE.md` | Condensed ACP v1 spec: every method you must implement. |
| `docs/03-TOOL-CATALOG.md` | Xplorer Tauri commands → MCP tools mapping table. |
| `docs/04-GOTCHAS.md` | **The most important file.** Every trap, ranked by severity. |
| `docs/05-TESTING.md` | Test strategy, fixtures, and the destructive-op test harness. |
| `docs/06-PROMPTS.md` | Copy-paste task prompts for Claude Code, one per phase. |
| `scaffold/` | Stub files to drop into the fork. Every stub has `TODO(claude)` markers. |
| `scaffold/config/` | `opencode.json` and agent launch configs. |

## Quick start

```bash
git clone https://github.com/kimlimjustin/xplorer.git xplorer-copilot
cd xplorer-copilot
git checkout next
git checkout -b feat/acp-copilot

# Personal fork: upstream is fetch-only, never push to it
git remote rename origin upstream
git remote set-url --push upstream DISABLED
git remote add origin https://github.com/<you>/xplorer-copilot.git

# Merge this kit in
cp /path/to/xplorer-copilot-buildkit/CLAUDE.copilot.md ./CLAUDE.copilot.md
cp -r /path/to/xplorer-copilot-buildkit/docs ./docs/copilot
cp -r /path/to/xplorer-copilot-buildkit/scaffold/* ./

# Then hand Claude Code: docs/copilot/06-PROMPTS.md, Phase 0.
```

> **Note on `CLAUDE.md`:** the upstream repo already ships a `CLAUDE.md`. Do **not**
> overwrite it — it contains real conventions you must obey. Copy this kit's
> `CLAUDE.copilot.md` in and append an `@CLAUDE.copilot.md` import line to the
> existing file, or merge the sections manually. This kit's own `CLAUDE.md` is
> for working on the kit and is not meant for the fork.

> **Ownership:** this is a personal fork. Upstream `kimlimjustin/xplorer` is
> fetch-only for pulling updates — never push to it. Push only to your own fork
> and to this kit's own repo. `docs/planning/` is kit-internal; do not copy it.
