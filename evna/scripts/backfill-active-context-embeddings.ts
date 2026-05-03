#!/usr/bin/env tsx
/**
 * Backfill content_embedding for existing active_context_stream rows.
 *
 * Usage:
 *   tsx scripts/backfill-active-context-embeddings.ts              # full run
 *   tsx scripts/backfill-active-context-embeddings.ts --limit=500  # cap rows (testing)
 *   tsx scripts/backfill-active-context-embeddings.ts --dry-run    # count only
 *
 * Progress model: the DB column `content_embedding IS NULL` IS the source
 * of truth. A successful embedding writes the vector and the row naturally
 * drops out of the next query. No sidecar checkpoint file — earlier
 * attempt had one and a bug poisoned it by marking failures as processed.
 *
 * Within a single run, we track a `recentlyFailed` set so transient
 * errors on the same rows don't infinite-loop the process. Across runs,
 * those failed rows get retried from scratch (which is usually what you
 * want — transient errors should retry).
 *
 * Rows processed newest → oldest so recent queries benefit first.
 *
 * Env:
 *   SUPABASE_URL + SUPABASE_SERVICE_ROLE_KEY (preferred) or SUPABASE_ANON_KEY
 *   CLOUDFLARE_ACCOUNT_ID
 *   CLOUDFLARE_API_TOKEN (with Workers AI permission) — falls back to
 *     AUTORAG_API_TOKEN, but that typically only has AutoRAG scope and
 *     will return 401 on /ai/run/@cf/baai/bge-base-en-v1.5.
 */

import { createClient } from '@supabase/supabase-js';
import { loadEnvWithFallback } from '../src/lib/env-loader.js';
import { EmbeddingsClient } from '../src/lib/embeddings-client.js';

loadEnvWithFallback();

const SUPABASE_URL = process.env.SUPABASE_URL!;
const SUPABASE_KEY =
  process.env.SUPABASE_SERVICE_ROLE_KEY ?? process.env.SUPABASE_ANON_KEY!;

if (!SUPABASE_URL || !SUPABASE_KEY) {
  console.error('Missing SUPABASE_URL or SUPABASE_*_KEY');
  process.exit(1);
}

const supabase = createClient(SUPABASE_URL, SUPABASE_KEY);
const embeddings = EmbeddingsClient.fromEnv();
if (!embeddings) {
  console.error(
    'Embeddings provider not configured. Set EVNA_EMBEDDINGS_PROVIDER=ollama (default) ' +
    'with OLLAMA_URL, or =cloudflare with CLOUDFLARE_ACCOUNT_ID + ' +
    '(CLOUDFLARE_API_TOKEN || AUTORAG_API_TOKEN).'
  );
  process.exit(1);
}
console.log(`[backfill] Provider: ${embeddings.providerName}`);

const BATCH_SIZE = 50;
const SLEEP_BETWEEN_BATCHES_MS = 1000;
// If this many consecutive batches fail entirely (zero succeeded), bail
// instead of thrashing. Indicates an auth/config issue, not a transient one.
const MAX_CONSECUTIVE_EMPTY_BATCHES = 3;

function parseArgs(): { limit?: number; dryRun: boolean } {
  const argv = process.argv.slice(2);
  const dryRun = argv.includes('--dry-run');

  // Accept both `--limit=500` and `--limit 500`. A bare `--limit` with no
  // following value is treated as missing rather than silently becoming
  // "no cap" — that silent upgrade is how a 500-row smoke test turned
  // into a 9994-row full run.
  let limit: number | undefined;
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith('--limit=')) {
      const n = parseInt(a.split('=')[1], 10);
      if (!Number.isNaN(n) && n > 0) limit = n;
      break;
    }
    if (a === '--limit') {
      const next = argv[i + 1];
      const n = next ? parseInt(next, 10) : NaN;
      if (!Number.isNaN(n) && n > 0) {
        limit = n;
      } else {
        console.error(`--limit expects a positive integer (got: ${next ?? 'nothing'})`);
        process.exit(1);
      }
      break;
    }
  }
  return { limit, dryRun };
}

async function countMissing(): Promise<number> {
  const { count, error } = await supabase
    .from('active_context_stream')
    .select('message_id', { count: 'exact', head: true })
    .is('content_embedding', null);
  if (error) throw new Error(`count failed: ${error.message}`);
  return count ?? 0;
}

async function fetchBatch(
  failedThisRun: Set<string>
): Promise<Array<{ message_id: string; content: string }>> {
  // Overfetch because `failedThisRun` removes some in-JS. When it grows
  // large, page further back by timestamp via the 'lt' cursor. For the
  // initial backfill 150 is enough; for pathological cases with many
  // transient failures we'd need true pagination, but by then you should
  // investigate the auth/quota issue instead.
  const { data, error } = await supabase
    .from('active_context_stream')
    .select('message_id, content, timestamp')
    .is('content_embedding', null)
    .order('timestamp', { ascending: false })
    .limit(BATCH_SIZE * 3);

  if (error) throw new Error(`fetch failed: ${error.message}`);
  return (data ?? [])
    .filter((r) => !failedThisRun.has(r.message_id))
    .slice(0, BATCH_SIZE)
    .map(({ message_id, content }) => ({ message_id, content }));
}

/**
 * Embed + store a batch. Returns { succeeded, failedIds } so the caller
 * can distinguish "all failed, don't retry same rows" from "succeeded,
 * DB dropped them out of next query naturally."
 */
async function embedAndStore(
  batch: Array<{ message_id: string; content: string }>
): Promise<{ succeeded: number; failedIds: string[] }> {
  const texts = batch.map((r) => r.content);
  let vectors: number[][];
  try {
    vectors = await embeddings.embedBatch(texts);
  } catch (err) {
    console.error('  batch embed failed:', err instanceof Error ? err.message : err);
    // Whole-batch failure — mark all as failed-this-run so we skip them
    // on the next fetch this run.
    return { succeeded: 0, failedIds: batch.map((r) => r.message_id) };
  }

  let succeeded = 0;
  const failedIds: string[] = [];
  for (let i = 0; i < batch.length; i++) {
    const { error } = await supabase
      .from('active_context_stream')
      .update({ content_embedding: EmbeddingsClient.toPgvector(vectors[i]) })
      .eq('message_id', batch[i].message_id);
    if (error) {
      console.error(`  row update failed ${batch[i].message_id}:`, error.message);
      failedIds.push(batch[i].message_id);
    } else {
      succeeded++;
    }
  }
  return { succeeded, failedIds };
}

async function main() {
  const { limit: argLimit, dryRun } = parseArgs();
  const missing = await countMissing();
  console.log(`Rows needing embeddings: ${missing}`);

  if (dryRun) {
    console.log('Dry run — no API calls, no writes. Exiting.');
    return;
  }
  if (missing === 0) {
    console.log('Nothing to do.');
    return;
  }

  const hardStop = argLimit ?? Infinity;
  const failedThisRun = new Set<string>();
  let totalEmbedded = 0;
  let batchNum = 0;
  let consecutiveEmpty = 0;

  console.log(`Starting backfill. Target cap: ${hardStop === Infinity ? 'all' : hardStop}.`);

  while (totalEmbedded < hardStop) {
    const batch = await fetchBatch(failedThisRun);
    if (batch.length === 0) {
      console.log('No more fetchable rows (either all embedded or all failed this run).');
      break;
    }

    batchNum++;
    const t0 = Date.now();
    const { succeeded, failedIds } = await embedAndStore(batch);
    totalEmbedded += succeeded;
    for (const id of failedIds) failedThisRun.add(id);

    console.log(
      `  batch ${batchNum}: embedded ${succeeded}/${batch.length} in ${Date.now() - t0}ms ` +
        `(run total: ${totalEmbedded}, failed-this-run: ${failedThisRun.size})`
    );

    if (succeeded === 0) {
      consecutiveEmpty++;
      if (consecutiveEmpty >= MAX_CONSECUTIVE_EMPTY_BATCHES) {
        console.error(
          `  aborting — ${consecutiveEmpty} consecutive batches with zero successes. ` +
            `Check Cloudflare token (needs Workers AI permission), rate limits, or Supabase write auth.`
        );
        break;
      }
    } else {
      consecutiveEmpty = 0;
    }

    if (totalEmbedded >= hardStop) break;
    await new Promise((r) => setTimeout(r, SLEEP_BETWEEN_BATCHES_MS));
  }

  const remaining = await countMissing();
  console.log(
    `Done. Embedded this run: ${totalEmbedded}. Rows still NULL: ${remaining}. ` +
      `Skipped as failed-this-run: ${failedThisRun.size}.`
  );
}

main().catch((err) => {
  console.error('Fatal:', err);
  process.exit(1);
});
