/**
 * TipTap editor-specific types
 */

import { Editor } from '@tiptap/core';
import { AgentOutputType, AgentOutputData } from './agent-outputs';

// Command palette item
export interface CommandItem {
  label: string;
  description: string;
  icon?: string;
  command: string;
  category: 'agent' | 'formatting' | 'insert';
}

// Command marker attributes
export interface CommandMarkerAttrs {
  command: string;
  params: Record<string, any>;
  status: 'pending' | 'running' | 'completed' | 'error';
  triggeredAt: string;
  commandId?: string; // Optional - generated if not provided
}

// Agent response attributes
export interface AgentResponseAttrs {
  outputType: AgentOutputType;
  data: AgentOutputData;
  commandId: string; // Links back to command marker
  timestamp: string;
}

// Editor context for React components
export interface EditorContext {
  editor: Editor;
  updateAttributes: (attrs: Record<string, any>) => void;
  deleteNode: () => void;
}
