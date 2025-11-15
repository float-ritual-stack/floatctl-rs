/**
 * Command Palette Component
 *
 * Dropdown UI shown when user types "/" in the editor.
 * Displays available commands with filtering.
 */

'use client';

import { forwardRef, useEffect, useImperativeHandle, useState } from 'react';
import { CommandItem } from '@/types/editor';

export interface CommandPaletteProps {
  items: CommandItem[];
  command: (item: CommandItem) => void;
}

export interface CommandPaletteRef {
  onKeyDown: (event: { event: KeyboardEvent }) => boolean;
}

export const CommandPalette = forwardRef<CommandPaletteRef, CommandPaletteProps>(
  ({ items, command }, ref) => {
    const [selectedIndex, setSelectedIndex] = useState(0);

    useEffect(() => {
      setSelectedIndex(0);
    }, [items]);

    const selectItem = (index: number) => {
      const item = items[index];
      if (item) {
        command(item);
      }
    };

    useImperativeHandle(ref, () => ({
      onKeyDown: ({ event }) => {
        if (event.key === 'ArrowUp') {
          setSelectedIndex((prev) => (prev + items.length - 1) % items.length);
          return true;
        }

        if (event.key === 'ArrowDown') {
          setSelectedIndex((prev) => (prev + 1) % items.length);
          return true;
        }

        if (event.key === 'Enter') {
          selectItem(selectedIndex);
          return true;
        }

        return false;
      },
    }));

    if (items.length === 0) {
      return (
        <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg p-2 min-w-[300px]">
          <div className="px-3 py-2 text-sm text-gray-500">No commands found</div>
        </div>
      );
    }

    return (
      <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg overflow-hidden min-w-[300px] max-h-[400px] overflow-y-auto">
        {items.map((item, index) => (
          <button
            key={item.command}
            onClick={() => selectItem(index)}
            className={`w-full text-left px-3 py-2 transition-colors ${
              index === selectedIndex
                ? 'bg-blue-50 dark:bg-blue-900/20'
                : 'hover:bg-gray-50 dark:hover:bg-gray-700'
            }`}
          >
            <div className="flex items-start gap-3">
              <span className="text-xl flex-shrink-0">{item.icon}</span>
              <div className="flex-1 min-w-0">
                <div className="font-medium text-gray-900 dark:text-gray-100">
                  /{item.label}
                </div>
                <div className="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
                  {item.description}
                </div>
                <div className="text-xs text-gray-400 dark:text-gray-500 mt-1">
                  {item.category}
                </div>
              </div>
            </div>
          </button>
        ))}
      </div>
    );
  }
);

CommandPalette.displayName = 'CommandPalette';
