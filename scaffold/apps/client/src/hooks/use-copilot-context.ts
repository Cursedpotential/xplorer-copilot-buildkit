/**
 * Turns the dual-pane file selection into ACP prompt content.
 *
 * This hook is the product's core idea: selection IS the context object.
 *
 * GOTCHA: never concatenate paths into the prompt text string. That is what
 * the existing packages/extensions/claude-code extension does and it is the
 * anti-pattern this workstream replaces. Structured blocks only.
 *
 * Build on the EXISTING selection model — use-cross-tab-selection.ts,
 * use-pane-sync.ts, use-split-layout.ts. Do not invent a parallel one.
 */

import type { ContentBlock, AgentCapabilities } from '@xplorer/sdk';

/** Total inline-text budget across all resource blocks. Tune empirically. */
const CONTEXT_BYTE_BUDGET = 256 * 1024;

export interface SelectedEntry {
  path: string;          // absolute
  name: string;
  isDirectory: boolean;
  size: number;
  mimeType?: string;
}

/**
 * GOTCHA S-4: `file://` URIs for Windows drive letters need THREE slashes:
 * `file:///C:/Users/matt/x.txt`. Two slashes makes `C:` a hostname.
 * Also percent-encode `#`, `%`, and spaces — the unicode/ fixture directory
 * exists to catch exactly this.
 */
export const toFileUri = (absPath: string): string => {
  const forward = absPath.replace(/\\/g, '/');
  const withRoot = forward.startsWith('/') ? forward : `/${forward}`;
  return `file://${withRoot.split('/').map(encodeURIComponent).join('/')}`;
};

/**
 * GOTCHA A-1 / B-1: prefer ContentBlock::Resource when
 * `promptEmbeddedContext` is advertised, since it avoids round-trips and can
 * carry text the agent cannot obtain itself (extracted PDF/DOCX/XLSX/PPTX).
 * Otherwise fall back to ResourceLink, which is baseline and always safe.
 * Never send Resource on a capability you did not observe.
 */
export const buildContextBlocks = (
  instruction: string,
  selection: SelectedEntry[],
  caps: AgentCapabilities,
  extractedText: Map<string, string>,
): ContentBlock[] => {
  const blocks: ContentBlock[] = [{ type: 'text', text: instruction }];
  let spent = 0;

  for (const entry of selection) {
    const uri = toFileUri(entry.path);

    if (!caps.promptEmbeddedContext || entry.isDirectory) {
      blocks.push({
        type: 'resource_link',
        uri,
        name: entry.name,
        mimeType: entry.mimeType,
      });
      continue;
    }

    const text = extractedText.get(entry.path);
    if (text === undefined) {
      // Binary or unparsed: metadata only, never bytes.
      blocks.push({ type: 'resource_link', uri, name: entry.name, mimeType: entry.mimeType });
      continue;
    }

    // TODO(claude) Phase 5: truncate at a CHARACTER boundary, not a byte
    // index, and surface truncation visibly in the chip UI so the user knows
    // the agent saw a partial file.
    const remaining = CONTEXT_BYTE_BUDGET - spent;
    const slice = text.length <= remaining ? text : text.slice(0, remaining);
    spent += slice.length;

    blocks.push({
      type: 'resource',
      resource: { uri, mimeType: entry.mimeType, text: slice },
    });
  }

  return blocks;
};

export const useCopilotContext = () => {
  // TODO(claude) Phase 5:
  //   - read active + inactive pane selection from the existing hooks
  //   - call operations::extract_document_text for pdf/docx/xlsx/pptx
  //   - expose removable file chips
  //   - expose { blocks, totalBytes, truncatedPaths }
  //   - add all new strings to en/zh/ja/id (GOTCHA B-10)
  throw new Error('TODO(claude) Phase 5');
};
