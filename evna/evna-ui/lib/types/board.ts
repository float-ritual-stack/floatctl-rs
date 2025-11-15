import { z } from 'zod';

/**
 * BBS-style board post/entry
 */
export const BoardPostSchema = z.object({
  id: z.string(),
  author: z.string().optional(),
  content: z.string(),
  timestamp: z.string().datetime(),
  tags: z.array(z.string()).optional(),
  metadata: z.record(z.any()).optional(),
});

export type BoardPost = z.infer<typeof BoardPostSchema>;

/**
 * Board entity - represents a BBS-style board
 */
export const BoardSchema = z.object({
  id: z.string(),
  title: z.string(),
  description: z.string().optional(),
  tags: z.array(z.string()),
  posts: z.array(BoardPostSchema),
  createdAt: z.string().datetime(),
  lastUpdatedAt: z.string().datetime(),
  metadata: z.record(z.any()).optional(),
});

export type Board = z.infer<typeof BoardSchema>;

/**
 * Board creation request
 */
export const CreateBoardRequestSchema = z.object({
  title: z.string(),
  description: z.string().optional(),
  tags: z.array(z.string()).optional(),
  initialPost: z.string().optional(),
});

export type CreateBoardRequest = z.infer<typeof CreateBoardRequestSchema>;

/**
 * Board update request
 */
export const UpdateBoardRequestSchema = z.object({
  title: z.string().optional(),
  description: z.string().optional(),
  tags: z.array(z.string()).optional(),
  addPost: BoardPostSchema.optional(),
});

export type UpdateBoardRequest = z.infer<typeof UpdateBoardRequestSchema>;
