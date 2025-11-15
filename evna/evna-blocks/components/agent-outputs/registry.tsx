/**
 * Agent Output Component Registry
 *
 * Maps output types to their corresponding React components.
 * Add new output types here as they are implemented.
 */

import React from 'react';
import { BrainBootOutput } from './brain-boot';
import { SearchResultsOutput } from './search-results';
import { ContextTimelineOutput } from './context-timeline';
import { BoardEmbedOutput } from './board-embed';
import { AgentOutputType, AgentOutputData } from '@/types/agent-outputs';

// Component type: receives data and renders output
export type AgentOutputComponent<T extends AgentOutputData = AgentOutputData> = (props: {
  data: T;
}) => React.JSX.Element;

// Registry mapping output types to components
export const AgentOutputRegistry: Record<AgentOutputType, AgentOutputComponent<any>> = {
  brain_boot: BrainBootOutput,
  search: SearchResultsOutput,
  context: ContextTimelineOutput,
  board_embed: BoardEmbedOutput,
} as const;

// Type guard to check if output type is valid
export function isValidOutputType(type: string): type is AgentOutputType {
  return type in AgentOutputRegistry;
}
