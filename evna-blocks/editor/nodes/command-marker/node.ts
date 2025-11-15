/**
 * CommandMarker TipTap node definition
 *
 * This node represents a user command (e.g., /brain_boot) that triggers an agent.
 * It stores the command, parameters, and execution status.
 */

import { Node, mergeAttributes } from '@tiptap/core';
import { ReactNodeViewRenderer } from '@tiptap/react';
import { CommandMarkerComponent } from './component';
import { CommandMarkerAttrs } from '@/types/editor';

export interface CommandMarkerOptions {
  HTMLAttributes: Record<string, any>;
}

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    commandMarker: {
      /**
       * Insert a command marker at the current position
       */
      insertCommandMarker: (attrs: Omit<CommandMarkerAttrs, 'status' | 'triggeredAt'>) => ReturnType;
      /**
       * Update command marker status
       */
      updateCommandMarkerStatus: (
        commandId: string,
        status: CommandMarkerAttrs['status']
      ) => ReturnType;
    };
  }
}

export const CommandMarker = Node.create<CommandMarkerOptions>({
  name: 'commandMarker',

  group: 'block',

  atom: true,

  selectable: true,

  draggable: true,

  addOptions() {
    return {
      HTMLAttributes: {},
    };
  },

  addAttributes() {
    return {
      command: {
        default: '',
        parseHTML: element => element.getAttribute('data-command'),
        renderHTML: attributes => {
          return {
            'data-command': attributes.command,
          };
        },
      },
      params: {
        default: {},
        parseHTML: element => {
          const params = element.getAttribute('data-params');
          return params ? JSON.parse(params) : {};
        },
        renderHTML: attributes => {
          return {
            'data-params': JSON.stringify(attributes.params),
          };
        },
      },
      status: {
        default: 'pending',
        parseHTML: element => element.getAttribute('data-status'),
        renderHTML: attributes => {
          return {
            'data-status': attributes.status,
          };
        },
      },
      triggeredAt: {
        default: null,
        parseHTML: element => element.getAttribute('data-triggered-at'),
        renderHTML: attributes => {
          return {
            'data-triggered-at': attributes.triggeredAt,
          };
        },
      },
      commandId: {
        default: null,
        parseHTML: element => element.getAttribute('data-command-id'),
        renderHTML: attributes => {
          return {
            'data-command-id': attributes.commandId,
          };
        },
      },
    };
  },

  parseHTML() {
    return [
      {
        tag: 'div[data-type="command-marker"]',
      },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      'div',
      mergeAttributes(
        { 'data-type': 'command-marker' },
        this.options.HTMLAttributes,
        HTMLAttributes
      ),
    ];
  },

  addNodeView() {
    return ReactNodeViewRenderer(CommandMarkerComponent);
  },

  addCommands() {
    return {
      insertCommandMarker:
        (attrs) =>
        ({ chain }) => {
          const commandId = `cmd_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

          return chain()
            .insertContent({
              type: this.name,
              attrs: {
                ...attrs,
                status: 'pending',
                triggeredAt: new Date().toISOString(),
                commandId,
              },
            })
            .run();
        },

      updateCommandMarkerStatus:
        (commandId, status) =>
        ({ tr, state }) => {
          let updated = false;

          state.doc.descendants((node, pos) => {
            if (node.type.name === 'commandMarker' && node.attrs.commandId === commandId) {
              tr.setNodeMarkup(pos, undefined, {
                ...node.attrs,
                status,
              });
              updated = true;
              return false; // Stop searching
            }
          });

          return updated;
        },
    };
  },
});
