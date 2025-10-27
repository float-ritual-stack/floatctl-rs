/**
 * Brain Boot Tool
 * Rich context synthesis for morning check-ins and context restoration
 */

import { DatabaseClient } from '../lib/db.js';
import { EmbeddingsClient } from '../lib/embeddings.js';
import { GitHubClient } from '../lib/github.js';
import { DailyNotesReader } from '../lib/daily-notes.js';
import { ActiveContextStream } from '../lib/active-context-stream.js';
import { CohereReranker } from '../lib/cohere-reranker.js';
import { PgVectorSearchTool } from './pgvector-search.js'; // Dual-source semantic search (embeddings + active_context)

export interface BrainBootOptions {
  query: string;
  project?: string;
  lookbackDays?: number;
  maxResults?: number;
  githubRepo?: string;
  githubUsername?: string;
  includeDailyNote?: boolean; // If true, include full daily note (defaults to false, can get long)
}

export interface BrainBootResult {
  summary: string;
  relevantContext: Array<{
    content: string;
    timestamp: string;
    project?: string;
    conversation?: string;
    similarity: number;
  }>;
  recentActivity: Array<{
    content: string;
    timestamp: string;
    project?: string;
  }>;
}

export class BrainBootTool {
  private github?: GitHubClient;
  private dailyNotes: DailyNotesReader;
  private activeContext: ActiveContextStream;
  private reranker?: CohereReranker; // Cohere cross-encoder for multi-source fusion
  private pgvectorTool: PgVectorSearchTool; // Dual-source semantic search (embeddings + active_context)

  constructor(
    private db: DatabaseClient,
    private embeddings: EmbeddingsClient,
    githubRepo?: string,
    dailyNotesDir?: string,
    cohereApiKey?: string // Optional Cohere API key for reranking
  ) {
    if (githubRepo) {
      this.github = new GitHubClient(githubRepo);
    }
    if (cohereApiKey) {
      this.reranker = new CohereReranker(cohereApiKey);
    }
    this.dailyNotes = new DailyNotesReader(dailyNotesDir);
    this.activeContext = new ActiveContextStream(db);
    this.pgvectorTool = new PgVectorSearchTool(db, embeddings); // NEW: Dual-source search tool
  }

  /**
   * Perform a brain boot: semantic search + recent context + GitHub status + synthesis
   */
  async boot(options: BrainBootOptions): Promise<BrainBootResult> {
    const {
      query,
      project,
      lookbackDays = 7,
      maxResults = 10,
      githubUsername,
      includeDailyNote = false,
    } = options;

    // Calculate since timestamp
    const since = new Date();
    since.setDate(since.getDate() - lookbackDays);
    const sinceISO = since.toISOString();

    // Parallel fetch: MULTI-SOURCE search
    // Strategy: If project is specified, fetch WITH project first (prioritized)
    // Then backfill with unfiltered results if needed (soft filter, not hard exclusion)

    // 1. Message embeddings + active_context (dual-source via pgvectorTool)
    const semanticWithProject = project
      ? await this.pgvectorTool.search({
          query,
          limit: maxResults,
          project,
          since: sinceISO,
          threshold: 0.3,
        })
      : [];

    const semanticFallback = semanticWithProject.length < maxResults
      ? await this.pgvectorTool.search({
          query,
          limit: maxResults * 2, // Get extra for deduplication
          project: undefined, // No filter - get all
          since: sinceISO,
          threshold: 0.3,
        })
      : [];

    // Merge: project-filtered first, then backfill with unfiltered (deduplicated)
    const semanticResultsRaw = [...semanticWithProject, ...semanticFallback];
    const seenMessages = new Set<string>();
    const semanticResults = semanticResultsRaw
      .filter((r) => {
        const key = `${r.message.id}::${r.message.timestamp}`;
        if (seenMessages.has(key)) return false;
        seenMessages.add(key);
        return true;
      })
      .slice(0, maxResults);

    // 2-5. Other sources (parallel)
    const promises: [
      ReturnType<typeof this.db.semanticSearchNotes>,
      ReturnType<typeof this.db.getRecentMessages>,
      Promise<string | null>,
      ReturnType<typeof this.dailyNotes.getRecentNotes>
    ] = [
      this.db.semanticSearchNotes(query, {
        limit: Math.ceil(maxResults * 0.5), // Allocate 50% to notes (imprints-first)
        threshold: 0.25, // Slightly lower threshold for notes
      }),
      this.db.getRecentMessages({
        limit: 20,
        project: undefined, // Don't filter recent messages by project (let reranker decide)
        since: sinceISO,
      }),
      githubUsername && this.github
        ? this.github.getUserStatus(githubUsername)
        : Promise.resolve(null),
      this.dailyNotes.getRecentNotes(lookbackDays),
    ];

    const [noteResults, recentMessages, githubStatus, dailyNotes] = await Promise.all(promises);

    // NEW: Cohere reranking - fuse all sources by relevance to query
    // Note: semanticResults already includes active_context (via pgvectorTool dual-source)
    // Cohere reranks: dual-source semantic + note embeddings (imprints-first) + daily notes + recent messages + GitHub
    let relevantContext: BrainBootResult['relevantContext'];
    let recentActivity: BrainBootResult['recentActivity'];

    if (this.reranker) {
      // Fuse all sources with Cohere reranking
      const rankedResults = await this.reranker.fuseMultiSource(
        query,
        {
          semanticResults: semanticResults.map((r) => ({
            content: r.message.content,
            metadata: {
              timestamp: r.message.timestamp,
              project: r.message.project,
              conversation: r.conversation?.title || r.conversation?.conv_id,
              similarity: r.similarity,
              source: r.source || 'semantic_search', // Use source field (active_context or embeddings)
            },
          })),
          noteEmbeddings: noteResults.map((r) => ({
            content: r.message.content,
            metadata: {
              note_path: r.conversation?.title || r.conversation?.conv_id,
              similarity: r.similarity,
              source: 'note_embeddings', // Imprints, daily notes, bridges
            },
          })),
          dailyNotes: dailyNotes.map((note) => ({
            content: note.content,
            metadata: {
              date: note.date,
              type: 'daily_note',
            },
          })),
          githubActivity: githubStatus
            ? [{ content: githubStatus, metadata: { source: 'github' } }]
            : [],
        },
        maxResults
      );

      // Map Cohere results to relevantContext format
      relevantContext = rankedResults.map((r) => ({
        content: r.document.text,
        timestamp: r.document.metadata?.timestamp || '',
        project: r.document.metadata?.project,
        conversation: r.document.metadata?.conversation,
        similarity: r.relevanceScore, // Cohere score (0-1)
      }));

      recentActivity = []; // Not needed - all sources fused into relevantContext
    } else {
      // Fallback: No Cohere reranking (original behavior)
      relevantContext = semanticResults.map((result) => ({
        content: result.message.content,
        timestamp: result.message.timestamp,
        project: result.message.project || undefined,
        conversation: result.conversation?.title || result.conversation?.conv_id,
        similarity: result.similarity,
      }));

      recentActivity = recentMessages.map((msg) => ({
        content: msg.content,
        timestamp: msg.timestamp,
        project: msg.project || undefined,
      }));
    }

    // Format daily notes (for summary display only if no Cohere)
    const dailyNotesSummary = this.dailyNotes.formatRecentNotes(dailyNotes);

    // Load full daily note if requested (boring thing: just read the file)
    let fullDailyNote: string | undefined;
    if (includeDailyNote) {
      try {
        const today = new Date().toISOString().split('T')[0]; // YYYY-MM-DD
        const fs = await import('fs/promises');
        const path = await import('path');
        const os = await import('os');
        const notePath = path.join(os.homedir(), '.evans-notes', 'daily', `${today}.md`);
        fullDailyNote = await fs.readFile(notePath, 'utf-8');
      } catch {
        // File doesn't exist or can't be read - skip silently
        fullDailyNote = undefined;
      }
    }

    // NOTE: active_context is now included in semanticResults via pgvectorTool (dual-source)
    // No need for separate activeContext formatting - it's merged into relevantContext

    // Generate summary
    const summary = this.generateSummary({
      query,
      relevantContext,
      recentActivity,
      project,
      lookbackDays,
      githubStatus,
      dailyNotes: dailyNotesSummary,
      fullDailyNote,
    });

    return {
      summary,
      relevantContext,
      recentActivity,
    };
  }

  /**
   * Smart truncation at sentence boundaries (from active_context_stream.ts)
   * @param content Content to truncate
   * @param maxLength Maximum length (default 400)
   * @returns Truncated content with clean boundaries
   */
  private smartTruncate(content: string, maxLength: number = 400): string {
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

    // Use sentence boundary if reasonably close to maxLength
    if (endPos > maxLength - 100) {
      return content.substring(0, endPos + 1).trim(); // +1 to include punctuation
    }

    // No good sentence boundary, try word boundary
    const wordBoundary = content.lastIndexOf(' ', maxLength);
    if (wordBoundary > maxLength - 50) {
      return content.substring(0, wordBoundary).trim() + '...';
    }

    // Fallback: hard truncate at maxLength
    return content.substring(0, maxLength).trim() + '...';
  }

  /**
   * Generate a human-readable summary
   */
  private generateSummary(context: {
    query: string;
    relevantContext: BrainBootResult['relevantContext'];
    recentActivity: BrainBootResult['recentActivity'];
    project?: string;
    lookbackDays: number;
    githubStatus?: string | null;
    dailyNotes?: string;
    fullDailyNote?: string; // Full daily note content (if includeDailyNote=true)
  }): string {
    const { query, relevantContext, recentActivity, project, lookbackDays, githubStatus, dailyNotes, fullDailyNote } = context;

    const lines: string[] = [];
    lines.push(`# Brain Boot: ${new Date().toLocaleDateString()}`);
    lines.push('');

    if (project) {
      lines.push(`**Project**: ${project}`);
      lines.push('');
    }

    lines.push(`**Query**: ${query}`);
    lines.push(`**Lookback**: Last ${lookbackDays} days`);
    lines.push('');

    // GitHub status (if available)
    if (githubStatus) {
      lines.push(githubStatus);
      lines.push('');
    }

    // Daily notes (if available)
    if (dailyNotes) {
      lines.push(dailyNotes);
      lines.push('');
    }

    // Full daily note (if requested)
    if (fullDailyNote) {
      lines.push(`## 📝 Daily Note (Full)`);
      lines.push('');
      lines.push(fullDailyNote);
      lines.push('');
      lines.push('---');
      lines.push('');
    }

    // NOTE: Active context now merged into Semantically Relevant Context via dual-source search
    // Look for similarity: 1.00 entries (those are from active_context_stream)

    lines.push(`## Semantically Relevant Context (${relevantContext.length} results)`);
    lines.push('');
    if (relevantContext.length === 0) {
      lines.push('*No relevant context found*');
    } else {
      relevantContext.slice(0, 5).forEach((ctx, idx) => {
        lines.push(`### ${idx + 1}. ${new Date(ctx.timestamp).toLocaleString()} (similarity: ${ctx.similarity.toFixed(2)})`);
        if (ctx.project) lines.push(`   **Project**: ${ctx.project}`);
        if (ctx.conversation) lines.push(`   **Conversation**: ${ctx.conversation}`);
        lines.push('');
        // Use smart truncation (400 chars, sentence-boundary aware)
        lines.push(`   ${this.smartTruncate(ctx.content)}`);
        lines.push('');
      });
    }

    lines.push(`## Recent Activity (${recentActivity.length} messages)`);
    lines.push('');
    if (recentActivity.length === 0) {
      lines.push('*No recent activity*');
    } else {
      recentActivity.slice(0, 10).forEach((activity) => {
        const timestamp = new Date(activity.timestamp).toLocaleString();
        const projectTag = activity.project ? ` [${activity.project}]` : '';
        lines.push(`- **${timestamp}**${projectTag}: ${activity.content.substring(0, 100)}...`);
      });
    }

    return lines.join('\n');
  }
}
