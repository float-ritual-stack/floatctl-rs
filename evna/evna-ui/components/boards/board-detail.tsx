'use client';

import { Board } from '@/lib/types';
import { formatTimestamp } from '@/lib/utils';
import { Tag, ArrowLeft } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface BoardDetailProps {
  board: Board;
  onBack?: () => void;
}

export function BoardDetail({ board, onBack }: BoardDetailProps) {
  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="border-b p-4">
        <div className="mb-2 flex items-center gap-2">
          {onBack && (
            <Button variant="ghost" size="icon" onClick={onBack}>
              <ArrowLeft className="h-4 w-4" />
            </Button>
          )}
          <div className="flex-1">
            <h2 className="text-lg font-semibold text-zinc-900 dark:text-zinc-100">
              {board.title}
            </h2>
            {board.description && (
              <p className="mt-1 text-sm text-zinc-600 dark:text-zinc-400">
                {board.description}
              </p>
            )}
          </div>
        </div>

        {/* Tags */}
        {board.tags.length > 0 && (
          <div className="flex flex-wrap gap-1">
            {board.tags.map((tag) => (
              <span
                key={tag}
                className="inline-flex items-center gap-1 rounded bg-blue-100 px-2 py-1 text-xs text-blue-800 dark:bg-blue-900 dark:text-blue-300"
              >
                <Tag className="h-3 w-3" />
                {tag}
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Posts (BBS-style) */}
      <div className="flex-1 overflow-y-auto p-4">
        <div className="space-y-3">
          {board.posts.map((post, index) => (
            <div
              key={post.id}
              className="rounded-lg border border-zinc-300 bg-white p-3 dark:border-zinc-700 dark:bg-zinc-900"
            >
              {/* Post header */}
              <div className="mb-2 flex items-center justify-between text-xs text-zinc-500">
                <span className="font-mono">#{index + 1}</span>
                <div className="flex items-center gap-2">
                  {post.author && (
                    <span className="font-medium text-zinc-700 dark:text-zinc-300">
                      @{post.author}
                    </span>
                  )}
                  <span>{formatTimestamp(post.timestamp)}</span>
                </div>
              </div>

              {/* Post content */}
              <div className="text-sm text-zinc-800 dark:text-zinc-200">
                {post.content}
              </div>

              {/* Post tags */}
              {post.tags && post.tags.length > 0 && (
                <div className="mt-2 flex flex-wrap gap-1">
                  {post.tags.map((tag) => (
                    <span
                      key={tag}
                      className="rounded bg-zinc-100 px-1.5 py-0.5 text-xs text-zinc-600 dark:bg-zinc-800 dark:text-zinc-400"
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>

        {board.posts.length === 0 && (
          <div className="flex h-full items-center justify-center text-zinc-500">
            <div className="text-center">
              <div className="mb-2 text-4xl">📝</div>
              <p className="text-sm">No posts yet</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
