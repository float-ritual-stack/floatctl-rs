/**
 * AgentResponse TipTap node definition
 *
 * This node displays agent responses using structured outputs.
 * It renders different React components based on the output type.
 */

import { Node, mergeAttributes } from '@tiptap/core';
import { ReactNodeViewRenderer } from '@tiptap/react';
import { AgentResponseComponent } from './component';
import { AgentResponseAttrs } from '@/types/editor';

export interface AgentResponseOptions {
  HTMLAttributes: Record<string, any>;
}

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    agentResponse: {
      /**
       * Insert an agent response at the current position
       */
      insertAgentResponse: (attrs: AgentResponseAttrs) => ReturnType;
    };
  }
}

export const AgentResponse = Node.create<AgentResponseOptions>({
  name: 'agentResponse',

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
      outputType: {
        default: 'brain_boot',
        parseHTML: element => element.getAttribute('data-output-type'),
        renderHTML: attributes => {
          return {
            'data-output-type': attributes.outputType,
          };
        },
      },
      data: {
        default: {},
        parseHTML: element => {
          const data = element.getAttribute('data-output-data');
          try {
            return data ? JSON.parse(data) : {};
          } catch (e) {
            console.error('Failed to parse agent response data:', e);
            return {};
          }
        },
        renderHTML: attributes => {
          try {
            return {
              'data-output-data': JSON.stringify(attributes.data),
            };
          } catch (e) {
            console.error('Failed to stringify agent response data:', e);
            return { 'data-output-data': '{}' };
          }
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
      timestamp: {
        default: null,
        parseHTML: element => element.getAttribute('data-timestamp'),
        renderHTML: attributes => {
          return {
            'data-timestamp': attributes.timestamp,
          };
        },
      },
    };
  },

  parseHTML() {
    return [
      {
        tag: 'div[data-type="agent-response"]',
      },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      'div',
      mergeAttributes(
        { 'data-type': 'agent-response' },
        this.options.HTMLAttributes,
        HTMLAttributes
      ),
    ];
  },

  addNodeView() {
    return ReactNodeViewRenderer(AgentResponseComponent);
  },

  addCommands() {
    return {
      insertAgentResponse:
        (attrs) =>
        ({ chain }) => {
          return chain()
            .insertContent({
              type: this.name,
              attrs: {
                ...attrs,
                timestamp: attrs.timestamp || new Date().toISOString(),
              },
            })
            .run();
        },
    };
  },
});
