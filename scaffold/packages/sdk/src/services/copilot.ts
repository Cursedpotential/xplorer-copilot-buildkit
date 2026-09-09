/**
 * Copilot service — the ONLY route from React to the ACP client.
 *
 * GOTCHA B-8: never call invoke() from a component. `apps/client/src/lib/
 * tauri-api.ts` delegates to this service, matching the 24 existing services.
 * GOTCHA B-9: every new Tauri command must be added to the invoke mock in
 * `apps/client/src/__tests__/setup.ts` or tests fail with undefined errors.
 */

import { invoke } from '../transport';

/** GOTCHA B-2: fixed enum from the ACP spec. Do not invent kinds. */
export type ToolKind =
  | 'read' | 'edit' | 'delete' | 'move' | 'search'
  | 'execute' | 'think' | 'fetch' | 'other';

/**
 * GOTCHA B-3: `pending` means EITHER "input still streaming" OR "awaiting
 * approval". Same value, two very different UI states. Disambiguate by
 * checking for an outstanding permission request on the same toolCallId, or
 * users will think it's hung.
 */
export type ToolCallStatus = 'pending' | 'in_progress' | 'completed' | 'failed';

export type PermissionOptionKind =
  | 'allow_once' | 'allow_always' | 'reject_once' | 'reject_always';

/** GOTCHA B-5: `oldText` is null for NEW files. Render null as an empty side. */
export type ToolCallContent =
  | { type: 'content'; content: ContentBlock }
  | { type: 'diff'; path: string; oldText: string | null; newText: string }
  | { type: 'terminal'; terminalId: string };

export type ContentBlock =
  | { type: 'text'; text: string }
  /** Baseline — always supported in prompts. */
  | { type: 'resource_link'; uri: string; name?: string; mimeType?: string }
  /**
   * Gated on `promptCapabilities.embeddedContext`.
   * GOTCHA B-1: PREFERRED when available — avoids round-trips and lets you
   * supply extracted PDF/DOCX text the agent has no parser for.
   */
  | { type: 'resource'; resource: { uri: string; mimeType?: string; text?: string } }
  | { type: 'image'; data: string; mimeType: string }
  | { type: 'audio'; data: string; mimeType: string };

/** GOTCHA A-11: `line` is 1-BASED. Your editor component is 0-based. */
export interface ToolCallLocation { path: string; line?: number }

export interface ToolCall {
  toolCallId: string;
  title: string;
  kind?: ToolKind;
  status?: ToolCallStatus;
  content?: ToolCallContent[];
  locations?: ToolCallLocation[];
  rawInput?: unknown;
  rawOutput?: unknown;
}

/** Mirrors NegotiatedCapabilities in acp/mod.rs. Absent === unsupported. */
export interface AgentCapabilities {
  loadSession: boolean;
  promptImage: boolean;
  promptAudio: boolean;
  promptEmbeddedContext: boolean;
  mcpHttp: boolean;
  sessionResume: boolean;
  sessionClose: boolean;
  sessionAdditionalDirectories: boolean;
}

export type CopilotMode = 'discuss' | 'draft' | 'apply';

export const copilotService = {
  connect: (profile: 'claude' | 'opencode'): Promise<string> =>
    invoke('acp_connect', { profile }),

  /**
   * `cwd` = active pane (absolute). `additionalDirectories` = inactive pane.
   *
   * GOTCHA A-4: on reconnect you must re-send the COMPLETE intended list.
   * Omitting it or sending [] activates NO roots — it does not restore stored
   * ones. If the copilot relies on the inactive pane being a root, forgetting
   * this makes the agent silently lose access and start refusing operations
   * for no visible reason.
   */
  newSession: (
    connectionId: string,
    cwd: string,
    additionalDirectories: string[],
  ): Promise<string> =>
    invoke('acp_new_session', { connectionId, cwd, additionalDirectories }),

  prompt: (sessionId: string, blocks: ContentBlock[]): Promise<string> =>
    invoke('acp_prompt', { sessionId, blocks }),

  /**
   * GOTCHA S-7: cancel is a notification and NOT a hard stop. Keep the update
   * sink alive, answer any outstanding permission request with the `cancelled`
   * outcome, wait for the prompt response carrying Cancelled, then clean up.
   */
  cancel: (sessionId: string): Promise<void> =>
    invoke('acp_cancel', { sessionId }),

  /**
   * GOTCHA S-5: allow_always is a persistent grant. It MUST be scoped to
   * (session, tool, path-root) — never to tool name alone, or "always allow
   * move_file" becomes a lifetime license to move anything anywhere.
   */
  respondPermission: (
    sessionId: string, requestId: number, optionId: string,
  ): Promise<void> =>
    invoke('acp_respond_permission', { sessionId, requestId, optionId }),

  /** Only pass modes the agent actually advertised in `availableModes`. */
  setMode: (sessionId: string, mode: CopilotMode): Promise<void> =>
    invoke('acp_set_mode', { sessionId, mode }),

  /** Phase 9. Must work mid-turn and with an outstanding permission request. */
  killSwitch: (sessionId: string): Promise<void> =>
    invoke('acp_kill_switch', { sessionId }),
};
