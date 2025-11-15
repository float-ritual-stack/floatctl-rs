/**
 * Workspace Page Client Component
 *
 * Main interactive workspace that orchestrates editor, commands, and agent responses.
 */

'use client';

import { useCallback, useRef } from 'react';
import { WorkspaceLayout } from '@/components/workspace/layout';
import { WorkspaceSidebar } from '@/components/workspace/sidebar';
import { BoardPreview } from '@/components/workspace/board-preview';
import { EvnaEditor, EditorInstance } from '@/components/editor/editor';
import { executeBrainBoot } from './actions/brain-boot';
import { toast } from 'sonner';

export function WorkspacePage() {
  const editorRef = useRef<EditorInstance | null>(null);

  const handleEditorReady = useCallback((instance: EditorInstance) => {
    editorRef.current = instance;
  }, []);

  const handleCommandExecute = useCallback(async (command: string, params: Record<string, any>, commandId: string) => {
    try {
      // Show loading state
      toast.loading(`Executing /${command}...`, { id: command });

      let outputType: string;
      let data: any;

      // Route command to appropriate server action
      switch (command) {
        case 'brain_boot':
          data = await executeBrainBoot({
            query: params.query || 'recent work',
            project: params.project,
            lookbackDays: params.lookbackDays || 7,
          });
          outputType = 'brain_boot';
          break;

        case 'search':
          // TODO: Implement search action
          toast.error('Search not yet implemented', { id: command });
          return;

        case 'context':
          // TODO: Implement context action
          toast.error('Context not yet implemented', { id: command });
          return;

        case 'ask':
          // TODO: Implement ask_evna action
          toast.error('Ask evna not yet implemented', { id: command });
          return;

        case 'board':
          // TODO: Implement board embed
          toast.error('Board embed not yet implemented', { id: command });
          return;

        default:
          toast.error(`Unknown command: ${command}`, { id: command });
          return;
      }

      // Insert agent response into editor using commandId from event
      if (!editorRef.current) {
        throw new Error('Editor not initialized');
      }
      editorRef.current.insertAgentResponse(outputType, data, commandId);

      toast.success(`/${command} completed`, { id: command });
    } catch (error) {
      console.error('Command execution error:', error);
      toast.error(`Failed to execute /${command}`, { id: command });
    }
  }, []);

  return (
    <>
      <WorkspaceLayout
        sidebar={<WorkspaceSidebar />}
        editor={<EvnaEditor onCommandExecute={handleCommandExecute} onEditorReady={handleEditorReady} />}
        boardPreview={<BoardPreview />}
        showSidebar={false}
        showBoardPreview={true}
      />
    </>
  );
}
