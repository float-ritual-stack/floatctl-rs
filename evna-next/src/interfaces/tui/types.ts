/**
 * Type definitions for Agent SDK + OpenTUI integration
 */

export interface CursorPosition {
  line: number
  col: number
}

export interface AgentMessage {
  role: "user" | "assistant" | "system"
  content: ContentBlock[]
  usage?: UsageStats
  stop_reason?: string
  model?: string
}

export type ContentBlock =
  | TextBlock
  | ToolUseBlock
  | ToolResultBlock
  | ThinkingBlock

export interface TextBlock {
  type: "text"
  text: string
}

export interface ToolUseBlock {
  type: "tool_use"
  id: string
  name: string
  input: any
}

export interface ToolResultBlock {
  type: "tool_result"
  tool_use_id: string
  content: any
  is_error: boolean
}

export interface ThinkingBlock {
  type: "thinking"
  thinking: string
}

export interface UsageStats {
  input_tokens: number
  output_tokens: number
  cache_read_input_tokens?: number
  cache_creation_input_tokens?: number
}

export interface ConversationState {
  messages: AgentMessage[]
  isProcessing: boolean
  focusState: "input" | "history"
  totalTokens: {
    input: number
    output: number
  }
}
