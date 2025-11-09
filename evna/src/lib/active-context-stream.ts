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

    // Auto-create bridge if structured capture with project + issue
    await this.maybeCreateBridge(captured);
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

    // Query active_context_stream table directly
    const results = await this.db.queryActiveContext({
      limit: exclude_current_session ? limit * 2 : limit, // Get more if we'll filter
      project,
      since,
      client_type,
    });

    // Convert to CapturedMessage format
    let messages: CapturedMessage[] = results.map((row) => ({
      message_id: row.message_id,
      conversation_id: row.conversation_id,
      role: row.role as 'user' | 'assistant',
      content: row.content,
      timestamp: new Date(row.timestamp),
      client_type: row.client_type as 'desktop' | 'claude_code' | undefined,
      metadata: row.metadata,
    }));

    // Post-process filtering
    if (exclude_current_session && this.currentSessionId) {
      messages = messages.filter((m) => m.conversation_id !== this.currentSessionId);
    }

    if (personas && personas.length > 0) {
      messages = messages.filter((m) =>
        personas.some((p) => m.metadata.personas?.includes(p))
      );
    }

    return messages.slice(0, limit);
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
   * Auto-create or update bridge for structured captures (project + issue)
   */
  private async maybeCreateBridge(message: CapturedMessage): Promise<void> {
    const { metadata, content } = message;

    // Only create bridge if we have project + issue + substantial content
    if (!metadata.project || !metadata.issue || content.length < 200) {
      return;
    }

    try {
      // Generate bridge filename: rangle-pharmacy-issue-656.md
      const projectSlug = this.slugify(metadata.project);
      const issueNumber = metadata.issue.replace(/[^0-9]/g, ''); // Extract just numbers
      const bridgeFilename = `${projectSlug}-issue-${issueNumber}.md`;

      console.error(`[active-context] Auto-bridge: ${bridgeFilename}`);

      // Check if bridge exists
      const { readFile, writeFile, mkdir } = await import('fs/promises');
      const { join } = await import('path');
      const { homedir } = await import('os');

      const bridgesDir = join(homedir(), 'float-hub', 'float.dispatch', 'bridges');
      const bridgePath = join(bridgesDir, bridgeFilename);

      // Ensure bridges directory exists
      await mkdir(bridgesDir, { recursive: true });

      // Get current timestamp for section header
      const timestamp = new Date().toISOString();
      const dateTime = timestamp.split('T')[0] + ' @ ' +
                       new Date().toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: true });

      let bridgeExists = false;
      try {
        await readFile(bridgePath, 'utf-8');
        bridgeExists = true;
      } catch (error) {
        // Bridge doesn't exist yet
      }

      if (bridgeExists) {
        // Append to existing bridge
        const updateSection = `\n\n## Update: ${dateTime}\n\n${content}\n`;
        await writeFile(bridgePath, updateSection, { flag: 'a' }); // Append mode
        console.error(`[active-context] Appended to bridge: ${bridgeFilename}`);
      } else {
        // Create new bridge with frontmatter
        const newBridge = `---
type: work_log
created: ${timestamp}
project: ${metadata.project}
issue: ${metadata.issue}
---

# ${metadata.project} - Issue #${issueNumber}

## Created: ${dateTime}

${content}
`;
        await writeFile(bridgePath, newBridge, 'utf-8');
        console.error(`[active-context] Created new bridge: ${bridgeFilename}`);
      }
    } catch (error) {
      console.error('[active-context] Error creating bridge:', error);
      // Graceful failure - don't throw
    }
  }

  /**
   * Slugify text for bridge filenames
   */
  private slugify(text: string): string {
    return text
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '')
      .slice(0, 50);
  }

  /**
   * Store captured message to database
   */
  private async storeMessage(message: CapturedMessage): Promise<void> {
    // Store to active_context_stream table with full JSONB metadata
    await this.db.storeActiveContext({
      message_id: message.message_id,
      conversation_id: message.conversation_id,
      role: message.role,
      content: message.content,
      timestamp: message.timestamp,
      client_type: message.client_type,
      metadata: message.metadata as Record<string, any>,
    });
  }

  /**
   * Generate unique message ID
   */
  private generateMessageId(): string {
    return `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Smart truncation that preserves sentence boundaries and critical details
   * @param content - The content to truncate
   * @param maxLength - Maximum length (default: 1200)
   * @returns Truncated content with clean boundaries and [n/x] indicator
   */
  private smartTruncate(content: string, maxLength: number = 1200): string {
    const originalLength = content.length;

    // Short enough? Return as-is
    if (content.length <= maxLength) {
      return content;
    }

    // Try sentence boundary (. ! ?) within reasonable range
    // Search backwards from maxLength + 50 to find last sentence ending
    const searchEnd = Math.min(maxLength + 50, content.length);
    const searchText = content.substring(0, searchEnd);

    // Find last sentence ending by searching backwards
    const lastPeriod = searchText.lastIndexOf('. ');
    const lastExclaim = searchText.lastIndexOf('! ');
    const lastQuestion = searchText.lastIndexOf('? ');
    const endPos = Math.max(lastPeriod, lastExclaim, lastQuestion);

    let truncated: string;

    // Use sentence boundary if reasonably close to maxLength
    if (endPos > maxLength - 100) {
      truncated = content.substring(0, endPos + 1).trim(); // +1 to include punctuation
    }
    // No good sentence boundary, try word boundary
    else {
      const wordBoundary = content.lastIndexOf(' ', maxLength);
      if (wordBoundary > maxLength - 50) {
        truncated = content.substring(0, wordBoundary).trim() + '...';
      } else {
        // Fallback: hard truncate at maxLength
        truncated = content.substring(0, maxLength).trim() + '...';
      }
    }

    // Add [n/x] indicator showing truncated/total chars
    return `${truncated} [${truncated.length}/${originalLength}]`;
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

      // Show preview of content with smart truncation
      const preview = this.smartTruncate(msg.content);
      lines.push(`\n${preview}\n`);

      if (msg.metadata.highlights && msg.metadata.highlights.length > 0) {
        lines.push(`**Highlights**: ${msg.metadata.highlights.join('; ')}`);
      }

      lines.push('---\n');
    });

    return lines.join('\n');
  }
}
