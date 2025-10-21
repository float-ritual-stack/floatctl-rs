/**
 * Active Context Stream
 * Real-time message capture with annotation parsing
 */

import { DatabaseClient } from './db.js';
import { AnnotationParser, MessageMetadata } from './annotation-parser.js';

export interface CapturedMessage {
  message_id: string;
  conversation_id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  client_type?: 'desktop' | 'claude_code';
  metadata: MessageMetadata;
}

export interface ContextQuery {
  limit?: number;
  client_type?: 'desktop' | 'claude_code';
  exclude_current_session?: boolean;
  project?: string;
  since?: Date;
  personas?: string[];
}

export class ActiveContextStream {
  private parser: AnnotationParser;
  private currentSessionId?: string;
  private currentClientType?: 'desktop' | 'claude_code';

  constructor(private db: DatabaseClient) {
    this.parser = new AnnotationParser();
  }

  /**
   * Capture a message to active context stream
   */
  async captureMessage(message: {
    message_id?: string;
    conversation_id: string;
    role: 'user' | 'assistant';
    content: string;
    timestamp?: Date;
    client_type?: 'desktop' | 'claude_code';
  }): Promise<void> {
    const metadata = this.parser.extractMetadata(message.content);

    const captured: CapturedMessage = {
      message_id: message.message_id || this.generateMessageId(),
      conversation_id: message.conversation_id,
      role: message.role,
      content: message.content,
      timestamp: message.timestamp || new Date(),
      client_type: message.client_type || this.detectClientType(message.content),
      metadata,
    };

    // Store to database with rich metadata
    await this.storeMessage(captured);
  }

  /**
   * Query active context with client-aware filtering
   */
  async queryContext(query: ContextQuery): Promise<CapturedMessage[]> {
    const {
      limit = 10,
      client_type,
      exclude_current_session = false,
      project,
      since,
      personas,
    } = query;

    // Build query for recent messages
    // Note: This would integrate with actual database schema
    // For now, using getRecentMessages instead of semantic search (no embedding needed)

    const sinceISO = since ? since.toISOString() : undefined;

    // Use getRecentMessages for recency-based queries (no embedding required)
    const messages = await this.db.getRecentMessages({
      limit,
      project,
      since: sinceISO,
    });

    // Post-process for client-aware filtering
    // (This would be more efficient with proper database queries)
    let filtered = messages.map((msg) => ({
      message_id: msg.id || '',
      conversation_id: msg.conversation_id,
      role: msg.role as 'user' | 'assistant',
      content: msg.content,
      timestamp: new Date(msg.timestamp),
      client_type: undefined as 'desktop' | 'claude_code' | undefined,
      metadata: {
        project: msg.project || undefined,
        personas: [],
        connections: [],
        highlights: [],
        commands: [],
        patterns: [],
      },
    }));

    if (client_type) {
      filtered = filtered.filter((m) => m.client_type === client_type);
    }

    if (exclude_current_session && this.currentSessionId) {
      filtered = filtered.filter((m) => m.conversation_id !== this.currentSessionId);
    }

    return filtered.slice(0, limit);
  }

  /**
   * Set current session context for filtering
   */
  setSession(sessionId: string, clientType: 'desktop' | 'claude_code'): void {
    this.currentSessionId = sessionId;
    this.currentClientType = clientType;
  }

  /**
   * Get client-aware context for current session
   * - Desktop: Surface claude_code context (exclude same-session echoes)
   * - Claude Code: Surface desktop context
   * - First message: Surface all relevant context
   */
  async getClientAwareContext(options: {
    isFirstMessage: boolean;
    project?: string;
    limit?: number;
  }): Promise<CapturedMessage[]> {
    const { isFirstMessage, project, limit = 10 } = options;

    if (isFirstMessage) {
      // First message: surface all relevant context
      return this.queryContext({
        limit,
        project,
        exclude_current_session: false,
      });
    }

    // Subsequent messages: cross-client context surfacing
    const otherClientType =
      this.currentClientType === 'desktop' ? 'claude_code' : 'desktop';

    return this.queryContext({
      limit,
      project,
      client_type: otherClientType,
      exclude_current_session: true,
    });
  }

  /**
   * Detect client type from message patterns
   * (This is a heuristic - ideally would come from MCP connection metadata)
   */
  private detectClientType(content: string): 'desktop' | 'claude_code' {
    // Heuristics for detection:
    // - Claude Code messages often have code blocks, file paths, technical commands
    // - Desktop messages tend to be more conversational

    const hasCodeBlocks = /```[\s\S]*?```/.test(content);
    const hasFilePaths = /\/[\w/-]+\.[\w]+/.test(content);
    const hasTechnicalCommands = /(cargo|npm|git|bash|cd|ls|grep)\s+/i.test(content);

    if (hasCodeBlocks || hasFilePaths || hasTechnicalCommands) {
      return 'claude_code';
    }

    return 'desktop';
  }

  /**
   * Store captured message to database
   */
  private async storeMessage(message: CapturedMessage): Promise<void> {
    // For now, we'll use the existing messages table
    // In production, might want a separate active_context table

    // Note: This is a placeholder - actual implementation would depend on
    // database schema for active context stream
    // Could use Chroma collection or extend messages table with metadata

    // Extract for database storage
    const project = message.metadata.project;
    const markers = [
      ...(message.metadata.personas || []),
      ...(message.metadata.patterns || []),
      ...(message.metadata.highlights || []),
    ];

    // Store message with metadata
    // This would integrate with actual database schema
    // For now, relying on semantic search via Rust CLI
  }

  /**
   * Generate unique message ID
   */
  private generateMessageId(): string {
    return `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Format context for display
   */
  formatContext(messages: CapturedMessage[]): string {
    if (messages.length === 0) {
      return '**No active context found**';
    }

    const lines: string[] = [];
    lines.push(`## 🔴 Active Context Stream (${messages.length} messages)\n`);

    messages.forEach((msg, idx) => {
      const timestamp = msg.timestamp.toLocaleString();
      const clientBadge = msg.client_type === 'claude_code' ? '💻' : '💬';
      const roleBadge = msg.role === 'user' ? '👤' : '🤖';

      lines.push(`### ${idx + 1}. ${clientBadge} ${roleBadge} ${timestamp}`);

      if (msg.metadata.project) {
        lines.push(`**Project**: ${msg.metadata.project}`);
      }

      if (msg.metadata.personas && msg.metadata.personas.length > 0) {
        lines.push(`**Personas**: ${msg.metadata.personas.join(', ')}`);
      }

      if (msg.metadata.ctx) {
        const ctx = msg.metadata.ctx;
        if (ctx.mode) {
          lines.push(`**Mode**: ${ctx.mode}`);
        }
      }

      // Show preview of content
      const preview = msg.content.substring(0, 200);
      lines.push(`\n${preview}${msg.content.length > 200 ? '...' : ''}\n`);

      if (msg.metadata.highlights && msg.metadata.highlights.length > 0) {
        lines.push(`**Highlights**: ${msg.metadata.highlights.join('; ')}`);
      }

      lines.push('---\n');
    });

    return lines.join('\n');
  }
}
