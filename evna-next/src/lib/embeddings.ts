/**
 * OpenAI embeddings helper
 * Generates embeddings for semantic search
 */

import OpenAI from 'openai';

export class EmbeddingsClient {
  private openai: OpenAI;
  private model = 'text-embedding-3-small';

  constructor(apiKey: string) {
    this.openai = new OpenAI({ apiKey });
  }

  /**
   * Generate embedding for a single text
   */
  async embed(text: string): Promise<number[]> {
    const response = await this.openai.embeddings.create({
      model: this.model,
      input: text,
    });

    return response.data[0].embedding;
  }

  /**
   * Generate embeddings for multiple texts
   */
  async embedBatch(texts: string[]): Promise<number[][]> {
    const response = await this.openai.embeddings.create({
      model: this.model,
      input: texts,
    });

    // Sort by index to ensure correct order
    return response.data
      .sort((a, b) => a.index - b.index)
      .map((item) => item.embedding);
  }
}
