import { z } from 'zod';

/**
 * Block types that can be rendered in the main chat area
 */
export const BlockType = z.enum([
  'userCommand',
  'agentResponse',
  'boardSummary',
  'boardReference',
  'structuredComponent',
  'noteDecoration',
  'error',
]);

export type BlockType = z.infer<typeof BlockType>;

/**
 * Base block metadata
 */
export const BlockMetadataSchema = z.object({
  timestamp: z.string().datetime(),
  agent: z.string().optional(),
  associatedBoardId: z.string().optional(),
  sidebarMarkerRange: z.object({
    from: z.number(),
    to: z.number(),
  }).optional(),
});

export type BlockMetadata = z.infer<typeof BlockMetadataSchema>;

/**
 * Core block structure
 */
export const BlockSchema = z.object({
  id: z.string(),
  blockType: BlockType,
  role: z.enum(['user', 'assistant', 'system']),
  content: z.string(),
  metadata: BlockMetadataSchema,
  structuredOutput: z.any().optional(), // Structured data from AI
});

export type Block = z.infer<typeof BlockSchema>;

/**
 * Structured output types from AI
 */
export const StructuredOutputType = z.enum([
  'boardSummary',
  'boardCreation',
  'noteDecoration',
  'codeBlock',
  'dataVisualization',
  'terminalOutput',
]);

export type StructuredOutputType = z.infer<typeof StructuredOutputType>;

/**
 * Board summary structured output
 */
export const BoardSummaryOutputSchema = z.object({
  type: z.literal('boardSummary'),
  boardId: z.string(),
  title: z.string(),
  items: z.array(z.object({
    id: z.string(),
    content: z.string(),
    timestamp: z.string().datetime().optional(),
  })),
});

export type BoardSummaryOutput = z.infer<typeof BoardSummaryOutputSchema>;

/**
 * Note decoration structured output
 */
export const NoteDecorationOutputSchema = z.object({
  type: z.literal('noteDecoration'),
  range: z.object({
    from: z.number(),
    to: z.number(),
  }),
  style: z.string(),
  annotation: z.string().optional(),
});

export type NoteDecorationOutput = z.infer<typeof NoteDecorationOutputSchema>;

/**
 * Generic structured output
 */
export const StructuredOutputSchema = z.discriminatedUnion('type', [
  BoardSummaryOutputSchema,
  NoteDecorationOutputSchema,
  // More types can be added here
]);

export type StructuredOutput = z.infer<typeof StructuredOutputSchema>;
