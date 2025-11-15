/**
 * Main TipTap Editor Component
 *
 * The core editor for evna-blocks workspace.
 * Handles command execution, agent responses, and content editing.
 */

'use client';

import { useEditor, EditorContent } from '@tiptap/react';
import { createEditorExtensions, EditorConfig } from '@/editor/config';
import { useEffect, useCallback } from 'react';

export interface EvnaEditorProps {
  config?: EditorConfig;
  initialContent?: string;
  onUpdate?: (content: string) => void;
  onCommandExecute?: (command: string, params: Record<string, any>, commandId: string) => void;
}

export function EvnaEditor({
  config,
  initialContent,
  onUpdate,
  onCommandExecute,
}: EvnaEditorProps) {
  const editor = useEditor({
    extensions: createEditorExtensions(config),
    content: initialContent || '',
    editable: config?.editable ?? true,
    autofocus: config?.autofocus ?? 'end',

    onUpdate: ({ editor }) => {
      const html = editor.getHTML();
      onUpdate?.(html);
    },

    editorProps: {
      attributes: {
        class:
          'prose dark:prose-invert max-w-none focus:outline-none min-h-[calc(100vh-200px)] px-8 py-6',
      },
    },
  });

  // Handle command execution events
  useEffect(() => {
    const handleExecuteCommand = (event: CustomEvent) => {
      if (!editor) {
        console.warn('Editor not ready for command execution');
        return;
      }

      // Extract commandId from event detail (generated in commands.ts)
      const { command, params, commandId } = event.detail;

      if (!commandId) {
        console.error('Command execution missing commandId');
        return;
      }

      // Notify parent component to execute the command with commandId
      onCommandExecute?.(command, params, commandId);

      // Update command marker status to "running"
      editor.chain().focus().updateCommandMarkerStatus(commandId, 'running').run();
    };

    window.addEventListener('execute-command', handleExecuteCommand as EventListener);

    return () => {
      window.removeEventListener('execute-command', handleExecuteCommand as EventListener);
    };
  }, [editor, onCommandExecute]);

  // Public method to insert agent response
  const insertAgentResponse = useCallback(
    (outputType: string, data: any, commandId: string) => {
      if (!editor) return;

      // Update command marker status to completed
      editor.chain().focus().updateCommandMarkerStatus(commandId, 'completed').run();

      // Insert agent response after the command marker
      editor
        .chain()
        .focus()
        .insertAgentResponse({
          outputType: outputType as any,
          data,
          commandId,
          timestamp: new Date().toISOString(),
        })
        .run();
    },
    [editor]
  );

  // Expose methods via ref if needed
  useEffect(() => {
    if (editor) {
      // Attach methods to window for external access (temporary solution)
      (window as any).__evnaEditor = {
        insertAgentResponse,
        updateCommandStatus: (commandId: string, status: string) => {
          editor.chain().focus().updateCommandMarkerStatus(commandId, status as any).run();
        },
      };
    }

    return () => {
      delete (window as any).__evnaEditor;
    };
  }, [editor, insertAgentResponse]);

  if (!editor) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        Loading editor...
      </div>
    );
  }

  return (
    <div className="h-full w-full relative">
      <EditorContent editor={editor} />

      {/* Character count footer */}
      <div className="fixed bottom-4 right-4 text-xs text-gray-400 bg-white dark:bg-gray-800 px-2 py-1 rounded border border-gray-200 dark:border-gray-700">
        {editor.storage.characterCount.characters()} characters
      </div>
    </div>
  );
}
