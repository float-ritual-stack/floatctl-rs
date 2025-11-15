'use client';

import { Block } from '@/lib/types';
import { BlockItem } from './block-item';

interface BlockListProps {
  blocks: Block[];
  onBlockClick?: (blockId: string) => void;
}

export function BlockList({ blocks, onBlockClick }: BlockListProps) {
  if (blocks.length === 0) {
    return (
      <div className="flex h-full items-center justify-center text-zinc-500">
        <div className="text-center">
          <div className="mb-2 text-4xl">💬</div>
          <p className="text-sm">No blocks yet. Start a conversation!</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4 p-4">
      {blocks.map((block) => (
        <BlockItem key={block.id} block={block} onBlockClick={onBlockClick} />
      ))}
    </div>
  );
}
