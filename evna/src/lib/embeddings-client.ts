/**
 * Embeddings Client — provider-pluggable
 *
 * Two providers, same 768-dim vector space conceptually but DIFFERENT
 * vector spaces in practice (different models can't be mixed in cosine
 * search). All rows in active_context_stream must use the SAME provider
 * — switching providers requires a full re-embed.
 *
 * Providers:
 *   - cloudflare: @cf/baai/bge-base-en-v1.5 via Workers AI (768-dim)
 *   - ollama: nomic-embed-text via local Ollama (768-dim)
 *
 * Default: ollama (local, free, no egress). Set EVNA_EMBEDDINGS_PROVIDER
 * to "cloudflare" to override.
 *
 * Migration note (2026-05-03): switched default from cloudflare → ollama
 * after the May 3 trigger-bus work made embeddings load-bearing for
 * working-memory retrieval. Local Ollama on float-box was sitting idle;
 * latency dropped 3-5x and zero egress on captures.
 */

const CLOUDFLARE_MODEL = '@cf/baai/bge-base-en-v1.5';
const OLLAMA_MODEL = 'nomic-embed-text';
const EMBEDDING_DIM = 768;
const CLOUDFLARE_MAX_BATCH_SIZE = 100;

export type EmbeddingsProvider = 'cloudflare' | 'ollama';

interface CloudflareAIResponse {
  result?: {
    shape: number[];
    data: number[][];
  };
  success: boolean;
  errors: Array<{ code: number; message: string }>;
  messages: unknown[];
}

interface OllamaEmbeddingsResponse {
  embedding: number[];
}

export interface EmbeddingsClientOpts {
  provider: EmbeddingsProvider;
  /** Cloudflare provider only */
  accountId?: string;
  /** Cloudflare provider only */
  apiToken?: string;
  /** Ollama provider only — defaults to http://localhost:11434 */
  ollamaUrl?: string;
}

export class EmbeddingsClient {
  private readonly provider: EmbeddingsProvider;
  private readonly endpoint: string;
  private readonly apiToken?: string;
  private readonly model: string;

  constructor(opts: EmbeddingsClientOpts) {
    this.provider = opts.provider;
    if (opts.provider === 'cloudflare') {
      if (!opts.accountId) throw new Error('EmbeddingsClient(cloudflare): accountId is required');
      if (!opts.apiToken) throw new Error('EmbeddingsClient(cloudflare): apiToken is required');
      this.endpoint = `https://api.cloudflare.com/client/v4/accounts/${opts.accountId}/ai/run/${CLOUDFLARE_MODEL}`;
      this.apiToken = opts.apiToken;
      this.model = CLOUDFLARE_MODEL;
    } else if (opts.provider === 'ollama') {
      const baseUrl = opts.ollamaUrl ?? 'http://localhost:11434';
      this.endpoint = `${baseUrl}/api/embeddings`;
      this.model = OLLAMA_MODEL;
    } else {
      throw new Error(`EmbeddingsClient: unknown provider "${opts.provider}"`);
    }
  }

  /** Returns the embedding dimension. Same across providers (768). */
  static get dim(): number {
    return EMBEDDING_DIM;
  }

  /** Which provider this instance uses. */
  get providerName(): EmbeddingsProvider {
    return this.provider;
  }

  /** Embed a single text. Returns a 768-dim vector. */
  async embed(text: string): Promise<number[]> {
    if (this.provider === 'ollama') {
      return this.embedOllama(text);
    }
    const [vec] = await this.embedBatch([text]);
    return vec;
  }

  /**
   * Embed up to CLOUDFLARE_MAX_BATCH_SIZE texts in one call. Caller is
   * responsible for chunking larger batches (backfill script handles this).
   *
   * For the Ollama provider, the API is per-prompt — we loop the batch
   * sequentially. Local network, no rate limits, latency dominated by
   * GPU inference (~50-150ms per item).
   *
   * Throws on non-200 / success:false — caller decides fallback.
   */
  async embedBatch(texts: string[]): Promise<number[][]> {
    if (texts.length === 0) return [];

    if (this.provider === 'ollama') {
      const results: number[][] = [];
      for (const text of texts) {
        results.push(await this.embedOllama(text));
      }
      return results;
    }

    // Cloudflare provider
    if (texts.length > CLOUDFLARE_MAX_BATCH_SIZE) {
      throw new Error(
        `EmbeddingsClient(cloudflare): batch size ${texts.length} exceeds MAX_BATCH_SIZE ${CLOUDFLARE_MAX_BATCH_SIZE}. ` +
        `Chunk the input at call site.`
      );
    }

    const response = await fetch(this.endpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiToken!}`,
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

  private async embedOllama(text: string): Promise<number[]> {
    const response = await fetch(this.endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ model: this.model, prompt: text }),
    });

    if (!response.ok) {
      const body = await response.text();
      throw new Error(
        `Ollama embeddings ${response.status} ${response.statusText}: ${body.slice(0, 300)}`
      );
    }

    const json = await response.json() as OllamaEmbeddingsResponse;
    if (!Array.isArray(json.embedding)) {
      throw new Error('Ollama embeddings: response missing `embedding` array');
    }
    if (json.embedding.length !== EMBEDDING_DIM) {
      throw new Error(
        `Ollama embeddings returned dim ${json.embedding.length}, expected ${EMBEDDING_DIM} ` +
        `(model "${this.model}" may have wrong size — pull a 768-dim model)`
      );
    }
    return json.embedding;
  }

  /**
   * Embed with exponential backoff retry. 429/5xx are retried; 4xx (other)
   * are terminal. Retries are transparent to the caller; final failure throws.
   */
  async embedWithRetry(text: string, maxAttempts = 3): Promise<number[]> {
    let lastError: Error | undefined;
    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      try {
        return await this.embed(text);
      } catch (err) {
        lastError = err instanceof Error ? err : new Error(String(err));
        const isRetryable = /\b(429|5\d\d|ECONNREFUSED|ECONNRESET|ETIMEDOUT)\b/.test(lastError.message);
        if (!isRetryable || attempt === maxAttempts) throw lastError;
        const backoffMs = 2 ** (attempt - 1) * 500; // 500ms, 1s, 2s
        await new Promise(r => setTimeout(r, backoffMs));
      }
    }
    throw lastError ?? new Error('embedWithRetry: unknown failure');
  }

  /**
   * Serialize a vector to pgvector's text representation: "[0.1,0.2,...]"
   * Used when parameterizing SQL INSERT/UPDATE against a vector(768) column.
   */
  static toPgvector(vec: number[]): string {
    return `[${vec.join(',')}]`;
  }

  /**
   * Construct an EmbeddingsClient from environment variables.
   * - EVNA_EMBEDDINGS_PROVIDER=ollama|cloudflare (default: ollama)
   * - For ollama: OLLAMA_URL (default http://localhost:11434)
   * - For cloudflare: CLOUDFLARE_ACCOUNT_ID + (CLOUDFLARE_API_TOKEN || AUTORAG_API_TOKEN)
   *
   * Returns null if the chosen provider's credentials are missing.
   */
  static fromEnv(): EmbeddingsClient | null {
    const provider = (process.env.EVNA_EMBEDDINGS_PROVIDER ?? 'ollama') as EmbeddingsProvider;

    if (provider === 'ollama') {
      return new EmbeddingsClient({
        provider: 'ollama',
        ollamaUrl: process.env.OLLAMA_URL,
      });
    }

    if (provider === 'cloudflare') {
      const accountId = process.env.CLOUDFLARE_ACCOUNT_ID;
      const apiToken = process.env.CLOUDFLARE_API_TOKEN ?? process.env.AUTORAG_API_TOKEN;
      if (!accountId || !apiToken) return null;
      return new EmbeddingsClient({ provider: 'cloudflare', accountId, apiToken });
    }

    throw new Error(`Unknown EVNA_EMBEDDINGS_PROVIDER: "${provider}" (expected "ollama" or "cloudflare")`);
  }
}
