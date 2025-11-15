/**
 * Context Timeline Output Component
 *
 * Renders a timeline of recent context events (messages, commits, PRs, issues).
 */

'use client';

import { Card } from '@/components/ui/card';
import { ContextTimeline as ContextTimelineData } from '@/types/agent-outputs';
import { Clock, MessageSquare, GitCommit, GitPullRequest, CircleDot } from 'lucide-react';

interface ContextTimelineOutputProps {
  data: ContextTimelineData;
}

export function ContextTimelineOutput({ data }: ContextTimelineOutputProps) {
  const getEventIcon = (type: string) => {
    switch (type) {
      case 'message':
        return <MessageSquare className="w-4 h-4" />;
      case 'commit':
        return <GitCommit className="w-4 h-4" />;
      case 'pr':
        return <GitPullRequest className="w-4 h-4" />;
      case 'issue':
        return <CircleDot className="w-4 h-4" />;
      default:
        return <Clock className="w-4 h-4" />;
    }
  };

  return (
    <Card className="p-4 border-l-4 border-l-green-500 bg-gradient-to-br from-green-50/50 to-transparent dark:from-green-950/20 dark:to-transparent">
      {/* Header */}
      <div className="flex items-center gap-2 mb-3">
        <Clock className="w-5 h-5 text-green-600 dark:text-green-400" />
        <span className="font-semibold text-green-900 dark:text-green-100">
          Context Timeline
        </span>
        <span className="ml-auto text-sm text-gray-500">
          {data.timeRange.start} → {data.timeRange.end}
        </span>
      </div>

      {/* Timeline */}
      <div className="relative space-y-4">
        {/* Vertical line */}
        <div className="absolute left-[11px] top-2 bottom-2 w-[2px] bg-gray-200 dark:bg-gray-700" />

        {data.events.map((event, idx) => (
          <div key={event.id} className="relative flex gap-3">
            {/* Icon */}
            <div className="relative z-10 flex-shrink-0 w-6 h-6 bg-white dark:bg-gray-800 border-2 border-green-500 rounded-full flex items-center justify-center text-green-600 dark:text-green-400">
              {getEventIcon(event.type)}
            </div>

            {/* Content */}
            <div className="flex-1 pb-4">
              <div className="flex items-start justify-between gap-2 mb-1">
                <h4 className="font-medium text-gray-900 dark:text-gray-100">
                  {event.title}
                </h4>
                <span className="text-xs text-gray-500 whitespace-nowrap">
                  {new Date(event.timestamp).toLocaleTimeString()}
                </span>
              </div>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                {event.content}
              </p>
              {event.metadata && (
                <div className="mt-2 text-xs text-gray-500 font-mono">
                  {Object.entries(event.metadata).map(([key, value]) => (
                    <span key={key} className="mr-3">
                      {key}: {String(value)}
                    </span>
                  ))}
                </div>
              )}
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
}
