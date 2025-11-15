/**
 * Board Embed Output Component
 *
 * Renders an inline preview of a BBS board with recent threads.
 */

'use client';

import { Card } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { BoardEmbed as BoardEmbedData } from '@/types/agent-outputs';
import { Layout, ExternalLink, MessageCircle } from 'lucide-react';

interface BoardEmbedOutputProps {
  data: BoardEmbedData;
}

export function BoardEmbedOutput({ data }: BoardEmbedOutputProps) {
  const handleShowBoard = () => {
    window.dispatchEvent(
      new CustomEvent('show-board', {
        detail: { boardId: data.boardId },
      })
    );
  };

  return (
    <Card className="p-4 border-l-4 border-l-orange-500 bg-gradient-to-br from-orange-50/50 to-transparent dark:from-orange-950/20 dark:to-transparent">
      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Layout className="w-5 h-5 text-orange-600 dark:text-orange-400" />
          <span className="font-semibold text-orange-900 dark:text-orange-100">
            Board: {data.boardName}
          </span>
          <span className="text-sm text-gray-500">
            {data.threadCount} threads
          </span>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={handleShowBoard}
          className="gap-2"
        >
          Open in preview
          <ExternalLink className="w-3 h-3" />
        </Button>
      </div>

      {/* Recent Threads */}
      <div className="space-y-2">
        {data.recentThreads.map((thread) => (
          <div
            key={thread.id}
            className="p-3 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md hover:shadow-md transition-shadow cursor-pointer"
            onClick={handleShowBoard}
          >
            <div className="flex items-start gap-2">
              <MessageCircle className="w-4 h-4 text-gray-400 mt-0.5 flex-shrink-0" />
              <div className="flex-1 min-w-0">
                <h4 className="font-medium text-gray-900 dark:text-gray-100 truncate">
                  {thread.title}
                </h4>
                <div className="text-xs text-gray-500 mt-1">
                  {thread.author} · {new Date(thread.timestamp).toLocaleDateString()}
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
}
