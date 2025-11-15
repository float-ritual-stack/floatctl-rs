/**
 * Slash Commands Extension
 *
 * Provides a command palette for triggering agent actions.
 * Activated by typing "/" in the editor.
 */

import { Extension } from '@tiptap/core';
import Suggestion from '@tiptap/suggestion';
import { ReactRenderer } from '@tiptap/react';
import { CommandItem } from '@/types/editor';
import tippy, { Instance as TippyInstance } from 'tippy.js';

// Command definitions
export const AGENT_COMMANDS: CommandItem[] = [
  {
    label: 'brain_boot',
    description: 'Morning synthesis with multi-source context',
    icon: '🧠',
    command: 'brain_boot',
    category: 'agent',
  },
  {
    label: 'search',
    description: 'Semantic search across conversation history',
    icon: '🔍',
    command: 'search',
    category: 'agent',
  },
  {
    label: 'context',
    description: 'Query recent active context',
    icon: '📝',
    command: 'context',
    category: 'agent',
  },
  {
    label: 'ask',
    description: 'Ask evna orchestrator (general query)',
    icon: '💬',
    command: 'ask',
    category: 'agent',
  },
  {
    label: 'board',
    description: 'Insert BBS board embed',
    icon: '📋',
    command: 'board',
    category: 'insert',
  },
];

export const Commands = Extension.create({
  name: 'commands',

  addOptions() {
    return {
      suggestion: {
        char: '/',
        allowSpaces: true,
        startOfLine: false,

        items: ({ query }: { query: string }) => {
          return AGENT_COMMANDS.filter((item) =>
            item.label.toLowerCase().startsWith(query.toLowerCase())
          );
        },

        render: () => {
          let component: ReactRenderer;
          let popup: TippyInstance[];

          return {
            onStart: (props: any) => {
              // Import component dynamically to avoid SSR issues
              import('@/components/editor/command-palette').then(
                ({ CommandPalette }) => {
                  component = new ReactRenderer(CommandPalette, {
                    props,
                    editor: props.editor,
                  });

                  if (!props.clientRect) {
                    return;
                  }

                  popup = tippy('body', {
                    getReferenceClientRect: props.clientRect,
                    appendTo: () => document.body,
                    content: component.element,
                    showOnCreate: true,
                    interactive: true,
                    trigger: 'manual',
                    placement: 'bottom-start',
                  });
                }
              );
            },

            onUpdate(props: any) {
              component?.updateProps(props);

              if (!props.clientRect) {
                return;
              }

              popup?.[0]?.setProps({
                getReferenceClientRect: props.clientRect,
              });
            },

            onKeyDown(props: any) {
              if (props.event.key === 'Escape') {
                popup?.[0]?.hide();
                return true;
              }

              // @ts-ignore
              return component?.ref?.onKeyDown?.(props);
            },

            onExit() {
              popup?.[0]?.destroy();
              component?.destroy();
            },
          };
        },

        command: ({ editor, range, props }: any) => {
          const command = props.command as string;

          // Delete the trigger character and command text
          editor.chain().focus().deleteRange(range).run();

          // Insert command marker
          editor
            .chain()
            .focus()
            .insertCommandMarker({
              command,
              params: {},
            })
            .run();

          // Trigger agent execution
          // This will be handled by the Editor component via event listener
          window.dispatchEvent(
            new CustomEvent('execute-command', {
              detail: { command, params: {} },
            })
          );
        },
      },
    };
  },

  addProseMirrorPlugins() {
    return [
      Suggestion({
        editor: this.editor,
        ...this.options.suggestion,
      }),
    ];
  },
});
