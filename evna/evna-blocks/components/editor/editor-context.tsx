/**
 * Editor Context
 *
 * Provides access to editor methods without global window mutations.
 * Allows multiple editor instances and better TypeScript support.
 */

'use client';

import { createContext, useContext } from 'react';
import { Editor } from '@tiptap/core';

export interface EditorContextValue {
  editor: Editor | null;
  insertAgentResponse: (
    outputType: string,
    data: any,
    commandId: string
  ) => void;
  updateCommandStatus: (commandId: string, status: string) => void;
}

export const EditorContext = createContext<EditorContextValue | null>(null);

export function useEditorContext() {
  const context = useContext(EditorContext);
  if (!context) {
    throw new Error('useEditorContext must be used within EditorProvider');
  }
  return context;
}
