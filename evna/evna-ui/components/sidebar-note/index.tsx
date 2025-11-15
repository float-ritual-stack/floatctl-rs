'use client';

import { useState } from 'react';
import { SidebarEditor } from './sidebar-editor';
import { Separator } from '@/components/ui/separator';

export function SidebarNote() {
  const [content, setContent] = useState('');

  return (
    <div className="flex h-full flex-col bg-zinc-50 dark:bg-zinc-900">
      <div className="flex items-center justify-between border-b px-4 py-3">
        <h2 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
          Continuous Note
        </h2>
        <div className="flex gap-2 text-xs text-zinc-500">
          <span>Tiptap</span>
        </div>
      </div>
      
      <Separator />
      
      <div className="flex-1 overflow-y-auto">
        <SidebarEditor content={content} onUpdate={setContent} />
      </div>
      
      <Separator />
      
      <div className="border-t px-4 py-2 text-xs text-zinc-500">
        <p>Write notes, mark commands, reference blocks</p>
      </div>
    </div>
  );
}
