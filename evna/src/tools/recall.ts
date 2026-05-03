/**
 * Recall — search evna's memory
 *
 * Two-track presentation: active_context_stream as Working Memory tier,
 * AutoRAG as Corpus tier. Each tier sorts by similarity within itself —
 * the merge step from the Apr 20 fix is gone because flat-similarity
 * merge drowned working memory under a 6× larger corpus.
 *
 * R3 primitive (`recent` option): bypass dual-source flow entirely and
 * return the last N active_context captures for a project. Use case:
 * "I haven't touched X in months, pick up the thread."
 *
 * Recency floor: when the semantic RPC returns fewer rows than the
 * working-memory budget (threshold filter dropping everything), follow
 * up with a recency-only query so working memory always presents when
 * captures exist in the window.
 *
 * Cross-client age annotation: every result gets a relative age band
 * ("today" / "N days ago" / "N months ago") so agents reading results
 * acknowledge with the right tense — old siblings don't get pattern-
 * matched as fresh-and-urgent.
 *
 * Apr 20 work that stays: BGE embeddings on every active_context row,
 * temporal filters (on/after/before/between), AutoRAG try/catch fallback,
 * project canonicalization at the RPC layer.
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
  /** R3 resume primitive — when set, similarity search is bypassed and
   *  the last N active_context captures (project-scoped if `project` is
   *  given) are returned in timestamp-DESC order. Use `query` OR `recent`,
   *  not both — `recent` takes precedence. */
  recent?: number;
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

/**
 * Convert an active_context row to the SearchResult shape used by the
 * recall surface. Similarity comes from the semantic RPC; recency-only
 * rows have no score and fall back to `threshold` (the existing
 * convention — separate orthogonal pattern fix tracked elsewhere).
 */
function activeRowToResult(
  msg: {
    message_id: string;
    conversation_id: string;
    role: string;
    content: string;
    timestamp: string;
    metadata: Record<string, any>;
    similarity?: number;
  },
  threshold: number,
): SearchResult {
  return {
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
  };
}

function dedupTier(tier: SearchResult[]): SearchResult[] {
  const seen = new Set<string>();
  return tier.filter((result) => {
    const key = `${result.message.conversation_id}::${result.message.timestamp}::${result.message.content.substring(0, 50)}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

/**
 * Compute a relative age band for display. Bands:
 *   today / yesterday / N days ago / N weeks ago / N months ago / N years ago
 * Uses UTC calendar dates for today/yesterday so cross-timezone clients
 * agree on what "today" means.
 */
function ageBand(timestamp: string, queryTime: Date = new Date()): string {
  const ts = new Date(timestamp);
  const tsDate = ts.toISOString().slice(0, 10);
  const queryDate = queryTime.toISOString().slice(0, 10);
  if (tsDate === queryDate) return 'today';

  const yesterday = new Date(queryTime);
  yesterday.setUTCDate(yesterday.getUTCDate() - 1);
  if (tsDate === yesterday.toISOString().slice(0, 10)) return 'yesterday';

  const diffDays = (queryTime.getTime() - ts.getTime()) / (1000 * 60 * 60 * 24);
  if (diffDays < 0) return 'today'; // future timestamp (clock skew) — treat as recent
  if (diffDays < 7) return `${Math.floor(diffDays)} days ago`;
  if (diffDays < 30) return `${Math.floor(diffDays / 7)} weeks ago`;
  if (diffDays < 365) return `${Math.floor(diffDays / 30)} months ago`;
  return `${Math.floor(diffDays / 365)} years ago`;
}

export class RecallTool {
  constructor(private db: DatabaseClient) {}

  /**
   * Search evna's memory. Two-track shape: working memory (active_context)
   * and corpus (AutoRAG), each ranked by similarity within tier. Working
   * memory has a recency floor so recent captures surface even when no
   * row clears the similarity threshold.
   *
   * R3 short-circuit: if `options.recent` is set, similarity search is
   * skipped entirely and the last N active_context captures are returned.
   */
  async search(options: SearchOptions): Promise<SearchResult[]> {
    const { query, limit = 10, project, threshold = 0.5 } = options;

    // R3 — project-scoped resume primitive. Bypasses dual-source merge.
    if (typeof options.recent === 'number' && options.recent > 0) {
      if (!project) {
        logger.error('recall', 'recent= without project — falling back to global last-N', {
          recent: options.recent,
        });
      }
      const rows = await this.db.queryActiveContext({
        limit: options.recent,
        project,
        // intentionally no query/since/until — pure recency, ORDER BY timestamp DESC
      });
      return rows.map((msg) => activeRowToResult(msg, threshold));
    }

    const window = parseTimeWindow(options);
    const explicitTemporal = hasExplicitTemporal(options);

    // Default 7-day lookback when no explicit after/since/on/between lower bound
    const effectiveStart = window.start ?? new Date(Date.now() - 7 * 24 * 60 * 60 * 1000);

    // Working memory budget — 30/70 partition kept from prior shape.
    // limit=10 → 3 active + 7 corpus. limit=3 → 1 active + 2 corpus.
    const activeLimit = Math.max(Math.floor(limit * 0.3), 1);
    const corpusLimit = Math.max(limit - activeLimit, 0);

    // Tier 1: active_context via semantic RPC (real cosine similarity)
    const activeMessages = await this.db.queryActiveContext({
      limit: activeLimit,
      project,
      since: effectiveStart,
      until: window.end,
      query,
      threshold,
    });

    let workingMemory: SearchResult[] = activeMessages.map((msg) =>
      activeRowToResult(msg, threshold),
    );

    // Recency floor: if the semantic threshold dropped everything below
    // the working-memory budget, follow up with recency-only so the tier
    // never goes empty when captures exist in the window. Inherits the
    // same project + window filters.
    if (workingMemory.length < activeLimit) {
      const needed = activeLimit - workingMemory.length;
      try {
        const recencyRows = await this.db.queryActiveContext({
          limit: needed * 2, // overfetch for dedup buffer
          project,
          since: effectiveStart,
          until: window.end,
          // intentionally no `query` — forces recency mode in db.ts
        });
        const seenIds = new Set(workingMemory.map((r) => r.message.id));
        const additional = recencyRows
          .filter((msg) => !seenIds.has(msg.message_id))
          .map((msg) => activeRowToResult(msg, threshold))
          .slice(0, needed);
        workingMemory = [...workingMemory, ...additional];
      } catch (error) {
        // Recency floor is a nicety, not load-bearing. Log + continue.
        logger.error('recall', 'Recency floor follow-up failed; semantic-only working memory', {
          error: error instanceof Error ? error.message : String(error),
        });
      }
    }

    // Tier 2: AutoRAG historical. Overfetch when temporal filter is active
    // because we post-filter on modified_date and don't want the window to
    // empty the set.
    let corpus: SearchResult[] = [];
    const autoragFetchLimit = explicitTemporal ? limit * 4 : limit * 2;
    try {
      corpus = await this.db.semanticSearch(query, {
        limit: autoragFetchLimit,
        project,
        since: effectiveStart.toISOString(),
        threshold,
      });

      if (explicitTemporal) {
        const startMs = effectiveStart.getTime();
        const endMs = window.end?.getTime() ?? Number.POSITIVE_INFINITY;
        const preFilter = corpus.length;
        corpus = corpus.filter((r) => {
          const ts = new Date(r.message.timestamp).getTime();
          return ts >= startMs && ts < endMs;
        });
        if (preFilter > 0 && corpus.length < preFilter / 2) {
          logger.error('recall', 'AutoRAG post-filter dropped majority of results', {
            preFilter,
            postFilter: corpus.length,
            window: { start: effectiveStart.toISOString(), end: window.end?.toISOString() },
            hint: 'Consider widening the temporal window or increasing the overfetch multiplier.',
          });
        }
      }
    } catch (error) {
      logger.error('recall', 'AutoRAG search failed; returning working-memory only', {
        query,
        error: error instanceof Error ? error.message : String(error),
      });
      corpus = [];
    }

    // Sort within each tier by similarity DESC, dedup per-tier, slice to budget.
    const sortedWorking = [...workingMemory].sort((a, b) => b.similarity - a.similarity);
    const sortedCorpus = [...corpus].sort((a, b) => b.similarity - a.similarity);
    const dedupedWorking = dedupTier(sortedWorking).slice(0, activeLimit);
    const dedupedCorpus = dedupTier(sortedCorpus).slice(0, corpusLimit);

    return [...dedupedWorking, ...dedupedCorpus];
  }

  /**
   * Format recall results as markdown for MCP text response. Two
   * sections: Working Memory (active_context) and Corpus (AutoRAG),
   * each with its own header. Every result carries a relative-age
   * annotation so cross-client siblings aren't pattern-matched as
   * fresh-and-urgent when they're months old.
   */
  formatResults(results: SearchResult[]): string {
    if (results.length === 0) {
      return '**No results found**';
    }

    const working = results.filter((r) => r.source === 'active_context');
    const corpus = results.filter((r) => r.source !== 'active_context');

    const lines: string[] = [];
    const queryTime = new Date();

    if (working.length > 0) {
      lines.push(`# 🔴 Working Memory (${working.length} recent ${working.length === 1 ? 'capture' : 'captures'})\n`);
      working.forEach((result, idx) => {
        lines.push(...this.renderResult(result, idx, queryTime));
      });
    }

    if (corpus.length > 0) {
      if (working.length > 0) lines.push('');
      lines.push(`# 📚 Corpus (${corpus.length} also relevant)\n`);
      corpus.forEach((result, idx) => {
        lines.push(...this.renderResult(result, idx, queryTime));
      });
    }

    return lines.join('\n');
  }

  private renderResult(result: SearchResult, idx: number, queryTime: Date): string[] {
    const { message, conversation, similarity, source } = result;
    const timestamp = new Date(message.timestamp).toLocaleString();
    const projectTag = message.project ? ` [${message.project}]` : '';
    const age = ageBand(message.timestamp, queryTime);

    const isActiveContext = source === 'active_context';
    const conversationTitle =
      conversation?.title ||
      (isActiveContext ? 'Active Context' : conversation?.conv_id || 'Unknown');

    return [
      `## ${idx + 1}. ${timestamp} (${age})${projectTag} (similarity: ${similarity.toFixed(2)})`,
      `**Conversation**: ${conversationTitle}`,
      `**Role**: ${message.role}`,
      '',
      message.content,
      '',
      '---',
      '',
    ];
  }
}
