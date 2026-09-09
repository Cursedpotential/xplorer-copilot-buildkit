# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> _Byline: Claude Code · Fable 5.1 · 2026-09-09_

## What this repository is

A **build kit**, not an application. It holds the plan, reference docs, and stub
files for turning a personal fork of Xplorer (Tauri 2 + Rust + React file
manager) into a dual-pane file manager with an ACP-driven AI copilot. There is
nothing to build, lint, or test *here*. All real code lives in the fork after
the kit is copied in (see `README.md` quick start). The fork's verification
loop and repo conventions are in `CLAUDE.copilot.md`, not here.

Two `CLAUDE` files exist and serve different readers:

| File | Audience | Edit it when |
|---|---|---|
| `CLAUDE.md` (this file) | Claude working **in the kit** | kit layout or policy changes |
| `CLAUDE.copilot.md` | Claude working **in the fork**; copied to the fork root and imported via `@CLAUDE.copilot.md` | fork conventions, guard rules, or the verification loop change |

Never merge the two. `CLAUDE.copilot.md` must stay self-contained because it
is read alongside upstream Xplorer's own `CLAUDE.md`, which it must not override.

## Ownership and git policy (owner ruling 2026-09-09)

- This is a **personal tool**. No contributions go back to upstream.
- Upstream `kimlimjustin/xplorer` is **fetch-only**: keep it as an `upstream`
  remote to pull updates, with push disabled
  (`git remote set-url --push upstream DISABLED`). **Never push to it under any
  circumstances.**
- Push only to the owner's own fork and to this kit's own repo
  (`origin` on the `Cursedpotential` GitHub account).
- Fork from branch `next`, not `main`. That is the rewrite the kit targets.

## Layout and reading order

- `BUILD_GUIDE.md` is the one-page architecture and the safety inversion. Read first.
- `docs/00` to `07` are in reading order. `00-PREFLIGHT` is a hard gate;
  `04-GOTCHAS` ranks every trap S/A/B; `06-PROMPTS` has one copy-paste Claude
  Code prompt per phase; `07-QUICK-REFERENCE` is the one-page cheat sheet.
- `scaffold/` mirrors the fork's tree exactly (`apps/src-tauri/src/acp/`,
  `packages/sdk/src/services/`, `apps/client/src/hooks/`). Every stub carries
  `TODO(claude) Phase N` markers tied to `docs/01-PHASES.md`.
- `scaffold/config/opencode.json` denies every opencode built-in filesystem
  and shell tool and routes everything through the `xplorer` MCP server. This
  is the single most important safety artifact in the kit.
- `docs/planning/` holds kit-internal daily TODO files. Do **not** copy it into the fork.

## Architecture the docs describe

Xplorer is the **ACP Client** and spawns `claude-agent-acp` or `opencode acp`
as a child process, speaking JSON-RPC over stdio. The agent reaches Xplorer's
file-manager verbs back through **MCP** (the same binary launched with
`--mcp-server`). Every agent-supplied path funnels through the existing
`agent::security` guards, all deletion routes to `move_to_trash`, and every
mutation is preview-then-commit via `session/request_permission` and the
`plan` session update.

Three unrelated version identifiers coexist and must never be mixed:
ACP wire `protocolVersion` is the integer `1`, the `agent-client-protocol`
crate is `2.x`, and the MCP protocol version is the date string `"2024-11-05"`.

## Conventions for editing the kit

- **Gotcha IDs are stable identifiers.** `S-1`..`S-8`, `A-1`..`A-15`, `B-1`..`B-16`
  in `docs/04-GOTCHAS.md` are cited by number from the phase plan, the test
  doc, the stub comments, `opencode.json`, and the scaffold READMEs. Never
  renumber. Add new gotchas at the end of their severity band and update
  every cross-reference in the same change.
- **Phases have exit criteria.** A change to `docs/01-PHASES.md` must keep
  each phase's `**Exit:**` line, and `docs/06-PROMPTS.md` must stay in step
  (one prompt per phase, same numbering).
- **Stub paths are contracts.** A path under `scaffold/` must match the
  "Where code goes" table in `CLAUDE.copilot.md` and the fork's real tree.
- **Doc drift is the enemy.** When a fact changes (agent package versions,
  the MCP tool count, a capability name), grep the whole kit for the old
  value and correct every occurrence in the same turn.
- Every doc carries a byline line (`tool · model · date`) under its title.
