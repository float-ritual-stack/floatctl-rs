import { Extension } from '@tiptap/core';
import { Node } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';
import Placeholder from '@tiptap/extension-placeholder';

/**
 * Command Marker Node - marks when a command was sent
 */
export const CommandMarker = Node.create({
  name: 'commandMarker',
  
  group: 'block',
  
  content: 'inline*',
  
  addAttributes() {
    return {
      commandId: {
        default: null,
      },
      timestamp: {
        default: null,
      },
      agent: {
        default: null,
      },
      command: {
        default: null,
      },
    };
  },
  
  parseHTML() {
    return [
      {
        tag: 'div[data-command-marker]',
      },
    ];
  },
  
  renderHTML({ HTMLAttributes }) {
    return ['div', { 'data-command-marker': '', ...HTMLAttributes }, 0];
  },
});

/**
 * Block Reference Node - references a block in the main area
 */
export const BlockReference = Node.create({
  name: 'blockReference',
  
  group: 'inline',
  
  inline: true,
  
  addAttributes() {
    return {
      blockId: {
        default: null,
      },
      blockType: {
        default: null,
      },
    };
  },
  
  parseHTML() {
    return [
      {
        tag: 'span[data-block-reference]',
      },
    ];
  },
  
  renderHTML({ HTMLAttributes }) {
    return ['span', { 'data-block-reference': '', ...HTMLAttributes }];
  },
});

/**
 * Board Reference Node - references a board
 */
export const BoardReferenceNode = Node.create({
  name: 'boardReference',
  
  group: 'block',
  
  atom: true,
  
  addAttributes() {
    return {
      boardId: {
        default: null,
      },
      boardTitle: {
        default: null,
      },
    };
  },
  
  parseHTML() {
    return [
      {
        tag: 'div[data-board-reference]',
      },
    ];
  },
  
  renderHTML({ HTMLAttributes }) {
    return ['div', { 'data-board-reference': '', ...HTMLAttributes }];
  },
});

/**
 * Get all Tiptap extensions for the sidebar editor
 */
export function getSidebarExtensions() {
  return [
    StarterKit.configure({
      heading: {
        levels: [1, 2, 3],
      },
    }),
    Placeholder.configure({
      placeholder: 'Write notes, ideas, or commands...',
    }),
    CommandMarker,
    BlockReference,
    BoardReferenceNode,
  ];
}

/**
 * Note: Custom commands for insertingCommandMarker can be added later.
 * For now, users can insert command markers manually or via toolbar buttons.
 * 
 * To insert a command marker programmatically:
 * editor.chain().focus().insertContent({
 *   type: 'commandMarker',
 *   attrs: { commandId: 'xxx', timestamp: '...', command: '...' }
 * }).run()
 */
