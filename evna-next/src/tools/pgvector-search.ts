/**
 * pgvector Semantic Search Tool
 * Direct semantic search against conversation history
 * Searches BOTH active_context_stream (recent, real-time) AND embeddings (historical, archived)
 */

import { DatabaseClient, SearchResult } from '../lib/db.js';
import { EmbeddingsClient } from '../lib/embeddings.js';

export interface SearchOptions {
  query: string;
  limit?: number;
  project?: string;
  since?: string;
  threshold?: number;
}

export class PgVectorSearchTool {
  constructor(
    private db: DatabaseClient,
    private embeddings: EmbeddingsClient
  ) {}

  /**
   * Perform semantic search across conversation history
   * Two-tier search: active_context_stream (recent) + embeddings (historical)
   * Active context results appear first (priority), then semantic results
   */
  async search(options: SearchOptions): Promise<SearchResult[]> {
    const { query, limit = 10, project, since, threshold = 0.5 } = options;

    // Calculate lookback for active context (default: 7 days)
    const lookbackDate = since ? new Date(since) : new Date(Date.now() - 7 * 24 * 60 * 60 * 1000);

    // Query 1: Active context stream (recent messages, no embeddings)
    // Get more than limit to account for deduplication
    const activeContextMessages = await this.db.queryActiveContext({
      limit: Math.min(limit * 2, 20), // Get 2x limit or max 20
      project,
      since: lookbackDate,
    });

    // Query 2: Semantic search via embeddings (historical)
    const embeddingResults = await this.db.semanticSearch(query, {
      limit,
      project,
      since: since || lookbackDate.toISOString(),
      threshold,
    });

    // Convert active context messages to SearchResult format
    // Assign similarity = 1.0 to indicate "priority" (recent = more relevant)
    const activeResults: SearchResult[] = activeContextMessages.map((msg) => ({
      message: {
        id: msg.message_id,
        conversation_id: msg.conversation_id,
        idx: 0,
        role: msg.role,
        timestamp: msg.timestamp,
        content: msg.content,
        project: msg.metadata?.project || null,
        meeting: msg.metadata?.meeting || null,
        markers: [], // Active context doesn't have markers (yet)
      },
      conversation: {
        id: msg.conversation_id,
        conv_id: msg.conversation_id,
        title: null, // Active context doesn't track conversation titles
        created_at: msg.timestamp,
        markers: [],
      },
      similarity: 1.0, // Priority score for recent context
    }));

    // Merge results: active context first, then embeddings
    const allResults = [...activeResults, ...embeddingResults];

    // Deduplicate by message ID
    const seen = new Set<string>();
    const deduplicated = allResults.filter((result) => {
      if (seen.has(result.message.id)) {
        return false;
      }
      seen.add(result.message.id);
      return true;
    });

    // Return top N results
    return deduplicated.slice(0, limit);
  }

  /**
   * Format search results as markdown
   * Handles both active_context (recent) and embeddings (historical) results
   */
  formatResults(results: SearchResult[]): string {
    if (results.length === 0) {
      return '**No results found**';
    }

    const lines: string[] = [];
    lines.push(`# Search Results (${results.length} matches)\n`);

    results.forEach((result, idx) => {
      const { message, conversation, similarity } = result;
      const timestamp = new Date(message.timestamp).toLocaleString();
      const projectTag = message.project ? ` [${message.project}]` : '';

      // Active context results have similarity = 1.0 (priority)
      const isActiveContext = similarity === 1.0;
      const sourceTag = isActiveContext ? ' 🔴 Recent' : '';
      const conversationTitle = conversation?.title || (isActiveContext ? 'Active Context' : conversation?.conv_id || 'Unknown');

      lines.push(`## ${idx + 1}. ${timestamp}${projectTag}${sourceTag} (similarity: ${similarity.toFixed(2)})`);
      lines.push(`**Conversation**: ${conversationTitle}`);
      lines.push(`**Role**: ${message.role}`);
      lines.push('');
      lines.push(message.content);
      lines.push('');
      lines.push('---');
      lines.push('');
    });

    return lines.join('\n');
  }
}
