/**
 * Workspace Layout Component
 *
 * Three-pane layout: Sidebar | Editor | Board Preview
 * Supports collapsible panes and responsive design.
 */

'use client';

import { useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  PanelLeftClose,
  PanelLeftOpen,
  PanelRightClose,
  PanelRightOpen,
} from 'lucide-react';

export interface WorkspaceLayoutProps {
  sidebar?: React.ReactNode;
  editor: React.ReactNode;
  boardPreview?: React.ReactNode;
  showSidebar?: boolean;
  showBoardPreview?: boolean;
}

export function WorkspaceLayout({
  sidebar,
  editor,
  boardPreview,
  showSidebar: initialShowSidebar = false,
  showBoardPreview: initialShowBoardPreview = true,
}: WorkspaceLayoutProps) {
  const [showSidebar, setShowSidebar] = useState(initialShowSidebar);
  const [showBoardPreview, setShowBoardPreview] = useState(initialShowBoardPreview);

  return (
    <div className="h-screen w-screen flex flex-col bg-gray-50 dark:bg-gray-900">
      {/* Header */}
      <header className="h-14 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 flex items-center px-4 gap-2">
        {/* Toggle Sidebar */}
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setShowSidebar(!showSidebar)}
          className="gap-2"
        >
          {showSidebar ? (
            <PanelLeftClose className="w-4 h-4" />
          ) : (
            <PanelLeftOpen className="w-4 h-4" />
          )}
        </Button>

        {/* Title */}
        <div className="flex-1">
          <h1 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            EVNA Blocks
          </h1>
        </div>

        {/* Toggle Board Preview */}
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setShowBoardPreview(!showBoardPreview)}
          className="gap-2"
        >
          {showBoardPreview ? (
            <>
              <PanelRightClose className="w-4 h-4" />
              <span className="hidden sm:inline">Hide Board</span>
            </>
          ) : (
            <>
              <PanelRightOpen className="w-4 h-4" />
              <span className="hidden sm:inline">Show Board</span>
            </>
          )}
        </Button>
      </header>

      {/* Main Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Sidebar */}
        {showSidebar && sidebar && (
          <aside className="w-64 border-r border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 overflow-y-auto">
            {sidebar}
          </aside>
        )}

        {/* Editor */}
        <main
          className={`flex-1 bg-white dark:bg-gray-800 overflow-y-auto ${
            !showSidebar && !showBoardPreview ? 'max-w-4xl mx-auto' : ''
          }`}
        >
          {editor}
        </main>

        {/* Board Preview */}
        {showBoardPreview && boardPreview && (
          <aside className="w-96 border-l border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 overflow-y-auto">
            {boardPreview}
          </aside>
        )}
      </div>
    </div>
  );
}
