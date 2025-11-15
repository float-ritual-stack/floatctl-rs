/**
 * AgentResponse React component
 *
 * Renders agent responses using the component registry.
 * Dynamically selects the appropriate component based on output type.
 */

'use client';

import { NodeViewWrapper } from '@tiptap/react';
import { NodeViewProps } from '@tiptap/core';
import { AgentResponseAttrs } from '@/types/editor';
import { AgentOutputRegistry } from '@/components/agent-outputs/registry';

export function AgentResponseComponent({ node }: NodeViewProps) {
  const attrs = node.attrs as AgentResponseAttrs;
  const { outputType, data } = attrs;

  // Get the appropriate component from registry
  const OutputComponent = AgentOutputRegistry[outputType];

  if (!OutputComponent) {
    return (
      <NodeViewWrapper className="my-4">
        <div className="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md">
          <p className="text-red-600 dark:text-red-400 text-sm">
            Unknown output type: {outputType}
          </p>
          <pre className="mt-2 text-xs text-gray-600 dark:text-gray-400 overflow-auto">
            {JSON.stringify(data, null, 2)}
          </pre>
        </div>
      </NodeViewWrapper>
    );
  }

  return (
    <NodeViewWrapper className="my-4">
      <OutputComponent data={data} />
    </NodeViewWrapper>
  );
}
