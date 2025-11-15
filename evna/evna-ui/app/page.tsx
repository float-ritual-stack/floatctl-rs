'use client';

import { useState } from 'react';
import { Panel, PanelGroup, PanelResizeHandle } from 'react-resizable-panels';
import { SidebarNote } from '@/components/sidebar-note';
import { BlockChat } from '@/components/block-chat';
import { BoardsPanel } from '@/components/boards';
import { Block } from '@/lib/types';
import { generateId } from '@/lib/utils';
import { dispatcher } from '@/lib/ai/dispatcher';

export default function Home() {
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [isProcessing, setIsProcessing] = useState(false);

  const handleCommand = async (command: string) => {
    // Create a user command block
    const commandBlock: Block = {
      id: generateId('block'),
      blockType: 'userCommand',
      role: 'user',
      content: command,
      metadata: {
        timestamp: new Date().toISOString(),
      },
    };
    
    setBlocks((prev) => [...prev, commandBlock]);
    setIsProcessing(true);
    
    try {
      // Send to AI using fetch API
      const response = await fetch('/api/chat', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          messages: [
            {
              role: 'user',
              content: command,
            },
          ],
        }),
      });

      if (!response.ok) {
        throw new Error('Failed to get response from AI');
      }

      // Read the streaming response
      const reader = response.body?.getReader();
      const decoder = new TextDecoder();
      let fullContent = '';

      if (reader) {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          
          const chunk = decoder.decode(value);
          fullContent += chunk;
        }
      }

      // Parse the response for structured outputs
      const parsed = dispatcher.parseResponse(fullContent);
      
      // Create a block for the assistant response
      const responseBlock: Block = {
        id: generateId('block'),
        blockType: 'agentResponse',
        role: 'assistant',
        content: parsed.text,
        metadata: {
          timestamp: new Date().toISOString(),
          agent: 'evna',
        },
      };
      
      setBlocks((prev) => [...prev, responseBlock]);
      
      // Process any structured outputs
      parsed.structuredOutputs.forEach((output) => {
        dispatcher.dispatch(output);
      });
    } catch (error) {
      // Create an error block
      const errorBlock: Block = {
        id: generateId('block'),
        blockType: 'error',
        role: 'system',
        content: error instanceof Error ? error.message : 'An error occurred',
        metadata: {
          timestamp: new Date().toISOString(),
        },
      };
      
      setBlocks((prev) => [...prev, errorBlock]);
    } finally {
      setIsProcessing(false);
    }
  };

  return (
    <div className="h-screen w-screen overflow-hidden bg-zinc-100 dark:bg-zinc-950">
      <PanelGroup direction="horizontal">
        {/* Left Sidebar - Continuous Note */}
        <Panel defaultSize={20} minSize={15} maxSize={30}>
          <SidebarNote />
        </Panel>

        <PanelResizeHandle className="w-1 bg-zinc-300 transition-colors hover:bg-blue-500 dark:bg-zinc-700" />

        {/* Main Block Chat Area */}
        <Panel defaultSize={50} minSize={30}>
          <BlockChat
            blocks={blocks}
            onCommand={handleCommand}
            isProcessing={isProcessing}
          />
        </Panel>

        <PanelResizeHandle className="w-1 bg-zinc-300 transition-colors hover:bg-blue-500 dark:bg-zinc-700" />

        {/* Right Panel - BBS Boards */}
        <Panel defaultSize={30} minSize={20} maxSize={40}>
          <BoardsPanel />
        </Panel>
      </PanelGroup>
    </div>
  );
}
