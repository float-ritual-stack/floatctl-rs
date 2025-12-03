/**
 * Semantic Search Tool (AutoRAG + active_context)
 * Direct semantic search against conversation history
 * Searches BOTH active_context_stream (recent, real-time) AND AutoRAG (historical, archived)
 *
 * Nov 28, 2025: Removed OpenAI embeddings dependency (vestigial limb from pgvector era)
 * - AutoRAG handles all historical semantic search
 * - Active context trusts recency as relevance proxy (already project-filtered + time-limited)
 */

import { DatabaseClient, SearchResult } from '../lib/db.js';

export interface SearchOptions {
  query: string;
  limit?: number;
  project?: string;
  since?: string;
  threshold?: number;
}

export class PgVectorSearchTool {
  constructor(
    private db: DatabaseClient
  ) {}

  /**
   * Perform semantic search across conversation history
   * Two-tier search: active_context_stream (recent) + AutoRAG (historical)
   * Active context results appear first (priority), then semantic results
   */
  async search(options: SearchOptions): Promise<SearchResult[]> {
    const { query, limit = 10, project, since, threshold = 0.5 } = options;

    // Calculate lookback for active context (default: 7 days)
    const lookbackDate = since ? new Date(since) : new Date(Date.now() - 7 * 24 * 60 * 60 * 1000);

    // Query 1: Active context stream (recent messages)
    // RABBIT: Fast, recent context (limit to 30% of total results to leave room for turtle)
    // Trust recency as relevance proxy - already project-filtered + time-limited
    const activeLimit = Math.max(Math.floor(limit * 0.3), 3); // 30% or min 3
    const activeContextMessages = await this.db.queryActiveContext({
      limit: activeLimit,
      project,
      since: lookbackDate,
    });

    // Query 2: Semantic search via AutoRAG (historical)
    // TURTLE: Deep, historical knowledge (get more to account for active_context overlap)
    const embeddingResults = await this.db.semanticSearch(query, {
      limit: limit * 2, // Get 2x to account for potential overlap with active_context
      project,
      since: since || lookbackDate.toISOString(),
      threshold,
    });

    // Convert active_context to SearchResult format
    // Recency-based relevance: flat 0.9 marks as "recent context, not semantic match"
    const activeResults: SearchResult[] = activeContextMessages.map((msg, idx) => ({
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
      similarity: 0.9, // Flat score for active_context (recency = relevance proxy)
      source: 'active_context', // Mark as active_context for brain_boot
    }));

    // Merge results: active context first, then embeddings
    const allResults = [...activeResults, ...embeddingResults];

    // Deduplicate by composite key (conversation_id + timestamp + content prefix)
    // Note: message.id is empty string for Rust CLI results, so use composite key
    const seen = new Set<string>();
    const deduplicated = allResults.filter((result) => {
      const key = `${result.message.conversation_id}::${result.message.timestamp}::${result.message.content.substring(0, 50)}`;
      if (seen.has(key)) {
        return false;
      }
      seen.add(key);
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
      const { message, conversation, similarity, source } = result;
      const timestamp = new Date(message.timestamp).toLocaleString();
      const projectTag = message.project ? ` [${message.project}]` : '';

      // Use source field instead of similarity === 1.0
      const isActiveContext = source === 'active_context';
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
