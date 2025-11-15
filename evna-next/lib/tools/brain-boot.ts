import { dynamicTool } from "ai";
import { z } from "zod";
import { generateEmbedding } from "../embeddings";
import { semanticSearch, getActiveContext } from "../db";

export const brainBootTool = dynamicTool({
  description:
    "Morning brain boot: Combines semantic search with recent context synthesis. Use this for morning check-ins, context restoration, or when returning from breaks. Provides a comprehensive overview of recent and relevant work.",
  inputSchema: z.object({
    query: z
      .string()
      .describe("Natural language description of what to retrieve (e.g., 'what was I working on yesterday?')"),
    project: z.string().optional().describe("Filter by project name"),
    lookbackDays: z.number().optional().default(7).describe("How many days to look back"),
    maxResults: z.number().optional().default(10).describe("Maximum results to return"),
  }),
  execute: async (input: any) => {
    const { query, project, lookbackDays, maxResults } = input;
    try {
      // Calculate since timestamp
      const since = new Date(
        Date.now() - (lookbackDays || 7) * 24 * 60 * 60 * 1000
      ).toISOString();

      // Parallel fetch: semantic search + active context
      const [embedding, activeContextResults] = await Promise.all([
        generateEmbedding(query),
        getActiveContext({ query, limit: Math.floor(maxResults * 0.3), project }),
      ]);

      const semanticResults = await semanticSearch({
        query,
        embedding,
        limit: maxResults * 2, // Get more for deduplication
        threshold: 0.3, // Lower threshold for brain boot
        project,
        since,
      });

      // Combine and deduplicate results
      const allResults = [...semanticResults, ...activeContextResults];
      const seen = new Set<string>();
      const deduped = allResults.filter((r) => {
        const key = `${r.conversation_id}-${r.timestamp}-${r.content.substring(0, 50)}`;
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });

      // Sort by timestamp and limit
      const sorted = deduped
        .sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
        .slice(0, maxResults);

      if (sorted.length === 0) {
        return {
          success: true,
          message: "No recent context found. This might be your first session or you haven't captured context recently.",
          results: [],
        };
      }

      // Synthesize summary
      const summary = {
        totalResults: sorted.length,
        projects: [...new Set(sorted.map((r) => r.project).filter(Boolean))],
        timeRange: {
          earliest: sorted[sorted.length - 1]?.timestamp,
          latest: sorted[0]?.timestamp,
        },
      };

      return {
        success: true,
        message: `Brain boot complete! Found ${sorted.length} relevant items across ${summary.projects.length} project(s).`,
        summary,
        results: sorted.map((r) => ({
          content: r.content,
          timestamp: r.timestamp,
          project: r.project,
          meeting: r.meeting,
          mode: r.mode,
          similarity: r.similarity,
          source: r.source,
          conversationId: r.conversation_id,
        })),
      };
    } catch (error) {
      console.error("Brain boot error:", error);
      return {
        success: false,
        message: `Error performing brain boot: ${error instanceof Error ? error.message : "Unknown error"}`,
        results: [],
      };
    }
  },
});
