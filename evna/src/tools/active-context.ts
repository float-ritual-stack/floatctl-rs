/**
 * Active Context Tool
 * Query and capture live context with annotation parsing.
 *
 * Trigger-bus shape (May 3 2026): when `capture` is provided, the body
 * of the capture is auto-used as a semantic query against the substrate
 * via RecallTool. Lateral lore relevant to what was just captured comes
 * back alongside the capture confirmation — the agent doesn't have to
 * pick between active_context and recall, and the matcher's
 * expandProjectAliases recency-mode bug is sidestepped (semantic path
 * uses the RPC's own canonicalization, not the substring matcher).
 *
 * Operationalizes the March 4 "active_context as trigger bus" shower-
 * thought: capture is the event signal; recall fires automatically.
 */

import { ActiveContextStream, CaptureRejectedError } from '../lib/active-context-stream.js';
import { DatabaseClient, SearchResult } from '../lib/db.js';
import { ollama, OLLAMA_MODELS } from '../lib/ollama-client.js';
import { buildActiveContextSynthesisPrompt, SYNTHESIS_PRESETS } from '../prompts/active-context-synthesis.js';
import { collectPeripheralContext, formatPeripheralContext } from '../lib/peripheral-context.js';
import { RecallTool } from './recall.js';

export interface ActiveContextOptions {
  query?: string;
  capture?: string; // Message to capture
  limit?: number;
  project?: string;
  client_type?: 'desktop' | 'claude_code';
  include_cross_client?: boolean;
  synthesize?: boolean; // Use Ollama to synthesize context (default: true; ignored on capture trigger path)
  include_peripheral?: boolean; // Include daily notes + other projects (default: true; ignored on capture trigger path)
}

export class ActiveContextTool {
  private stream: ActiveContextStream;
  private currentProjectFilter?: string;

  constructor(db: DatabaseClient, private recall: RecallTool) {
    this.stream = new ActiveContextStream(db);
  }

  /**
   * Query active context. When `capture` is provided, the captured body
   * fires a recall semantic search automatically (trigger-bus shape) —
   * lateral lore returns alongside the capture confirmation.
   */
  async query(options: ActiveContextOptions): Promise<string> {
    const {
      query,
      capture,
      limit,
      project,
      client_type,
      include_cross_client = true,
      synthesize = true,
      include_peripheral = true,
    } = options;

    // Capture path. CaptureRejectedError bubbles up as a user-facing
    // teaching message — the three-path TIGHTEN/THREAD/PROMOTE guidance
    // is the response shape, not an error to be hidden behind "Error
    // during active context query".
    let capturedMessageId: string | null = null;
    if (capture) {
      try {
        const result = await this.stream.captureMessage({
          conversation_id: this.generateConversationId(),
          role: 'user',
          content: capture,
          timestamp: new Date(),
          client_type,
          project,
        });
        capturedMessageId = result.message_id;
      } catch (err) {
        if (err instanceof CaptureRejectedError) {
          return err.userMessage;
        }
        throw err;
      }
    }

    // Trigger-bus: when we just captured, body becomes the query and we
    // route through recall (semantic) instead of queryContext (recency).
    // Default limit drops to 5 because the trigger path wants lateral
    // lore, not an exhaustive feed — agents can override via `limit`.
    if (capture) {
      const triggerLimit = limit ?? 5;
      const effectiveQuery = query ?? capture;
      try {
        const results = await this.recall.search({
          query: effectiveQuery,
          limit: triggerLimit + 1, // +1 to accommodate the just-captured row before filter
          project,
          threshold: 0.3, // looser than recall's default 0.5 — lateral lore can be tangential
        });
        const filtered = capturedMessageId
          ? results.filter((r) => r.message.id !== capturedMessageId)
          : results;
        const sliced = filtered.slice(0, triggerLimit);
        if (sliced.length === 0) {
          return `**Captured.** No lateral lore matched the body content (threshold 0.3, ${triggerLimit} slot${triggerLimit === 1 ? '' : 's'}). Substrate may be light on this thread.`;
        }
        const header = `**Captured.** Body fired auto-recall — lateral lore below.\n\n`;
        return header + this.recall.formatResults(sliced as SearchResult[]);
      } catch (err) {
        // Recall failure on trigger path is non-fatal — capture already
        // succeeded; just acknowledge and let the agent continue.
        return `**Captured.** Lateral-lore lookup errored: ${err instanceof Error ? err.message : String(err)}. Capture itself is safe in the substrate.`;
      }
    }

    // Pure-query path (no capture). Existing recency + optional
    // synthesis flow preserved for back-compat with brain_boot and
    // other callers that use active_context for ambient awareness.
    this.currentProjectFilter = project;

    const messages = await this.stream.queryContext({
      limit: limit ?? 10,
      project,
      client_type: include_cross_client ? undefined : client_type,
    });

    if (messages.length === 0) {
      return "**No active context available**";
    }

    if (!synthesize || !query) {
      return this.stream.formatContext(messages);
    }

    return this.synthesizeContext(query, messages, include_peripheral);
  }

  /**
   * Synthesize active context using Ollama
   * Filters irrelevant content and avoids repeating user's query
   */
  private async synthesizeContext(query: string, messages: any[], includePeripheral: boolean): Promise<string> {
    // Select best available model from preference chain
    const model = await ollama.selectModel(OLLAMA_MODELS.balanced);

    if (!model) {
      console.error("[active_context] Ollama not available, falling back to raw format");
      return this.stream.formatContext(messages);
    }

    try {
      // Prepare context for synthesis
      const contextText = messages.map((m, i) => {
        const timestamp = new Date(m.timestamp).toLocaleString("en-US", { timeZone: "America/Toronto" });
        const project = m.metadata?.project ? `[${m.metadata.project}]` : "";
        return `[${i + 1}] ${timestamp} ${project}\n${m.content.substring(0, 400)}`;
      }).join("\n\n---\n\n");

      // Collect peripheral context if enabled
      let peripheralContext: string | undefined;
      if (includePeripheral) {
        const peripheral = await collectPeripheralContext();
        peripheralContext = formatPeripheralContext(peripheral);
      }

      // Use externalized prompt (easy to tweak)
      const preset = SYNTHESIS_PRESETS.default;
      const prompt = buildActiveContextSynthesisPrompt({
        query,
        contextText,
        maxWords: preset.maxWords,
        tweetSize: preset.tweetSize,
        projectFilter: this.currentProjectFilter,
        peripheralContext,
      });

      const synthesis = await ollama.generate({
        model, // use selected model from fallback chain
        prompt,
        temperature: preset.temperature,
      });

      return `## Active Context Synthesis\n\n${synthesis}`;
    } catch (error) {
      console.error("[active_context] Synthesis error:", error);
      // Fallback to raw format on error
      return this.stream.formatContext(messages);
    }
  }

  /**
   * Get client-aware context
   * Surfaces context from the other client (Desktop ↔ Claude Code)
   */
  async getClientAwareContext(options: {
    current_client: 'desktop' | 'claude_code';
    is_first_message: boolean;
    project?: string;
    limit?: number;
  }): Promise<string> {
    const { current_client, is_first_message, project, limit = 10 } = options;

    this.stream.setSession(this.generateConversationId(), current_client);

    const messages = await this.stream.getClientAwareContext({
      isFirstMessage: is_first_message,
      project,
      limit,
    });

    if (messages.length === 0) {
      return '**No active context available**';
    }

    const clientBadge = current_client === 'claude_code' ? '💻' : '💬';
    const otherClientBadge = current_client === 'claude_code' ? '💬' : '💻';

    const lines: string[] = [];
    lines.push(`## ${clientBadge} → ${otherClientBadge} Cross-Client Context\n`);
    lines.push(this.stream.formatContext(messages));

    return lines.join('\n');
  }

  /**
   * Generate a conversation ID (simplified for now)
   */
  private generateConversationId(): string {
    return `conv_${Date.now()}`;
  }
}
