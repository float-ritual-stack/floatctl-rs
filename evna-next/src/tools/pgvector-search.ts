/**
 * pgvector Semantic Search Tool
 * Direct semantic search against conversation history
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
   */
  async search(options: SearchOptions): Promise<SearchResult[]> {
    const { query, limit = 10, project, since, threshold = 0.5 } = options;

    // Perform semantic search via Rust CLI (no embedding needed)
    const results = await this.db.semanticSearch(query, {
      limit,
      project,
      since,
      threshold,
    });

    return results;
  }

  /**
   * Format search results as markdown
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
      const conversationTitle = conversation?.title || conversation?.conv_id || 'Unknown';

      lines.push(`## ${idx + 1}. ${timestamp}${projectTag} (similarity: ${similarity.toFixed(2)})`);
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
