'use client';

import { useState, useEffect } from 'react';
import { Board } from '@/lib/types';
import { boardStore, initializeSampleBoards } from '@/lib/boards/store';
import { BoardCard } from './board-card';
import { BoardDetail } from './board-detail';
import { Separator } from '@/components/ui/separator';

export function BoardsPanel() {
  const [boards, setBoards] = useState<Board[]>([]);
  const [selectedBoard, setSelectedBoard] = useState<Board | null>(null);

  useEffect(() => {
    // Initialize sample boards on mount
    initializeSampleBoards();
    // Use a callback to avoid calling setState directly in effect
    setTimeout(() => {
      setBoards(boardStore.getAllBoards());
    }, 0);
  }, []);

  const handleSelectBoard = (boardId: string) => {
    const board = boardStore.getBoard(boardId);
    if (board) {
      setSelectedBoard(board);
    }
  };

  const handleBack = () => {
    setSelectedBoard(null);
  };

  return (
    <div className="flex h-full flex-col bg-zinc-50 dark:bg-zinc-900">
      {/* Header */}
      <div className="flex items-center justify-between border-b px-4 py-3">
        <h2 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
          BBS Boards
        </h2>
        <div className="flex gap-2 text-xs text-zinc-500">
          <span>{boards.length} boards</span>
        </div>
      </div>

      <Separator />

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {selectedBoard ? (
          <BoardDetail board={selectedBoard} onBack={handleBack} />
        ) : (
          <div className="space-y-3 p-4">
            {boards.map((board) => (
              <BoardCard
                key={board.id}
                board={board}
                onSelect={handleSelectBoard}
              />
            ))}

            {boards.length === 0 && (
              <div className="flex h-full items-center justify-center text-zinc-500">
                <div className="text-center">
                  <div className="mb-2 text-4xl">📋</div>
                  <p className="text-sm">No boards yet</p>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
