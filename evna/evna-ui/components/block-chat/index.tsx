'use client';

import { useRef, useEffect } from 'react';
import { Block } from '@/lib/types';
import { BlockList } from './block-list';
import { CommandInput } from './command-input';
import { Separator } from '@/components/ui/separator';

interface BlockChatProps {
  onCommand: (command: string) => Promise<void>;
  blocks: Block[];
  isProcessing?: boolean;
}

export function BlockChat({ onCommand, blocks, isProcessing = false }: BlockChatProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new blocks are added
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [blocks]);

  return (
    <div className="flex h-full flex-col bg-white dark:bg-zinc-950">
      {/* Header */}
      <div className="flex items-center justify-between border-b px-4 py-3">
        <h2 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
          Block Chat
        </h2>
        <div className="flex gap-2 text-xs text-zinc-500">
          <span>{blocks.length} blocks</span>
          {isProcessing && <span className="text-blue-500">● Processing...</span>}
        </div>
      </div>

      <Separator />

      {/* Block list */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto">
        <BlockList blocks={blocks} />
      </div>

      <Separator />

      {/* Command input */}
      <CommandInput onSubmit={onCommand} disabled={isProcessing} />
    </div>
  );
}
