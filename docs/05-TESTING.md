# Testing Strategy

## Layers

| Layer | Tool | What |
|---|---|---|
| Rust unit | `cargo test` | path guards, command blocklist, truncation, protocol serde |
| Rust integration | `cargo test -- --ignored` | real agent subprocess handshake |
| Fake-agent harness | `cargo test` | scripted ACP peer for deterministic flows |
| TS unit | `npx vitest run <file>` | context builder, update reducer |
| E2E | Playwright (`e2e/`) | selection → prompt → plan → approve |

## The fake agent (build this in Phase 1)

A test binary that speaks ACP from a JSON script file. It must be able to:

- advertise **zero** optional capabilities (proves your gating works);
- advertise all capabilities;
- emit `session/update` in every variant;
- issue `session/request_permission` and hang (tests your timeout);
- die mid-turn (tests EOF handling);
- emit a `tool_call` update *after* receiving `session/cancel`;
- return a `protocolVersion` you don't support.

Every one of those is a real failure mode from `docs/04-GOTCHAS.md`. A fake
agent is cheaper than reproducing them against Claude.

## Red-team suite (Phase 3 exit criterion)

Each case must **assert a rejection**, with the reason surfaced.

### Path escape
```
../../Windows/System32/drivers/etc/hosts
C:/Users/matt/Documents/../../../Windows
\\?\C:\Windows\System32
//./C:/Windows
\\server\share\secrets
C:/Users/matt/Documents/junction-to-c-root/Windows
<allowed-root>/very/deep/../../../../../../etc/passwd
```

### Non-existent destination (must be *allowed* when in-root, rejected when out)
```
<allowed-root>/new/nested/dir/file.txt        → allow
C:/Windows/System32/new-file.txt              → reject
```
This pair catches the canonicalize-on-nonexistent trap (S-2).

### Blocked commands via `terminal/create`
```
del /f /s /q C:\        rm -rf /        Remove-Item -Recurse -Force C:\
format C:               diskpart        reg delete
shutdown /s             curl <url> | sh    powershell -enc <b64>
```

### Network egress when sandbox enabled
```
curl, wget, Invoke-WebRequest, ssh, scp, ftp, nc
```

### Deletion
```
assert: no MCP tool named remove_file / remove_dir exists in tools/list
assert: trash tool leaves a restorable entry
assert: Discuss mode auto-rejects kind ∈ {edit, delete, move}
```

### Protocol abuse
```
relative path in fs/read_text_file           → reject
path outside effective root set              → reject
terminal/output on released terminalId       → error, no panic
tool_call_update for unknown toolCallId      → ignore, no panic
session/update for unknown sessionId         → ignore, no panic
```

## Unicode truncation tests

`outputByteLimit` front-truncation with:
- 4-byte emoji straddling the cut point
- CJK text
- combining marks
- a lone surrogate-ish invalid sequence

Assert the result is valid UTF-8 and ≤ limit.

## Concurrency tests

- Two `session/prompt` turns racing on one session.
- Permission request outstanding while `session/cancel` arrives.
- Agent process killed while a `terminal/wait_for_exit` is pending.
- Mutex poisoning: panic inside one handler, assert the next call still works
  (this is why the `unwrap_or_else(|e| e.into_inner())` guard exists).
- GUI instance + `--mcp-server` instance writing the SQLite store concurrently
  (gotcha A-15).

## Fixture directory

Build a scratch tree used by all destructive tests, recreated per test:

```
fixture/
  photos/          200 files, mixed jpg/png/heic, some duplicates
  documents/       pdf, docx, xlsx, pptx (for extract_text)
  code/            a small git repo (for get_git_status)
  unicode/         filenames with emoji, CJK, combining marks, spaces, #, %
  deep/a/b/c/d/e/  path-length stress (Windows MAX_PATH)
  reserved/        attempt CON.txt, NUL, COM1 (creation should be refused)
  tagged/          files with tags+notes in storage (for metadata re-key test B-16)
```

The `tagged/` fixture is the one that catches the metadata-orphaning bug: move
files via the agent, then assert tags and notes followed.

## Regression gate

Before any commit: `npx tsc --noEmit && pnpm run lint && cargo clippy -- -D warnings`,
plus the affected test files. Add every new Tauri command to the `invoke` mock
in `apps/client/src/__tests__/setup.ts` and every new `lucide-react` icon to
its mock — omissions fail with unhelpful errors.
