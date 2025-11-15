'use client';

import { Block } from '@/lib/types';
import { cn, formatTimestamp } from '@/lib/utils';
import { User, Bot } from 'lucide-react';

interface BlockItemProps {
  block: Block;
  onBlockClick?: (blockId: string) => void;
}

export function BlockItem({ block, onBlockClick }: BlockItemProps) {
  const isUser = block.role === 'user';
  
  return (
    <div
      className={cn(
        'group relative rounded-lg border p-4 transition-all',
        'hover:shadow-md cursor-pointer',
        isUser
          ? 'border-blue-200 bg-blue-50 dark:border-blue-800 dark:bg-blue-950'
          : 'border-zinc-200 bg-white dark:border-zinc-800 dark:bg-zinc-950',
      )}
      onClick={() => onBlockClick?.(block.id)}
    >
      {/* Header */}
      <div className="mb-2 flex items-start justify-between">
        <div className="flex items-center gap-2">
          <div
            className={cn(
              'rounded-full p-1.5',
              isUser
                ? 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300'
                : 'bg-zinc-100 text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300'
            )}
          >
            {isUser ? <User className="h-4 w-4" /> : <Bot className="h-4 w-4" />}
          </div>
          <div>
            <div className="text-sm font-medium">
              {isUser ? 'You' : block.metadata.agent || 'Assistant'}
            </div>
            <div className="text-xs text-zinc-500">
              {formatTimestamp(block.metadata.timestamp)}
            </div>
          </div>
        </div>
        
        <div className="rounded bg-zinc-100 px-2 py-0.5 text-xs font-medium text-zinc-600 dark:bg-zinc-800 dark:text-zinc-400">
          {block.blockType}
        </div>
      </div>

      {/* Content */}
      <div className="prose prose-sm max-w-none dark:prose-invert">
        {block.blockType === 'userCommand' && (
          <div className="font-mono text-sm">$ {block.content}</div>
        )}
        {block.blockType === 'agentResponse' && (
          <div className="whitespace-pre-wrap">{block.content}</div>
        )}
        {block.blockType === 'boardSummary' && block.structuredOutput && (
          <div className="rounded border border-green-200 bg-green-50 p-3 dark:border-green-800 dark:bg-green-950">
            <div className="mb-2 text-sm font-semibold text-green-800 dark:text-green-300">
              📋 Board Summary
            </div>
            <pre className="text-xs">{JSON.stringify(block.structuredOutput, null, 2)}</pre>
          </div>
        )}
        {block.blockType === 'structuredComponent' && block.structuredOutput && (
          <div className="rounded border border-purple-200 bg-purple-50 p-3 dark:border-purple-800 dark:bg-purple-950">
            <div className="mb-2 text-sm font-semibold text-purple-800 dark:text-purple-300">
              🎨 Structured Output
            </div>
            <pre className="text-xs">{JSON.stringify(block.structuredOutput, null, 2)}</pre>
          </div>
        )}
        {block.blockType === 'error' && (
          <div className="rounded border border-red-200 bg-red-50 p-3 text-red-800 dark:border-red-800 dark:bg-red-950 dark:text-red-300">
            {block.content}
          </div>
        )}
      </div>

      {/* Associated board indicator */}
      {block.metadata.associatedBoardId && (
        <div className="mt-2 text-xs text-zinc-500">
          🔗 Linked to board: {block.metadata.associatedBoardId}
        </div>
      )}
    </div>
  );
}
