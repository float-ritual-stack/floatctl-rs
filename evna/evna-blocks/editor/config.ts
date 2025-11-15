/**
 * TipTap Editor Configuration
 *
 * Central configuration for the evna-blocks editor.
 * Brings together all extensions and custom nodes.
 */

import { Extensions } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';
import Placeholder from '@tiptap/extension-placeholder';
import CharacterCount from '@tiptap/extension-character-count';

// Custom extensions
import { Commands } from './extensions/commands';
import { CommandMarker } from './nodes/command-marker/node';
import { AgentResponse } from './nodes/agent-response/node';

export interface EditorConfig {
  placeholder?: string;
  editable?: boolean;
  autofocus?: boolean | 'start' | 'end';
}

export function createEditorExtensions(config: EditorConfig = {}): Extensions {
  const {
    placeholder = 'Start writing or type "/" for commands...',
    editable = true,
    autofocus = 'end',
  } = config;

  return [
    // Base extensions
    StarterKit.configure({
      heading: {
        levels: [1, 2, 3],
      },
      codeBlock: {
        HTMLAttributes: {
          class: 'bg-gray-100 dark:bg-gray-800 rounded p-2 font-mono text-sm',
        },
      },
    }),

    // Utility extensions
    Placeholder.configure({
      placeholder,
    }),

    CharacterCount,

    // Custom extensions
    Commands,
    CommandMarker,
    AgentResponse,
  ];
}

export const defaultEditorConfig: EditorConfig = {
  placeholder: 'Start writing or type "/" for commands...',
  editable: true,
  autofocus: 'end',
};
