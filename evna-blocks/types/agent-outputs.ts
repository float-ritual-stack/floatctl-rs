/**
 * Type definitions for agent outputs and structured responses
 */

import { z } from 'zod';

// Base types for all agent outputs
export interface BaseAgentOutput {
  type: string;
  timestamp: string; // ISO 8601 timestamp string
}

// Brain Boot Output Schema
export const BrainBootOutputSchema = z.object({
  summary: z.string(),
  sections: z.array(z.object({
    title: z.string(),
    items: z.array(z.string()),
    expandable: z.boolean(),
  })),
  boardReferences: z.array(z.object({
    id: z.string(),
    preview: z.string(),
  })),
});

export type BrainBootOutput = z.infer<typeof BrainBootOutputSchema>;

// Search Results Schema
export const SearchResultsSchema = z.object({
  query: z.string(),
  results: z.array(z.object({
    id: z.string(),
    title: z.string(),
    excerpt: z.string(),
    similarity: z.number(),
    source: z.enum(['active_context', 'embeddings']),
    timestamp: z.string(),
  })),
  totalResults: z.number(),
});

export type SearchResults = z.infer<typeof SearchResultsSchema>;

// Context Timeline Schema
export const ContextTimelineSchema = z.object({
  timeRange: z.object({
    start: z.string(),
    end: z.string(),
  }),
  events: z.array(z.object({
    id: z.string(),
    type: z.enum(['message', 'commit', 'pr', 'issue']),
    title: z.string(),
    content: z.string(),
    timestamp: z.string(),
    metadata: z.record(z.string(), z.any()).optional(),
  })),
});

export type ContextTimeline = z.infer<typeof ContextTimelineSchema>;

// Board Embed Schema
export const BoardEmbedSchema = z.object({
  boardId: z.string(),
  boardName: z.string(),
  threadCount: z.number(),
  recentThreads: z.array(z.object({
    id: z.string(),
    title: z.string(),
    author: z.string(),
    timestamp: z.string(),
  })),
});

export type BoardEmbed = z.infer<typeof BoardEmbedSchema>;

// Union type for all possible agent outputs
export type AgentOutputData =
  | BrainBootOutput
  | SearchResults
  | ContextTimeline
  | BoardEmbed;

// Output type discriminator
export type AgentOutputType =
  | 'brain_boot'
  | 'search'
  | 'context'
  | 'board_embed';

// Generic agent output container
export interface AgentOutput<T extends AgentOutputData = AgentOutputData> extends BaseAgentOutput {
  outputType: AgentOutputType;
  data: T;
}
