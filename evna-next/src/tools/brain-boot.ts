/**
 * Brain Boot Tool
 * Rich context synthesis for morning check-ins and context restoration
 */

import { DatabaseClient } from '../lib/db.js';
import { EmbeddingsClient } from '../lib/embeddings.js';
import { GitHubClient } from '../lib/github.js';
import { DailyNotesReader } from '../lib/daily-notes.js';

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

  constructor(
    private db: DatabaseClient,
    private embeddings: EmbeddingsClient,
    githubRepo?: string,
    dailyNotesDir?: string
  ) {
    if (githubRepo) {
      this.github = new GitHubClient(githubRepo);
    }
    this.dailyNotes = new DailyNotesReader(dailyNotesDir);
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

    // Parallel fetch: semantic search + recent messages + GitHub status + daily notes
    // Note: semanticSearch now calls Rust CLI directly (no embedding needed)
    const promises: [
      ReturnType<typeof this.db.semanticSearch>,
      ReturnType<typeof this.db.getRecentMessages>,
      Promise<string | null>,
      ReturnType<typeof this.dailyNotes.getRecentNotes>
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
    ];

    const [semanticResults, recentMessages, githubStatus, dailyNotes] = await Promise.all(promises);

    // Build relevant context from semantic search
    const relevantContext = semanticResults.map((result) => ({
      content: result.message.content,
      timestamp: result.message.timestamp,
      project: result.message.project || undefined,
      conversation: result.conversation?.title || result.conversation?.conv_id,
      similarity: result.similarity,
    }));

    // Build recent activity
    const recentActivity = recentMessages.map((msg) => ({
      content: msg.content,
      timestamp: msg.timestamp,
      project: msg.project || undefined,
    }));

    // Format daily notes
    const dailyNotesSummary = this.dailyNotes.formatRecentNotes(dailyNotes);

    // Generate summary
    const summary = this.generateSummary({
      query,
      relevantContext,
      recentActivity,
      project,
      lookbackDays,
      githubStatus,
      dailyNotes: dailyNotesSummary,
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
  }): string {
    const { query, relevantContext, recentActivity, project, lookbackDays, githubStatus, dailyNotes } = context;

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
