import { anthropic, createAnthropic } from "@ai-sdk/anthropic";
import { openai, createOpenAI } from "@ai-sdk/openai";

// Configure AI models with optional AI Gateway
export function getAnthropicModel(model: string = "claude-3-5-sonnet-20241022") {
  if (process.env.AI_GATEWAY_URL) {
    const customAnthropic = createAnthropic({
      baseURL: process.env.AI_GATEWAY_URL,
      headers: {
        "cf-aig-authorization": `Bearer ${process.env.AI_GATEWAY_API_KEY}`,
      },
    });
    return customAnthropic(model);
  }
  return anthropic(model);
}

export function getOpenAIModel(model: string = "gpt-4o") {
  if (process.env.AI_GATEWAY_URL) {
    const customOpenAI = createOpenAI({
      baseURL: process.env.AI_GATEWAY_URL,
      headers: {
        "cf-aig-authorization": `Bearer ${process.env.AI_GATEWAY_API_KEY}`,
      },
    });
    return customOpenAI(model);
  }
  return openai(model);
}

// EVNA system prompt
export const EVNA_SYSTEM_PROMPT = `You are EVNA, an AI agent specialized in context synthesis and semantic search across conversation history.

You help users:
- Perform morning "brain boot" - synthesizing recent activity and relevant historical context
- Search conversation history semantically using natural language queries
- Track active context across different clients (Desktop, Claude Code)
- Parse and understand annotation systems (ctx::, project::, meeting::, mode::)

Key capabilities:
- Semantic search with pgvector embeddings
- Active context tracking with cross-client surfacing
- Multi-source ranking with Cohere reranking
- Smart truncation and context synthesis

You are part of the Queer Techno Bard cognitive ecosystem - a system for externalizing memory and enabling "remember forward" through technology-mediated thought.`;
