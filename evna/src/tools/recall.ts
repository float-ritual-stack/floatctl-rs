/**
 * Recall — search evna's memory
 *
 * Two-tier search: active_context_stream (recent, real-time) + AutoRAG
 * (historical, archived). Both sides return real cosine similarity
 * scores; the merge ranks by similarity, not by source.
 *
 * Design notes after 2026-04-20 rewrite:
 *
 * - Active_context_stream now stores Cloudflare Workers AI BGE embeddings
 *   per row (content_embedding vector(768)). The RPC
 *   match_active_context_embeddings returns cosine similarity, so flat
 *   0.9 is retired.
 * - Score merge sorts [active, autorag] by similarity DESC before
 *   dedup+slice. Active context no longer wins unconditionally.
 * - Temporal filters (before/after/on/between) apply to both sources:
 *   SQL WHERE on active_context, client-side post-filter on AutoRAG
 *   (Cloudflare's search API doesn't accept attribute filters).
 * - AutoRAG failures are logged warn and fall back to active_context-only
 *   results so the user still sees *something* when Cloudflare is flaky.
 */

import { DatabaseClient, SearchResult } from '../lib/db.js';
import { logger } from '../lib/logger.js';

export interface SearchOptions {
  query: string;
  limit?: number;
  project?: string;
  threshold?: number;
  /** Temporal filters — all optional. `on` is sugar for a whole-day window.
   *  `between` is inclusive on both ends in spec, exclusive on end in SQL
   *  (the timestamp at end-of-day is rare enough that this is fine). */
  before?: string;
  after?: string;
  on?: string;
  between?: [string, string];
  /** @deprecated use `after` */
  since?: string;
}

interface TimeWindow {
  start?: Date;
  end?: Date;
}

let sinceDeprecationLogged = false;

/**
 * Normalize the four temporal-filter fields into a single {start, end}
 * window.
 *
 * Precedence: `on` > `between` > (`after` + `before`) > `since`.
 * `since` fires a one-shot deprecation warning per process.
 *
 * Throws on `between=[X, Y]` with Y before X — silent swap would mask a
 * typo as a successful query.
 */
function parseTimeWindow(opts: SearchOptions): TimeWindow {
  if (opts.on) {
    const start = new Date(`${opts.on}T00:00:00Z`);
    if (isNaN(start.getTime())) {
      throw new Error(`recall: on="${opts.on}" is not a valid ISO date`);
    }
    const end = new Date(start);
    end.setUTCDate(end.getUTCDate() + 1);
    return { start, end };
  }

  if (opts.between) {
    const [s, e] = opts.between;
    const start = new Date(s);
    const end = new Date(e);
    if (isNaN(start.getTime()) || isNaN(end.getTime())) {
      throw new Error(`recall: between=[${s}, ${e}] contains an invalid ISO date`);
    }
    if (end < start) {
      throw new Error(`recall: between=[${s}, ${e}] has end before start`);
    }
    return { start, end };
  }

  const window: TimeWindow = {};
  if (opts.after) window.start = new Date(opts.after);
  if (opts.before) window.end = new Date(opts.before);

  if (!window.start && opts.since) {
    if (!sinceDeprecationLogged) {
      logger.error('recall', 'SearchOptions.since is deprecated; use `after` instead', {});
      sinceDeprecationLogged = true;
    }
    window.start = new Date(opts.since);
  }

  return window;
}

function hasExplicitTemporal(opts: SearchOptions): boolean {
  return !!(opts.before || opts.after || opts.on || opts.between);
}

export class RecallTool {
  constructor(private db: DatabaseClient) {}

  /**
   * Search evna's memory. Merges semantic active_context results with
   * AutoRAG historical results, ranked by real similarity score.
   */
  async search(options: SearchOptions): Promise<SearchResult[]> {
    const { query, limit = 10, project, threshold = 0.5 } = options;
    const window = parseTimeWindow(options);
    const explicitTemporal = hasExplicitTemporal(options);

    // Default 7-day lookback when no explicit after/since/on/between lower bound
    const effectiveStart = window.start ?? new Date(Date.now() - 7 * 24 * 60 * 60 * 1000);

    // Tier 1: active_context via semantic RPC (real cosine similarity)
    // Floor at 1 — used to be 3 which meant limit=3 calls got 0 AutoRAG.
    const activeLimit = Math.max(Math.floor(limit * 0.3), 1);
    const activeMessages = await this.db.queryActiveContext({
      limit: activeLimit,
      project,
      since: effectiveStart,
      until: window.end,
      query,
      threshold,
    });

    // Tier 2: AutoRAG historical. Overfetch when temporal filter is active
    // because we post-filter on modified_date and don't want the window to
    // empty the set.
    let autoragResults: SearchResult[] = [];
    const autoragFetchLimit = explicitTemporal ? limit * 4 : limit * 2;
    try {
      autoragResults = await this.db.semanticSearch(query, {
        limit: autoragFetchLimit,
        project,
        since: effectiveStart.toISOString(),
        threshold,
      });

      if (explicitTemporal) {
        const startMs = effectiveStart.getTime();
        const endMs = window.end?.getTime() ?? Number.POSITIVE_INFINITY;
        const preFilter = autoragResults.length;
        autoragResults = autoragResults.filter((r) => {
          const ts = new Date(r.message.timestamp).getTime();
          return ts >= startMs && ts < endMs;
        });
        // If the post-filter dropped more than half, signal that the
        // over-fetch multiplier may need tuning.
        if (preFilter > 0 && autoragResults.length < preFilter / 2) {
          logger.error('recall', 'AutoRAG post-filter dropped majority of results', {
            preFilter,
            postFilter: autoragResults.length,
            window: { start: effectiveStart.toISOString(), end: window.end?.toISOString() },
            hint: 'Consider widening the temporal window or increasing the overfetch multiplier.',
          });
        }
      }
    } catch (error) {
      logger.error('recall', 'AutoRAG search failed; returning active_context results only', {
        query,
        error: error instanceof Error ? error.message : String(error),
      });
      autoragResults = [];
    }

    // Convert active_context rows to SearchResult. Similarity comes from
    // the RPC (cosine score) — in the rare recency-fallback path it may
    // be missing; default to threshold so it doesn't falsely outrank
    // AutoRAG results.
    const activeResults: SearchResult[] = activeMessages.map((msg) => ({
      message: {
        id: msg.message_id,
        conversation_id: msg.conversation_id,
        idx: 0,
        role: msg.role,
        timestamp: msg.timestamp,
        content: msg.content,
        project: msg.metadata?.project || null,
        meeting: msg.metadata?.meeting || null,
        markers: [],
      },
      conversation: {
        id: msg.conversation_id,
        conv_id: msg.conversation_id,
        title: null,
        created_at: msg.timestamp,
        markers: [],
      },
      similarity: typeof msg.similarity === 'number' ? msg.similarity : threshold,
      source: 'active_context',
    }));

    // Rank by similarity across both sources. This is the core fix —
    // previous behavior was [active, autorag] concat, which pinned
    // active_context to the top N slots regardless of query match.
    const merged = [...activeResults, ...autoragResults].sort(
      (a, b) => b.similarity - a.similarity
    );

    // Dedup by composite key (conversation + timestamp + content prefix).
    // This handles the case where the same row persisted to permanent
    // storage arrives via both active_context and AutoRAG — keep the
    // higher-scoring instance (first one wins after the sort).
    const seen = new Set<string>();
    const deduplicated = merged.filter((result) => {
      const key = `${result.message.conversation_id}::${result.message.timestamp}::${result.message.content.substring(0, 50)}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });

    return deduplicated.slice(0, limit);
  }

  /**
   * Format recall results as markdown for MCP text response.
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

      const isActiveContext = source === 'active_context';
      const sourceTag = isActiveContext ? ' 🔴 Recent' : '';
      const conversationTitle =
        conversation?.title ||
        (isActiveContext ? 'Active Context' : conversation?.conv_id || 'Unknown');

      lines.push(
        `## ${idx + 1}. ${timestamp}${projectTag}${sourceTag} (similarity: ${similarity.toFixed(2)})`
      );
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
