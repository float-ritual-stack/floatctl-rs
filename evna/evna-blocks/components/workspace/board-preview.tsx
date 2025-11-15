/**
 * Board Preview Pane Component
 *
 * Displays BBS boards in the right pane of the workspace.
 * Listens for "show-board" events to update the displayed board.
 */

'use client';

import { useEffect, useState } from 'react';
import { Card } from '@/components/ui/card';
import { Layout, MessageCircle, User, Clock } from 'lucide-react';

interface Board {
  id: string;
  name: string;
  description: string;
  threads: Thread[];
}

interface Thread {
  id: string;
  title: string;
  author: string;
  timestamp: string;
  preview: string;
  replies: number;
}

// Demo board data
const DEMO_BOARDS: Record<string, Board> = {
  'restoration': {
    id: 'restoration',
    name: 'Restoration',
    description: 'Project restoration discussions and updates',
    threads: [
      {
        id: '1',
        title: 'Database schema migration complete',
        author: 'evan',
        timestamp: '2025-11-15T10:30:00Z',
        preview: 'Successfully migrated all tables to new schema. Performance improvements visible...',
        replies: 5,
      },
      {
        id: '2',
        title: 'Frontend component refactoring',
        author: 'claude',
        timestamp: '2025-11-15T09:15:00Z',
        preview: 'Breaking down large components into smaller, reusable pieces...',
        replies: 3,
      },
      {
        id: '3',
        title: 'API endpoint optimization',
        author: 'evan',
        timestamp: '2025-11-14T16:45:00Z',
        preview: 'Reduced response times by 40% through query optimization and caching...',
        replies: 8,
      },
    ],
  },
  'evna': {
    id: 'evna',
    name: 'EVNA Development',
    description: 'EVNA agent development and enhancements',
    threads: [
      {
        id: '1',
        title: 'Brain boot multi-source integration',
        author: 'evan',
        timestamp: '2025-11-15T11:00:00Z',
        preview: 'Implemented dual-source search with Cohere reranking...',
        replies: 12,
      },
      {
        id: '2',
        title: 'TipTap blocks interface design',
        author: 'claude',
        timestamp: '2025-11-15T08:30:00Z',
        preview: 'Architecting block-based chat with custom React components...',
        replies: 6,
      },
    ],
  },
};

export function BoardPreview() {
  const [activeBoard, setActiveBoard] = useState<Board | null>(DEMO_BOARDS.restoration);

  useEffect(() => {
    const handleShowBoard = (event: Event) => {
      const customEvent = event as CustomEvent;
      const { boardId } = customEvent.detail;

      const board = DEMO_BOARDS[boardId];
      if (board) {
        setActiveBoard(board);
      }
    };

    window.addEventListener('show-board', handleShowBoard);

    return () => {
      window.removeEventListener('show-board', handleShowBoard);
    };
  }, []);

  if (!activeBoard) {
    return (
      <div className="h-full flex flex-col items-center justify-center text-gray-400 p-8">
        <Layout className="w-16 h-16 mb-4 opacity-50" />
        <p className="text-center">No board selected</p>
        <p className="text-sm text-center mt-2">
          Boards will appear here when referenced by agent responses
        </p>
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto">
      {/* Board Header */}
      <div className="sticky top-0 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 p-4 z-10">
        <div className="flex items-start gap-3">
          <Layout className="w-5 h-5 text-orange-500 mt-0.5 flex-shrink-0" />
          <div className="flex-1 min-w-0">
            <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
              {activeBoard.name}
            </h2>
            <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
              {activeBoard.description}
            </p>
          </div>
        </div>
      </div>

      {/* Threads */}
      <div className="p-4 space-y-3">
        {activeBoard.threads.map((thread) => (
          <Card
            key={thread.id}
            className="p-4 hover:shadow-md transition-shadow cursor-pointer"
          >
            <h3 className="font-medium text-gray-900 dark:text-gray-100 mb-2">
              {thread.title}
            </h3>

            <p className="text-sm text-gray-600 dark:text-gray-400 mb-3 line-clamp-2">
              {thread.preview}
            </p>

            <div className="flex items-center gap-4 text-xs text-gray-500">
              <div className="flex items-center gap-1">
                <User className="w-3 h-3" />
                <span>{thread.author}</span>
              </div>

              <div className="flex items-center gap-1">
                <MessageCircle className="w-3 h-3" />
                <span>{thread.replies} replies</span>
              </div>

              <div className="flex items-center gap-1 ml-auto">
                <Clock className="w-3 h-3" />
                <span>{new Date(thread.timestamp).toLocaleDateString()}</span>
              </div>
            </div>
          </Card>
        ))}
      </div>
    </div>
  );
}
