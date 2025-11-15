/**
 * CommandMarker React component
 *
 * Visual representation of a command marker in the editor.
 * Shows command name, parameters, and execution status.
 */

'use client';

import { NodeViewWrapper } from '@tiptap/react';
import { NodeViewProps } from '@tiptap/core';
import { CommandMarkerAttrs } from '@/types/editor';
import { Loader2, CheckCircle, XCircle, Terminal } from 'lucide-react';

export function CommandMarkerComponent({ node, updateAttributes }: NodeViewProps) {
  const attrs = node.attrs as CommandMarkerAttrs;
  const { command, params, status } = attrs;

  // Status icon
  const StatusIcon = () => {
    switch (status) {
      case 'pending':
        return <Terminal className="w-4 h-4 text-gray-400" />;
      case 'running':
        return <Loader2 className="w-4 h-4 text-blue-500 animate-spin" />;
      case 'completed':
        return <CheckCircle className="w-4 h-4 text-green-500" />;
      case 'error':
        return <XCircle className="w-4 h-4 text-red-500" />;
    }
  };

  // Format params for display
  const formatParams = (params: Record<string, any>): string => {
    const entries = Object.entries(params);
    if (entries.length === 0) return '';

    return entries
      .map(([key, value]) => {
        if (typeof value === 'string') return value;
        return `${key}=${JSON.stringify(value)}`;
      })
      .join(' ');
  };

  return (
    <NodeViewWrapper className="my-2">
      <div
        className="flex items-center gap-2 px-3 py-2 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-md font-mono text-sm group hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
        data-command-marker
      >
        <StatusIcon />

        <span className="text-blue-600 dark:text-blue-400 font-medium">
          /{command}
        </span>

        {Object.keys(params).length > 0 && (
          <span className="text-gray-600 dark:text-gray-400">
            {formatParams(params)}
          </span>
        )}

        {status === 'running' && (
          <span className="ml-auto text-xs text-gray-500">Running...</span>
        )}

        {status === 'error' && (
          <span className="ml-auto text-xs text-red-500">Failed</span>
        )}
      </div>
    </NodeViewWrapper>
  );
}
