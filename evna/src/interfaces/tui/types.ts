/**
 * Type definitions for EVNA Chat TUI
 * Comprehensive types for streaming, sessions, and rich message rendering
 */

// ============================================================================
// Cursor & Selection
// ============================================================================

export interface CursorPosition {
  line: number
  col: number
}

export interface TextSelection {
  start: CursorPosition
  end: CursorPosition
  active: boolean
}

// ============================================================================
// Message Content Blocks
// ============================================================================

export interface TextBlock {
  type: "text"
  text: string
}

export interface ToolUseBlock {
  type: "tool_use"
  id: string
  name: string
  input: Record<string, unknown>
}

export interface ToolResultBlock {
  type: "tool_result"
  tool_use_id: string
  content: unknown
  is_error: boolean
}

export interface ThinkingBlock {
  type: "thinking"
  thinking: string
}

export interface StreamingTextBlock {
  type: "streaming_text"
  text: string
  complete: boolean
}

export type ContentBlock =
  | TextBlock
  | ToolUseBlock
  | ToolResultBlock
  | ThinkingBlock
  | StreamingTextBlock

// ============================================================================
// Messages
// ============================================================================

export type MessageRole = "user" | "assistant" | "system" | "tool"

export interface UsageStats {
  input_tokens: number
  output_tokens: number
  cache_read_input_tokens?: number
  cache_creation_input_tokens?: number
}

export interface AgentMessage {
  id: string
  role: MessageRole
  content: ContentBlock[]
  usage?: UsageStats
  stop_reason?: string
  model?: string
  timestamp: number
  collapsed?: boolean  // For tool results
}

// ============================================================================
// Streaming State
// ============================================================================

export interface StreamingState {
  active: boolean
  currentMessageId: string | null
  accumulatedText: string
  toolCalls: ToolUseBlock[]
  thinkingText: string
  startTime: number
  tokensReceived: number
}

// ============================================================================
// Session & Persistence
// ============================================================================

export interface Session {
  id: string
  name: string
  messages: AgentMessage[]
  createdAt: number
  updatedAt: number
  totalTokens: TokenStats
  metadata?: Record<string, unknown>
}

export interface TokenStats {
  input: number
  output: number
  cached: number
}

export interface SessionSummary {
  id: string
  name: string
  messageCount: number
  lastMessage: string
  updatedAt: number
}

// ============================================================================
// UI State
// ============================================================================

export type FocusState = "input" | "history" | "help" | "command" | "search"

export interface ScrollState {
  offset: number
  maxOffset: number
  viewportHeight: number
  totalHeight: number
}

export interface ConversationState {
  messages: AgentMessage[]
  streaming: StreamingState
  focusState: FocusState
  scrollState: ScrollState
  totalTokens: TokenStats
  currentSession: Session | null
  searchQuery: string
  searchResults: number[]  // Message indices matching search
  commandBuffer: string
  showHelp: boolean
  showTimestamps: boolean
  compactMode: boolean
  error: string | null
}

// ============================================================================
// Theme & Styling
// ============================================================================

export interface ThemeColors {
  // Backgrounds
  bgPrimary: string
  bgSecondary: string
  bgHighlight: string
  bgError: string
  bgSuccess: string
  bgWarning: string

  // Text
  textPrimary: string
  textSecondary: string
  textMuted: string

  // Roles
  roleUser: string
  roleAssistant: string
  roleSystem: string
  roleTool: string

  // UI Elements
  border: string
  borderFocused: string
  cursor: string
  selection: string

  // Status
  statusReady: string
  statusThinking: string
  statusError: string
  statusStreaming: string
}

export const DEFAULT_THEME: ThemeColors = {
  bgPrimary: "#1a1a2e",
  bgSecondary: "#16213e",
  bgHighlight: "#0f3460",
  bgError: "#2a1a1a",
  bgSuccess: "#1a2a1a",
  bgWarning: "#2a2a1a",

  textPrimary: "#eaeaea",
  textSecondary: "#b0b0b0",
  textMuted: "#666666",

  roleUser: "#00ff88",
  roleAssistant: "#00aaff",
  roleSystem: "#ffaa00",
  roleTool: "#ff66ff",

  border: "#404040",
  borderFocused: "#00ff88",
  cursor: "#00ff88",
  selection: "#3a3a5e",

  statusReady: "#00ff88",
  statusThinking: "#ffaa00",
  statusError: "#ff4444",
  statusStreaming: "#00aaff",
}

// ============================================================================
// Keyboard Shortcuts
// ============================================================================

export interface KeyboardShortcut {
  key: string
  ctrl?: boolean
  shift?: boolean
  meta?: boolean
  alt?: boolean
  description: string
  action: string
}

export const KEYBOARD_SHORTCUTS: KeyboardShortcut[] = [
  // Input shortcuts
  { key: "return", ctrl: true, description: "Submit message", action: "submit" },
  { key: "escape", description: "Toggle focus / Submit", action: "toggle_focus" },
  { key: "d", ctrl: true, description: "Submit message", action: "submit" },

  // Navigation
  { key: "up", description: "Move cursor up / Scroll history", action: "navigate_up" },
  { key: "down", description: "Move cursor down / Scroll history", action: "navigate_down" },
  { key: "pageup", description: "Page up in history", action: "page_up" },
  { key: "pagedown", description: "Page down in history", action: "page_down" },
  { key: "home", ctrl: true, description: "Scroll to top", action: "scroll_top" },
  { key: "end", ctrl: true, description: "Scroll to bottom", action: "scroll_bottom" },

  // Actions
  { key: "l", ctrl: true, description: "Clear conversation", action: "clear" },
  { key: "r", ctrl: true, description: "Search messages", action: "search" },
  { key: "s", ctrl: true, description: "Save session", action: "save" },
  { key: "o", ctrl: true, description: "Load session", action: "load" },
  { key: "n", ctrl: true, description: "New session", action: "new_session" },
  { key: "h", ctrl: true, description: "Toggle help", action: "toggle_help" },
  { key: "t", ctrl: true, description: "Toggle timestamps", action: "toggle_timestamps" },
  { key: "m", ctrl: true, description: "Toggle compact mode", action: "toggle_compact" },

  // Input editing
  { key: "a", ctrl: true, description: "Move to line start", action: "line_start" },
  { key: "e", ctrl: true, description: "Move to line end", action: "line_end" },
  { key: "k", ctrl: true, description: "Delete to end of line", action: "kill_line" },
  { key: "u", ctrl: true, description: "Delete entire line", action: "kill_full_line" },
  { key: "w", ctrl: true, description: "Delete word backward", action: "delete_word" },
  { key: "left", ctrl: true, description: "Move word left", action: "word_left" },
  { key: "right", ctrl: true, description: "Move word right", action: "word_right" },

  // Exit
  { key: "c", ctrl: true, description: "Exit application", action: "exit" },
]

// ============================================================================
// Command System
// ============================================================================

export interface Command {
  name: string
  aliases: string[]
  description: string
  usage: string
  execute: (args: string[]) => Promise<void> | void
}

// ============================================================================
// Events
// ============================================================================

export interface ChatEvents {
  submit: (text: string) => void
  clear: () => void
  scroll: (direction: "up" | "down", amount: number) => void
  focusChange: (state: FocusState) => void
  sessionChange: (session: Session | null) => void
  error: (error: Error) => void
  streamStart: () => void
  streamChunk: (chunk: string) => void
  streamEnd: (message: AgentMessage) => void
}

// ============================================================================
// Utility Types
// ============================================================================

export function createEmptyState(): ConversationState {
  return {
    messages: [],
    streaming: {
      active: false,
      currentMessageId: null,
      accumulatedText: "",
      toolCalls: [],
      thinkingText: "",
      startTime: 0,
      tokensReceived: 0,
    },
    focusState: "input",
    scrollState: {
      offset: 0,
      maxOffset: 0,
      viewportHeight: 0,
      totalHeight: 0,
    },
    totalTokens: { input: 0, output: 0, cached: 0 },
    currentSession: null,
    searchQuery: "",
    searchResults: [],
    commandBuffer: "",
    showHelp: false,
    showTimestamps: false,
    compactMode: false,
    error: null,
  }
}

export function generateMessageId(): string {
  return `msg_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`
}

export function generateSessionId(): string {
  return `session_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`
}
