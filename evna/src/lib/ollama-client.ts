/**
 * Ollama Client for local LLM inference
 * Used for cost-free background bridge analysis and maintenance
 */

export interface OllamaGenerateRequest {
  model: string;
  prompt: string;
  system?: string;
  temperature?: number;
  stream?: boolean;
}

export interface OllamaGenerateResponse {
  model: string;
  created_at: string;
  response: string;
  done: boolean;
  context?: number[];
  total_duration?: number;
  load_duration?: number;
  prompt_eval_count?: number;
  eval_count?: number;
  eval_duration?: number;
}

export interface OllamaEmbeddingsRequest {
  model: string;
  prompt: string;
}

export interface OllamaEmbeddingsResponse {
  embedding: number[];
}

export class OllamaClient {
  private baseUrl: string;

  constructor(baseUrl: string = "http://localhost:11434") {
    this.baseUrl = baseUrl;
  }

  /**
   * Generate text completion using Ollama
   */
  async generate(request: OllamaGenerateRequest): Promise<string> {
    const response = await fetch(`${this.baseUrl}/api/generate`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: request.model,
        prompt: request.prompt,
        system: request.system,
        temperature: request.temperature ?? 0.7,
        stream: false,
      }),
    });

    if (!response.ok) {
      throw new Error(`Ollama generate failed: ${response.statusText}`);
    }

    const data = await response.json() as OllamaGenerateResponse;
    return data.response;
  }

  /**
   * Generate embeddings using Ollama
   */
  async embeddings(request: OllamaEmbeddingsRequest): Promise<number[]> {
    const response = await fetch(`${this.baseUrl}/api/embeddings`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: request.model,
        prompt: request.prompt,
      }),
    });

    if (!response.ok) {
      throw new Error(`Ollama embeddings failed: ${response.statusText}`);
    }

    const data = await response.json() as OllamaEmbeddingsResponse;
    return data.embedding;
  }

  /**
   * Check if Ollama is running and model is available
   */
  async checkHealth(model?: string): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/api/tags`);
      if (!response.ok) return false;

      if (model) {
        const data = await response.json() as { models?: Array<{ name: string }> };
        return data.models?.some((m) => m.name === model) ?? false;
      }

      return true;
    } catch {
      return false;
    }
  }

  /**
   * List available models
   */
  async listModels(): Promise<string[]> {
    const response = await fetch(`${this.baseUrl}/api/tags`);
    if (!response.ok) {
      throw new Error(`Ollama list models failed: ${response.statusText}`);
    }

    const data = await response.json() as { models?: Array<{ name: string }> };
    return data.models?.map((m) => m.name) ?? [];
  }
}

/**
 * Default Ollama client instance
 */
export const ollama = new OllamaClient();

/**
 * Recommended models for different tasks
 */
export const OLLAMA_MODELS = {
  // Fast categorization and tagging
  fast: "llama3.2:3b",
  
  // Balanced analysis and summarization
  balanced: "qwen2.5:7b",
  
  // Deep analysis and content generation
  deep: "qwen2.5:14b",
  
  // Embeddings for similarity detection
  embeddings: "nomic-embed-text",
} as const;
