/**
 * Active Context Tool
 * Query and capture live context with annotation parsing
 */

import { ActiveContextStream } from '../lib/active-context-stream.js';
import { DatabaseClient } from '../lib/db.js';

export interface ActiveContextOptions {
  query?: string;
  capture?: string; // Message to capture
  limit?: number;
  project?: string;
  client_type?: 'desktop' | 'claude_code';
  include_cross_client?: boolean;
}

export class ActiveContextTool {
  private stream: ActiveContextStream;

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

    // Query context
    const messages = await this.stream.queryContext({
      limit,
      project,
      client_type: include_cross_client ? undefined : client_type,
    });

    // Format results
    return this.stream.formatContext(messages);
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
