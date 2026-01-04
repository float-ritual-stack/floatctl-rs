/**
 * Active Context Tool
 * Query and capture live context with annotation parsing
 */

import { ActiveContextStream } from '../lib/active-context-stream.js';
import { DatabaseClient } from '../lib/db.js';
import { ollama, OLLAMA_MODELS } from '../lib/ollama-client.js';
import { buildActiveContextSynthesisPrompt, SYNTHESIS_PRESETS } from '../prompts/active-context-synthesis.js';
import { collectPeripheralContext, formatPeripheralContext } from '../lib/peripheral-context.js';

export interface ActiveContextOptions {
  query?: string;
  capture?: string; // Message to capture
  limit?: number;
  project?: string;
  client_type?: 'desktop' | 'claude_code';
  include_cross_client?: boolean;
  synthesize?: boolean; // Use Ollama to synthesize context (default: true)
  include_peripheral?: boolean; // Include daily notes + other projects (default: true)
}

export class ActiveContextTool {
  private stream: ActiveContextStream;
  private currentProjectFilter?: string;

  constructor(db: DatabaseClient) {
    this.stream = new ActiveContextStream(db);
  }

  /**
   * Query active context with optional capture
   */
  async query(options: ActiveContextOptions): Promise<string> {
    const {
      query,
      capture,
      limit = 10,
      project,
      client_type,
      include_cross_client = true,
      synthesize = true,
      include_peripheral = true,
    } = options;

    // Capture message if provided
    if (capture) {
      await this.stream.captureMessage({
        conversation_id: this.generateConversationId(),
        role: 'user',
        content: capture,
        timestamp: new Date(),
        client_type,
      });
    }

    // Store project filter for synthesis
    this.currentProjectFilter = project;

    // Query context
    const messages = await this.stream.queryContext({
      limit,
      project,
      client_type: include_cross_client ? undefined : client_type,
    });

    // If no messages, return early
    if (messages.length === 0) {
      return "**No active context available**";
    }

    // If synthesis disabled or no query provided, return raw formatted
    if (!synthesize || !query) {
      return this.stream.formatContext(messages);
    }

    // Synthesize context using Ollama (cost-free)
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
      const rawContext = this.stream.formatContext(messages);
      return `> ⚠️ **Note**: Ollama unavailable - showing raw context (no synthesis)\n\n${rawContext}`;
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
      // Fallback to raw format on error with warning
      const rawContext = this.stream.formatContext(messages);
      return `> ⚠️ **Note**: Synthesis failed - showing raw context\n\n${rawContext}`;
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
