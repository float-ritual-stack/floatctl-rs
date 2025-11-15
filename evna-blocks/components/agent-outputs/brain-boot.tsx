/**
 * Brain Boot Output Component
 *
 * Renders morning brain boot synthesis results with expandable sections
 * and board references.
 */

'use client';

import { useState } from 'react';
import { Card } from '@/components/ui/card';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { Button } from '@/components/ui/button';
import { BrainBootOutput as BrainBootData } from '@/types/agent-outputs';
import { Brain, ChevronDown, ChevronRight, ExternalLink } from 'lucide-react';

interface BrainBootOutputProps {
  data: BrainBootData;
}

export function BrainBootOutput({ data }: BrainBootOutputProps) {
  const [expandedSections, setExpandedSections] = useState<Set<number>>(new Set([0])); // First section expanded by default

  const toggleSection = (index: number) => {
    const next = new Set(expandedSections);
    if (next.has(index)) {
      next.delete(index);
    } else {
      next.add(index);
    }
    setExpandedSections(next);
  };

  const handleBoardClick = (boardId: string) => {
    // Dispatch event to show board in preview pane
    window.dispatchEvent(
      new CustomEvent('show-board', {
        detail: { boardId },
      })
    );
  };

  return (
    <Card className="p-4 border-l-4 border-l-blue-500 bg-gradient-to-br from-blue-50/50 to-transparent dark:from-blue-950/20 dark:to-transparent">
      {/* Header */}
      <div className="flex items-center gap-2 mb-3">
        <Brain className="w-5 h-5 text-blue-600 dark:text-blue-400" />
        <span className="font-semibold text-blue-900 dark:text-blue-100">Brain Boot</span>
      </div>

      {/* Summary */}
      <p className="mb-4 text-gray-700 dark:text-gray-300 leading-relaxed">
        {data.summary}
      </p>

      {/* Sections */}
      {data.sections.map((section, idx) => (
        <div key={idx} className="mb-3 last:mb-0">
          <Collapsible
            open={expandedSections.has(idx)}
            onOpenChange={() => toggleSection(idx)}
          >
            <CollapsibleTrigger asChild>
              <button className="flex items-center gap-2 w-full text-left hover:bg-gray-100 dark:hover:bg-gray-800 p-2 rounded transition-colors">
                {expandedSections.has(idx) ? (
                  <ChevronDown className="w-4 h-4 text-gray-500" />
                ) : (
                  <ChevronRight className="w-4 h-4 text-gray-500" />
                )}
                <span className="font-medium text-gray-900 dark:text-gray-100">
                  {section.title}
                </span>
                <span className="ml-auto text-xs text-gray-500">
                  {section.items.length} items
                </span>
              </button>
            </CollapsibleTrigger>

            <CollapsibleContent>
              <ul className="mt-2 ml-6 space-y-1">
                {section.items.map((item, i) => (
                  <li
                    key={i}
                    className="text-sm text-gray-700 dark:text-gray-300 list-disc"
                  >
                    {item}
                  </li>
                ))}
              </ul>
            </CollapsibleContent>
          </Collapsible>
        </div>
      ))}

      {/* Board References */}
      {data.boardReferences.length > 0 && (
        <div className="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
          <div className="text-sm text-gray-600 dark:text-gray-400 mb-2">
            Referenced Boards:
          </div>
          <div className="flex flex-wrap gap-2">
            {data.boardReferences.map((board) => (
              <Button
                key={board.id}
                variant="outline"
                size="sm"
                onClick={() => handleBoardClick(board.id)}
                className="gap-2"
              >
                {board.preview}
                <ExternalLink className="w-3 h-3" />
              </Button>
            ))}
          </div>
        </div>
      )}
    </Card>
  );
}
