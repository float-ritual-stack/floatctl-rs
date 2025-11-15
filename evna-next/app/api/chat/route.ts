import { streamText, convertToModelMessages } from "ai";
import { getAnthropicModel, EVNA_SYSTEM_PROMPT } from "@/lib/ai-config";
import { semanticSearchTool, activeContextTool, brainBootTool } from "@/lib/tools";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

export async function POST(req: Request) {
  const { messages } = await req.json();

  const result = streamText({
    model: getAnthropicModel(),
    system: EVNA_SYSTEM_PROMPT,
    messages: convertToModelMessages(messages),
    tools: {
      semantic_search: semanticSearchTool,
      active_context: activeContextTool,
      brain_boot: brainBootTool,
    },
  });

  return result.toTextStreamResponse();
}
