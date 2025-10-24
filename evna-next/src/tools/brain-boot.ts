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

export interface BrainBootOptions {
  query: string;
  project?: string;
  lookbackDays?: number;
  maxResults?: number;
  githubRepo?: string;
  githubUsername?: string;
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

  constructor(
    private db: DatabaseClient,
    private embeddings: EmbeddingsClient,
    githubRepo?: string,
    dailyNotesDir?: string,
    cohereApiKey?: string // NEW: Optional Cohere API key for reranking
  ) {
    if (githubRepo) {
      this.github = new GitHubClient(githubRepo);
    }
    if (cohereApiKey) {
      this.reranker = new CohereReranker(cohereApiKey);
    }
    this.dailyNotes = new DailyNotesReader(dailyNotesDir);
    this.activeContext = new ActiveContextStream(db);
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
    } = options;

    // Calculate since timestamp
    const since = new Date();
    since.setDate(since.getDate() - lookbackDays);
    const sinceISO = since.toISOString();

    // Parallel fetch: semantic search + recent messages + GitHub status + daily notes + active context
    // Note: semanticSearch now calls Rust CLI directly (no embedding needed)
    const promises: [
      ReturnType<typeof this.db.semanticSearch>,
      ReturnType<typeof this.db.getRecentMessages>,
      Promise<string | null>,
      ReturnType<typeof this.dailyNotes.getRecentNotes>,
      ReturnType<typeof this.activeContext.queryContext>
    ] = [
      this.db.semanticSearch(query, {
        limit: maxResults,
        project,
        since: sinceISO,
        threshold: 0.3, // Lower threshold for more results
      }),
      this.db.getRecentMessages({
        limit: 20,
        project,
        since: sinceISO,
      }),
      githubUsername && this.github
        ? this.github.getUserStatus(githubUsername)
        : Promise.resolve(null),
      this.dailyNotes.getRecentNotes(lookbackDays),
      this.activeContext.queryContext({
        limit: 10,
        project,
        since: new Date(sinceISO),
      }),
    ];

    const [semanticResults, recentMessages, githubStatus, dailyNotes, activeContextMessages] = await Promise.all(promises);

    // NEW: Cohere reranking - fuse all 5 sources by relevance to query
    // Why: Semantic search returns embeddings-only, active context separate, daily notes raw
    // Cohere cross-encoder scores ALL sources by query relevance → single ranked list
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
            },
          })),
          activeContext: activeContextMessages.map((m) => ({
            content: m.content,
            metadata: {
              timestamp: m.timestamp,
              project: m.metadata.project,
              client_type: m.client_type,
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

    // Format active context (for summary display only if no Cohere)
    const activeContextSummary = this.activeContext.formatContext(activeContextMessages);

    // Generate summary
    const summary = this.generateSummary({
      query,
      relevantContext,
      recentActivity,
      project,
      lookbackDays,
      githubStatus,
      dailyNotes: dailyNotesSummary,
      activeContext: activeContextSummary,
    });

    return {
      summary,
      relevantContext,
      recentActivity,
    };
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
    activeContext?: string;
  }): string {
    const { query, relevantContext, recentActivity, project, lookbackDays, githubStatus, dailyNotes, activeContext } = context;

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

    // Active context (live annotations)
    if (activeContext) {
      lines.push(activeContext);
      lines.push('');
    }

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
        lines.push(`   ${ctx.content.substring(0, 200)}${ctx.content.length > 200 ? '...' : ''}`);
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
