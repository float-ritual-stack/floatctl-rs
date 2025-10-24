/**
 * Cohere Reranking Client
 *
 * Why Cohere? Cross-encoder reranking beats cosine similarity for multi-source fusion.
 * Cohere's rerank-english-v3.0 scores query relevance across heterogeneous sources
 * (semantic search + active context + daily notes + GitHub) better than vector similarity alone.
 *
 * Hermit crab note: Stealing pattern from BRAIN-BOOT-SYNTHESIS-UPGRADE.md spec,
 * trimmed to 80 lines for shack vibes (not cathedral).
 */

import { CohereClient } from 'cohere-ai';

export interface RerankDocument {
  text: string;
  metadata?: Record<string, any>;
}

export interface RerankResult {
  index: number;
  relevanceScore: number;
  document: RerankDocument;
}

export class CohereReranker {
  private client: CohereClient;

  constructor(apiKey: string) {
    this.client = new CohereClient({ token: apiKey });
  }

  /**
   * Rerank documents by relevance to query
   * Returns top N documents with relevance scores (0-1, higher = more relevant)
   */
  async rerank(
    query: string,
    documents: RerankDocument[],
    topN: number = 10
  ): Promise<RerankResult[]> {
    if (documents.length === 0) return [];

    // Cohere expects array of strings (not objects)
    const texts = documents.map((doc) => doc.text);

    const response = await this.client.rerank({
      query,
      documents: texts,
      topN: Math.min(topN, documents.length),
      model: 'rerank-english-v3.0', // Latest Cohere rerank model (Jan 2024)
    });

    // Map back to original documents with scores
    return response.results.map((result) => ({
      index: result.index,
      relevanceScore: result.relevanceScore,
      document: documents[result.index],
    }));
  }

  /**
   * Fuse multiple result sources with reranking
   * Combines semantic search, active context, daily notes, GitHub into single ranked list
   *
   * Why this matters: brain_boot fetches 5 parallel sources but shows them as separate
   * sections. Cohere reranking fuses them into one relevance-sorted list.
   */
  async fuseMultiSource(
    query: string,
    sources: {
      semanticResults?: Array<{ content: string; metadata: any }>;
      activeContext?: Array<{ content: string; metadata: any }>;
      dailyNotes?: Array<{ content: string; metadata: any }>;
      githubActivity?: Array<{ content: string; metadata: any }>;
    },
    topN: number = 10
  ): Promise<RerankResult[]> {
    // Flatten all sources into single document array with source metadata
    const allDocuments: RerankDocument[] = [
      ...(sources.semanticResults || []).map((r) => ({
        text: r.content,
        metadata: { ...r.metadata, source: 'semantic_search' },
      })),
      ...(sources.activeContext || []).map((r) => ({
        text: r.content,
        metadata: { ...r.metadata, source: 'active_context' },
      })),
      ...(sources.dailyNotes || []).map((r) => ({
        text: r.content,
        metadata: { ...r.metadata, source: 'daily_notes' },
      })),
      ...(sources.githubActivity || []).map((r) => ({
        text: r.content,
        metadata: { ...r.metadata, source: 'github' },
      })),
    ];

    if (allDocuments.length === 0) return [];

    // Rerank across ALL sources by relevance to query
    return this.rerank(query, allDocuments, topN);
  }
}
