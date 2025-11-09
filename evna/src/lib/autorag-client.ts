/**
 * Cloudflare AutoRAG (AI Search) Client
 * Direct REST API integration for historical knowledge search
 */

export interface AutoRAGSearchOptions {
  query: string;
  rag_id?: string;              // Default: sysops-beta
  max_results?: number;         // Default: 10
  rewrite_query?: boolean;      // Default: true (improves retrieval)
  score_threshold?: number;     // Default: 0.3
  enable_reranking?: boolean;   // Default: true
  folder_filter?: string;       // Filter by folder (e.g., "bridges/")
}

export interface AutoRAGResult {
  file_id: string;
  filename: string;
  score: number;
  attributes: {
    modified_date?: number;
    folder?: string;
    file?: {
      url?: string;
      context?: string;
    };
  };
  content: Array<{
    id: string;
    type: string;
    text: string;
  }>;
}

export interface AutoRAGResponse {
  success: boolean;
  result: {
    search_query: string;
    response?: string;         // Only in ai-search mode
    data: AutoRAGResult[];
    has_more: boolean;
    next_page: string | null;
  };
}

export class AutoRAGClient {
  private accountId: string;
  private apiToken: string;
  private baseUrl: string;

  constructor(accountId: string, apiToken: string) {
    this.accountId = accountId;
    this.apiToken = apiToken;
    this.baseUrl = `https://api.cloudflare.com/client/v4/accounts/${accountId}/ai-search/rags`;
  }

  /**
   * AI Search - Retrieval + LLM synthesis
   * Returns synthesized answer + source documents
   */
  async aiSearch(options: AutoRAGSearchOptions): Promise<{ answer: string; sources: AutoRAGResult[] }> {
    const {
      query,
      rag_id = "sysops-beta",
      max_results = 10,
      rewrite_query = true,
      score_threshold = 0.3,
      enable_reranking = true,
      folder_filter,
    } = options;

    const body: any = {
      query,
      model: "@cf/meta/llama-3.3-70b-instruct-fp8-fast",
      rewrite_query,
      max_num_results: max_results,
      ranking_options: {
        score_threshold,
      },
      reranking: {
        enabled: enable_reranking,
        model: "@cf/baai/bge-reranker-base",
      },
      stream: false,
    };

    // Add folder filter if provided
    if (folder_filter) {
      body.filters = {
        type: "eq",
        key: "folder",
        value: folder_filter,
      };
    }

    const response = await fetch(`${this.baseUrl}/${rag_id}/ai-search`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${this.apiToken}`,
      },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`AutoRAG ai-search failed: ${response.statusText} - ${error}`);
    }

    const data = await response.json() as AutoRAGResponse;
    
    return {
      answer: data.result.response || "No answer generated",
      sources: data.result.data,
    };
  }

  /**
   * Search only - Retrieval without LLM synthesis
   * Returns raw document chunks
   */
  async search(options: AutoRAGSearchOptions): Promise<AutoRAGResult[]> {
    const {
      query,
      rag_id = "sysops-beta",
      max_results = 10,
      rewrite_query = true,
      score_threshold = 0.3,
      enable_reranking = true,
      folder_filter,
    } = options;

    const body: any = {
      query,
      rewrite_query,
      max_num_results: max_results,
      ranking_options: {
        score_threshold,
      },
      reranking: {
        enabled: enable_reranking,
        model: "@cf/baai/bge-reranker-base",
      },
    };

    // Add folder filter if provided
    if (folder_filter) {
      body.filters = {
        type: "eq",
        key: "folder",
        value: folder_filter,
      };
    }

    const response = await fetch(`${this.baseUrl}/${rag_id}/search`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${this.apiToken}`,
      },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`AutoRAG search failed: ${response.statusText} - ${error}`);
    }

    const data = await response.json() as AutoRAGResponse;
    return data.result.data;
  }

  /**
   * Format results as markdown for display
   */
  formatResults(answer: string, sources: AutoRAGResult[]): string {
    let output = `## AI Search Results\n\n${answer}\n\n`;

    if (sources.length > 0) {
      output += `### Sources (${sources.length})\n\n`;
      sources.forEach((source, i) => {
        const folder = source.attributes.folder || "";
        const score = Math.round(source.score * 100);
        output += `${i + 1}. **${source.filename}** (${score}% match)\n`;
        output += `   Folder: ${folder}\n`;
        if (source.content.length > 0) {
          const preview = source.content[0].text.substring(0, 200);
          output += `   Preview: ${preview}...\n`;
        }
        output += `\n`;
      });
    }

    return output;
  }
}
