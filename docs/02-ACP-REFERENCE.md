# ACP v1 — condensed implementation reference

Xplorer is the **Client**. Claude / opencode is the **Agent**. Transport is
JSON-RPC 2.0 over the agent subprocess's stdio.

Canonical schema: `https://github.com/agentclientprotocol/agent-client-protocol/releases/latest/download/schema.json`

---

## Handshake

```
Client ──initialize──▶ Agent
Client ◀─capabilities─ Agent
[optional] Client ──authenticate──▶ Agent
Client ──session/new──▶ Agent
Client ◀──sessionId──── Agent
```

### `initialize` request (Client → Agent)

```json
{
  "jsonrpc": "2.0", "id": 0, "method": "initialize",
  "params": {
    "protocolVersion": 1,
    "clientCapabilities": {
      "fs": { "readTextFile": true, "writeTextFile": true },
      "terminal": true
    },
    "clientInfo": { "name": "xplorer", "title": "Xplorer", "version": "0.1.0" }
  }
}
```

Client capability fields: `fs.readTextFile`, `fs.writeTextFile`, `terminal`,
`elicitation` (object advertising `form`/`url` modes — note ACP does **not**
treat `{}` as form support, unlike MCP), `session.configOptions.boolean`.

### `initialize` response (Agent → Client)

Defaults if omitted:
```json
{"auth":{},"loadSession":false,
 "mcpCapabilities":{"http":false,"sse":false},
 "promptCapabilities":{"audio":false,"embeddedContext":false,"image":false},
 "sessionCapabilities":{}}
```

| Field | Meaning |
|---|---|
| `loadSession` | `session/load` available (**top-level**, not under `sessionCapabilities`) |
| `promptCapabilities.image` / `.audio` / `.embeddedContext` | which non-baseline `ContentBlock`s you may send |
| `mcpCapabilities.http` / `.sse` | non-stdio MCP transports (SSE deprecated) |
| `auth.logout` | `logout` available |
| `sessionCapabilities.resume` / `.close` / `.delete` / `.list` / `.additionalDirectories` | optional session methods |

**Version negotiation:** send your latest. If the agent replies with a version
you don't support, close the connection and inform the user.

**Capability rule:** anything omitted is UNSUPPORTED. Never call an
unadvertised method.

---

## Agent methods (you call these)

| Method | Required? | Notes |
|---|---|---|
| `initialize` | yes | once per connection |
| `authenticate` | conditional | `methodId` from advertised `authMethods`; needed if `session/new` returns `auth_required` |
| `session/new` | **baseline** | `cwd` (absolute) + `mcpServers[]`; optional `additionalDirectories[]` |
| `session/prompt` | **baseline** | `sessionId` + `prompt: ContentBlock[]`; returns `stopReason` |
| `session/cancel` | **baseline** | notification |
| `session/load` | if `loadSession` | replays history via `session/update`, *then* responds |
| `session/resume` | if `sessionCapabilities.resume` | restores without replay |
| `session/close` | if `sessionCapabilities.close` | cancels work + frees resources |
| `session/delete` | if `sessionCapabilities.delete` | removes from `session/list` |
| `session/list` | if `sessionCapabilities.list` | cursor pagination, `cwd` filter |
| `session/set_mode` | if modes advertised | callable while idle **or** generating |
| `session/set_config_option` | if config options advertised | returns full option set |
| `logout` | if `auth.logout` | all new sessions then require auth |

### `session/new`

```json
{
  "jsonrpc":"2.0","id":1,"method":"session/new",
  "params":{
    "cwd":"C:/Users/matt/Documents",
    "additionalDirectories":["C:/Users/matt/Downloads"],
    "mcpServers":[
      {"name":"xplorer","command":"C:/abs/xplorer.exe","args":["--mcp-server"],"env":[]}
    ]
  }
}
```

`cwd` MUST be absolute, MUST be the session base for relative paths regardless
of where the subprocess was spawned, and MUST be in the effective root set.
Effective root set = `[cwd, ...additionalDirectories]` and SHOULD bound
filesystem tool operations.

MCP transports: **stdio is mandatory for all agents**. HTTP takes
`{type:"http",name,url,headers[]}`. SSE takes `{type:"sse",...}` and is
deprecated.

---

## Client methods (you implement these)

| Method | Gate | Purpose |
|---|---|---|
| `fs/read_text_file` | `fs.readTextFile` | `path` (abs), `line` (1-based), `limit` → `{content}` |
| `fs/write_text_file` | `fs.writeTextFile` | `path` (abs), `content` |
| `session/request_permission` | always | `toolCall: ToolCallUpdate`, `options: PermissionOption[]` → `{outcome}` |
| `session/update` | always | notification sink, **no response** |
| `terminal/create` | `terminal` | → `{terminalId}` immediately |
| `terminal/output` | `terminal` | → `{output, truncated, exitStatus?}` |
| `terminal/wait_for_exit` | `terminal` | → `{exitCode, signal}` |
| `terminal/kill` | `terminal` | terminal stays valid afterward |
| `terminal/release` | `terminal` | id becomes invalid; tool-call output SHOULD still display |
| `elicitation/create` | `elicitation` | only advertised modes |

### Permission outcomes

```json
{"outcome":{"outcome":"selected","optionId":"allow-once"}}
{"outcome":{"outcome":"cancelled"}}
```

`PermissionOption.kind` ∈ `allow_once` | `allow_always` | `reject_once` |
`reject_always`. Clients MAY auto-answer from user settings.

### `terminal/create` params

`sessionId`, `command`, `args[]`, `env[]` (`{name,value}`), `cwd` (absolute),
`outputByteLimit`. On exceeding the limit the **Client** truncates from the
**beginning**, at a character boundary.

Timeout is built client-side by the agent: create → race a timer against
`wait_for_exit` → `kill` → `output` → `release`.

---

## `session/update` variants

`user_message_chunk`, `agent_message_chunk`, `agent_thought_chunk`,
`tool_call`, `tool_call_update`, `plan`, `available_commands_update`,
`current_mode_update`, `config_option_update`, `session_info_update`,
`usage_update`.

Clients SHOULD keep accepting `tool_call` updates **after** sending
`session/cancel`.

### `tool_call`

```json
{"sessionUpdate":"tool_call","toolCallId":"call_001",
 "title":"Reading configuration file","kind":"read","status":"pending"}
```

Fields: `toolCallId` (req), `title` (req), `kind`, `status`, `content[]`,
`locations[]`, `rawInput`, `rawOutput`.

`kind` ∈ `read` | `edit` | `delete` | `move` | `search` | `execute` | `think` |
`fetch` | `other` (default).

`status` ∈ `pending` (streaming **or** awaiting approval) | `in_progress` |
`completed` | `failed`.

`tool_call_update` sends only changed fields — merge by `toolCallId`.

### `ToolCallContent` variants

```json
{"type":"content","content":{"type":"text","text":"..."}}
{"type":"diff","path":"C:/abs/file.json","oldText":null,"newText":"..."}
{"type":"terminal","terminalId":"term_xyz789"}
```

`locations[]` entries are `{path, line?}` (1-based) — drive follow-along UI.

---

## Prompt content blocks

Baseline (always supported): `Text`, `ResourceLink`.
Gated: `Image` (`promptCapabilities.image`), `Audio` (`.audio`),
`Resource` (`.embeddedContext`).

`Resource` is **preferred** when available: avoids round-trips and lets you
supply context the agent can't reach. This is how you deliver extracted
PDF/DOCX text.

## Extensibility

`_meta` on every request/response/notification. Custom methods prefixed `_`.
Custom capabilities advertised via `_meta` at init. Implementations MUST NOT
assume anything about `_meta` values.
