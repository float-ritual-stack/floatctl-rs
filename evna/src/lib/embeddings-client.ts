/**
 * Cloudflare Workers AI Embeddings Client
 * Generates BGE embeddings for active_context_stream content via the same
 * Cloudflare account that powers AutoRAG — one credential set, shared retry
 * logic, score range compatible with AutoRAG's BGE reranker.
 *
 * Model: @cf/baai/bge-base-en-v1.5 (768-dim, cosine similarity)
 */

const MODEL = '@cf/baai/bge-base-en-v1.5';
const EMBEDDING_DIM = 768;
const MAX_BATCH_SIZE = 100;

interface CloudflareAIResponse {
  result?: {
    shape: number[];
    data: number[][];
  };
  success: boolean;
  errors: Array<{ code: number; message: string }>;
  messages: unknown[];
}

export class EmbeddingsClient {
  private readonly accountId: string;
  private readonly apiToken: string;
  private readonly endpoint: string;

  constructor(accountId: string, apiToken: string) {
    if (!accountId) throw new Error('EmbeddingsClient: accountId is required');
    if (!apiToken) throw new Error('EmbeddingsClient: apiToken is required');
    this.accountId = accountId;
    this.apiToken = apiToken;
    this.endpoint = `https://api.cloudflare.com/client/v4/accounts/${accountId}/ai/run/${MODEL}`;
  }

  /** Returns the embedding dimension of this model. */
  static get dim(): number {
    return EMBEDDING_DIM;
  }

  /** Embed a single text. Returns a 768-dim vector. */
  async embed(text: string): Promise<number[]> {
    const [vec] = await this.embedBatch([text]);
    return vec;
  }

  /**
   * Embed up to MAX_BATCH_SIZE texts in one call. Caller is responsible
   * for chunking larger batches (backfill script handles this).
   *
   * Throws on non-200 / success:false — caller decides fallback.
   */
  async embedBatch(texts: string[]): Promise<number[][]> {
    if (texts.length === 0) return [];
    if (texts.length > MAX_BATCH_SIZE) {
      throw new Error(
        `EmbeddingsClient: batch size ${texts.length} exceeds MAX_BATCH_SIZE ${MAX_BATCH_SIZE}. ` +
        `Chunk the input at call site.`
      );
    }

    const response = await fetch(this.endpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiToken}`,
      },
      body: JSON.stringify({ text: texts }),
    });

    if (!response.ok) {
      const body = await response.text();
      throw new Error(
        `Cloudflare embeddings ${response.status} ${response.statusText}: ${body.slice(0, 500)}`
      );
    }

    const json = await response.json() as CloudflareAIResponse;
    if (!json.success || !json.result) {
      const errMsg = json.errors?.map(e => `${e.code}: ${e.message}`).join('; ') || 'unknown error';
      throw new Error(`Cloudflare embeddings failed: ${errMsg}`);
    }

    const { data, shape } = json.result;
    if (!Array.isArray(data) || data.length !== texts.length) {
      throw new Error(
        `Cloudflare embeddings returned ${data?.length ?? 'null'} vectors for ${texts.length} texts`
      );
    }
    if (shape?.[1] !== EMBEDDING_DIM) {
      throw new Error(
        `Cloudflare embeddings returned dim ${shape?.[1]}, expected ${EMBEDDING_DIM}`
      );
    }

    return data;
  }

  /**
   * Embed with exponential backoff retry. Shape follows retryAutoRAG in db.ts
   * — 429/5xx are retried; 4xx (other) are terminal. Retries are transparent
   * to the caller; final failure throws.
   */
  async embedWithRetry(text: string, maxAttempts = 3): Promise<number[]> {
    let lastError: Error | undefined;
    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      try {
        return await this.embed(text);
      } catch (err) {
        lastError = err instanceof Error ? err : new Error(String(err));
        const isRetryable = /\b(429|5\d\d)\b/.test(lastError.message);
        if (!isRetryable || attempt === maxAttempts) throw lastError;
        const backoffMs = 2 ** (attempt - 1) * 500; // 500ms, 1s, 2s
        await new Promise(r => setTimeout(r, backoffMs));
      }
    }
    // Unreachable — loop either returns or throws
    throw lastError ?? new Error('embedWithRetry: unknown failure');
  }

  /**
   * Serialize a vector to pgvector's text representation: "[0.1,0.2,...]"
   * Used when parameterizing SQL INSERT/UPDATE against a vector(768) column.
   */
  static toPgvector(vec: number[]): string {
    return `[${vec.join(',')}]`;
  }
}
