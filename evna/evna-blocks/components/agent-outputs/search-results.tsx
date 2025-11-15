/**
 * Search Results Output Component
 *
 * Renders semantic search results with similarity scores and source attribution.
 */

'use client';

import { Card } from '@/components/ui/card';
import { SearchResults as SearchResultsData } from '@/types/agent-outputs';
import { Search, ExternalLink } from 'lucide-react';

interface SearchResultsOutputProps {
  data: SearchResultsData;
}

export function SearchResultsOutput({ data }: SearchResultsOutputProps) {
  return (
    <Card className="p-4 border-l-4 border-l-purple-500 bg-gradient-to-br from-purple-50/50 to-transparent dark:from-purple-950/20 dark:to-transparent">
      {/* Header */}
      <div className="flex items-center gap-2 mb-3">
        <Search className="w-5 h-5 text-purple-600 dark:text-purple-400" />
        <span className="font-semibold text-purple-900 dark:text-purple-100">
          Search Results
        </span>
        <span className="ml-auto text-sm text-gray-500">
          {data.totalResults} results for "{data.query}"
        </span>
      </div>

      {/* Results */}
      <div className="space-y-3">
        {data.results.map((result) => (
          <div
            key={result.id}
            className="p-3 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md hover:shadow-md transition-shadow"
          >
            <div className="flex items-start justify-between gap-2 mb-1">
              <h4 className="font-medium text-gray-900 dark:text-gray-100">
                {result.title}
              </h4>
              <div className="flex items-center gap-2 text-xs text-gray-500">
                <span className={result.source === 'active_context' ? 'text-blue-600' : 'text-gray-600'}>
                  {result.source}
                </span>
                <span className="font-mono">
                  {(result.similarity * 100).toFixed(0)}%
                </span>
              </div>
            </div>
            <p className="text-sm text-gray-600 dark:text-gray-400 mb-2">
              {result.excerpt}
            </p>
            <div className="text-xs text-gray-500">
              {new Date(result.timestamp).toLocaleString()}
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
}
