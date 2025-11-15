import { StructuredOutput, Block } from '@/lib/types';
import { generateId } from '@/lib/utils';

/**
 * Dispatch structured AI outputs to appropriate handlers
 */
export class StructuredOutputDispatcher {
  private handlers: Map<string, (output: StructuredOutput) => void> = new Map();

  /**
   * Register a handler for a specific output type
   */
  registerHandler(type: string, handler: (output: StructuredOutput) => void) {
    this.handlers.set(type, handler);
  }

  /**
   * Dispatch a structured output to its handler
   */
  dispatch(output: StructuredOutput) {
    const handler = this.handlers.get(output.type);
    if (handler) {
      handler(output);
    } else {
      console.warn(`No handler registered for output type: ${output.type}`);
    }
  }

  /**
   * Parse AI response for structured outputs
   * This is a simple implementation - can be enhanced with more sophisticated parsing
   */
  parseResponse(content: string): { text: string; structuredOutputs: StructuredOutput[] } {
    const structuredOutputs: StructuredOutput[] = [];
    let text = content;

    // Look for JSON blocks in the response
    const jsonRegex = /```json\s*(\{[\s\S]*?\})\s*```/g;
    let match;

    while ((match = jsonRegex.exec(content)) !== null) {
      try {
        const parsed = JSON.parse(match[1]);
        if (this.isStructuredOutput(parsed)) {
          structuredOutputs.push(parsed);
          // Remove the JSON block from the text
          text = text.replace(match[0], '');
        }
      } catch {
        // Not valid JSON or not a structured output, skip
      }
    }

    return {
      text: text.trim(),
      structuredOutputs,
    };
  }

  /**
   * Type guard for structured outputs
   */
  private isStructuredOutput(obj: unknown): obj is StructuredOutput {
    return typeof obj === 'object' && obj !== null && 'type' in obj;
  }
}

/**
 * Convert structured output to a block
 */
export function structuredOutputToBlock(output: StructuredOutput): Block {
  return {
    id: generateId('block'),
    blockType: output.type === 'boardSummary' ? 'boardSummary' : 'structuredComponent',
    role: 'assistant',
    content: JSON.stringify(output, null, 2),
    metadata: {
      timestamp: new Date().toISOString(),
      agent: 'evna',
    },
    structuredOutput: output,
  };
}

/**
 * Global dispatcher instance
 */
export const dispatcher = new StructuredOutputDispatcher();
