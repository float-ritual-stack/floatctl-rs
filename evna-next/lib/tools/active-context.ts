import { dynamicTool } from "ai";
import { z } from "zod";
import { getActiveContext } from "../db";

export const activeContextTool = dynamicTool({
  description:
    "Query recent activity with annotation parsing. Captures and retrieves recent context across different clients (Desktop, Claude Code). Supports project filtering and cross-client surfacing.",
  inputSchema: z.object({
    query: z.string().optional().describe("Search query for filtering context"),
    limit: z.number().optional().default(10).describe("Maximum results to return"),
    project: z.string().optional().describe("Filter by project name (fuzzy matching)"),
  }),
  execute: async (input: any) => {
    const { query, limit, project } = input;
    try {
      const results = await getActiveContext({
        query,
        limit,
        project,
      });

      if (results.length === 0) {
        return {
          success: true,
          message: "No active context found.",
          results: [],
        };
      }

      return {
        success: true,
        message: `Found ${results.length} active context entries`,
        results: results.map((r) => ({
          content: r.content,
          timestamp: r.timestamp,
          project: r.project,
          meeting: r.meeting,
          mode: r.mode,
          clientType: r.client_type,
          conversationId: r.conversation_id,
        })),
      };
    } catch (error) {
      console.error("Active context error:", error);
      return {
        success: false,
        message: `Error retrieving active context: ${error instanceof Error ? error.message : "Unknown error"}`,
        results: [],
      };
    }
  },
});
