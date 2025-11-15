'use client';

import { useEditor, EditorContent } from '@tiptap/react';
import { getSidebarExtensions } from '@/lib/tiptap/extensions';
import { cn } from '@/lib/utils';

interface SidebarEditorProps {
  content?: string;
  onUpdate?: (content: string) => void;
  editable?: boolean;
  className?: string;
}

export function SidebarEditor({
  content = '',
  onUpdate,
  editable = true,
  className,
}: SidebarEditorProps) {
  const editor = useEditor({
    extensions: getSidebarExtensions(),
    content,
    editable,
    immediatelyRender: false, // Required for SSR compatibility
    editorProps: {
      attributes: {
        class: cn(
          'prose prose-sm max-w-none focus:outline-none',
          'prose-headings:font-semibold prose-headings:tracking-tight',
          'prose-h1:text-xl prose-h2:text-lg prose-h3:text-base',
          'prose-p:leading-relaxed prose-p:my-2',
          'prose-ul:my-2 prose-ol:my-2',
          'prose-li:my-1',
          '[&_[data-command-marker]]:bg-blue-50 [&_[data-command-marker]]:border-l-2',
          '[&_[data-command-marker]]:border-blue-400 [&_[data-command-marker]]:pl-3',
          '[&_[data-command-marker]]:py-2 [&_[data-command-marker]]:my-2',
          '[&_[data-command-marker]]:rounded-r [&_[data-command-marker]]:font-mono',
          '[&_[data-command-marker]]:text-sm [&_[data-command-marker]]:text-blue-800',
          '[&_[data-block-reference]]:bg-purple-100 [&_[data-block-reference]]:px-1',
          '[&_[data-block-reference]]:py-0.5 [&_[data-block-reference]]:rounded',
          '[&_[data-block-reference]]:text-purple-800 [&_[data-block-reference]]:text-xs',
          '[&_[data-block-reference]]:font-medium [&_[data-block-reference]]:cursor-pointer',
          '[&_[data-block-reference]]:hover:bg-purple-200',
          '[&_[data-board-reference]]:bg-green-50 [&_[data-board-reference]]:border',
          '[&_[data-board-reference]]:border-green-300 [&_[data-board-reference]]:p-2',
          '[&_[data-board-reference]]:rounded [&_[data-board-reference]]:my-2',
          '[&_[data-board-reference]]:cursor-pointer [&_[data-board-reference]]:hover:bg-green-100',
        ),
      },
    },
    onUpdate: ({ editor }) => {
      if (onUpdate) {
        onUpdate(editor.getHTML());
      }
    },
  });

  return (
    <div className={cn('min-h-full p-4', className)}>
      <EditorContent editor={editor} />
    </div>
  );
}
