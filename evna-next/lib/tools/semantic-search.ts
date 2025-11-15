import { dynamicTool } from "ai";
import { z } from "zod";
import { generateEmbedding } from "../embeddings";
import { semanticSearch } from "../db";

export const semanticSearchTool = dynamicTool({
  description:
    "Deep semantic search across conversation history using pgvector embeddings. Use this to find relevant past conversations, discussions, or context based on natural language queries.",
  inputSchema: z.object({
    query: z.string().describe("Search query (natural language, question, or keywords)"),
    limit: z.number().optional().default(10).describe("Maximum results to return"),
    project: z.string().optional().describe("Filter by project name (e.g., 'rangle/pharmacy')"),
    since: z.string().optional().describe("Filter by timestamp (ISO 8601 format)"),
    threshold: z.number().optional().default(0.5).describe("Similarity threshold 0-1 (lower = more results)"),
  }),
  execute: async (input: any) => {
    const { query, limit, project, since, threshold } = input;
    try {
      // Generate embedding for the query
      const embedding = await generateEmbedding(query);

      // Perform semantic search
      const results = await semanticSearch({
        query,
        embedding,
        limit,
        threshold,
        project,
        since,
      });

      if (results.length === 0) {
        return {
          success: true,
          message: "No results found matching your query.",
          results: [],
        };
      }

      return {
        success: true,
        message: `Found ${results.length} relevant results`,
        results: results.map((r) => ({
          content: r.content,
          similarity: r.similarity,
          timestamp: r.timestamp,
          project: r.project,
          meeting: r.meeting,
          conversationId: r.conversation_id,
        })),
      };
    } catch (error) {
      console.error("Semantic search error:", error);
      return {
        success: false,
        message: `Error performing semantic search: ${error instanceof Error ? error.message : "Unknown error"}`,
        results: [],
      };
    }
  },
});
