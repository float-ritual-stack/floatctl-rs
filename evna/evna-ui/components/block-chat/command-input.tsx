'use client';

import { useState, KeyboardEvent } from 'react';
import { Button } from '@/components/ui/button';
import { Send } from 'lucide-react';

interface CommandInputProps {
  onSubmit: (command: string) => void;
  disabled?: boolean;
}

export function CommandInput({ onSubmit, disabled = false }: CommandInputProps) {
  const [command, setCommand] = useState('');

  const handleSubmit = () => {
    if (command.trim() && !disabled) {
      onSubmit(command.trim());
      setCommand('');
    }
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <div className="flex items-end gap-2 border-t bg-white p-4 dark:bg-zinc-950">
      <textarea
        value={command}
        onChange={(e) => setCommand(e.target.value)}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        placeholder="Type a command or question..."
        className="min-h-[60px] flex-1 resize-none rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-zinc-700 dark:bg-zinc-900"
        rows={2}
      />
      <Button
        onClick={handleSubmit}
        disabled={disabled || !command.trim()}
        size="icon"
        className="h-[60px] w-[60px]"
      >
        <Send className="h-5 w-5" />
      </Button>
    </div>
  );
}
