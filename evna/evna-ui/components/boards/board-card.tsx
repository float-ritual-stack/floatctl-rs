'use client';

import { Board } from '@/lib/types';
import { formatTimestamp } from '@/lib/utils';
import { MessageSquare, Tag } from 'lucide-react';

interface BoardCardProps {
  board: Board;
  onSelect?: (boardId: string) => void;
}

export function BoardCard({ board, onSelect }: BoardCardProps) {
  return (
    <div
      onClick={() => onSelect?.(board.id)}
      className="cursor-pointer rounded-lg border border-zinc-300 bg-zinc-50 p-4 transition-all hover:border-blue-400 hover:shadow-md dark:border-zinc-700 dark:bg-zinc-900"
    >
      {/* Header */}
      <div className="mb-2">
        <h3 className="font-semibold text-zinc-900 dark:text-zinc-100">
          {board.title}
        </h3>
        {board.description && (
          <p className="mt-1 text-xs text-zinc-600 dark:text-zinc-400">
            {board.description}
          </p>
        )}
      </div>

      {/* Tags */}
      {board.tags.length > 0 && (
        <div className="mb-3 flex flex-wrap gap-1">
          {board.tags.map((tag) => (
            <span
              key={tag}
              className="inline-flex items-center gap-1 rounded bg-blue-100 px-2 py-0.5 text-xs text-blue-800 dark:bg-blue-900 dark:text-blue-300"
            >
              <Tag className="h-3 w-3" />
              {tag}
            </span>
          ))}
        </div>
      )}

      {/* Stats */}
      <div className="flex items-center justify-between text-xs text-zinc-500">
        <div className="flex items-center gap-1">
          <MessageSquare className="h-3 w-3" />
          <span>{board.posts.length} posts</span>
        </div>
        <span>Updated {formatTimestamp(board.lastUpdatedAt)}</span>
      </div>
    </div>
  );
}
