/**
 * Conversation Loop - Main Chat Orchestrator
 * Manages the full chat experience: input, messages, status, help, and sessions
 */

import {
  BoxRenderable,
  TextRenderable,
  type CliRenderer,
  type KeyEvent,
  t,
  bold,
  fg,
} from "@opentui/core"
import { MultilineInput } from "./MultilineInput.js"
import { MessageRenderer } from "./MessageRenderer.js"
import { StatusBar } from "./StatusBar.js"
import { HelpOverlay } from "./HelpOverlay.js"
import { SessionManager } from "./SessionManager.js"
import type {
  AgentMessage,
  ConversationState,
  FocusState,
  Session,
} from "../types.js"

// ============================================================================
// Conversation Loop Options
// ============================================================================

export interface ConversationLoopOptions {
  onSubmit: (userInput: string) => Promise<AgentMessage>
  formatMessage?: (msg: unknown) => AgentMessage
  model?: string
  enableAutoSave?: boolean
  showTimestamps?: boolean
  compactMode?: boolean
}

// ============================================================================
// Conversation Loop Component
// ============================================================================

export class ConversationLoop extends BoxRenderable {
  private state: ConversationState
  private options: ConversationLoopOptions

  // UI Components
  private input!: MultilineInput
  private history!: MessageRenderer
  private statusBar!: StatusBar
  private helpOverlay!: HelpOverlay

  // Managers
  private sessionManager: SessionManager

  // Event handlers
  private keyHandler: ((key: KeyEvent) => void) | null = null
  private statusResetTimer: ReturnType<typeof setTimeout> | null = null

  constructor(renderer: CliRenderer, options: ConversationLoopOptions) {
    super(renderer, {
      id: "conversation-loop",
      width: "100%",
      height: "100%",
      flexDirection: "column",
      backgroundColor: "#0d0d1a",
    })

    this.options = options
    this.sessionManager = new SessionManager()

    // Initialize state
    this.state = {
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
      currentSession: this.sessionManager.createSession(),
      searchQuery: "",
      searchResults: [],
      commandBuffer: "",
      showHelp: false,
      showTimestamps: options.showTimestamps ?? false,
      compactMode: options.compactMode ?? false,
      error: null,
    }

    this.setupUI()
    this.setupKeyHandler()
    this.setupEventHandlers()

    // Enable auto-save if configured
    if (options.enableAutoSave !== false && this.state.currentSession) {
      this.sessionManager.enableAutoSave(this.state.currentSession, 60000) // Every minute
    }
  }

  private setupUI(): void {
    // Header with title
    const header = new TextRenderable(this.ctx, {
      id: "header",
      content: t`${bold(fg("#00ff88")("🧠 EVNA Chat"))} ${fg("#606080")("─ Agentic AI Assistant")}`,
      position: "relative",
      paddingLeft: 1,
      paddingTop: 0,
      paddingBottom: 0,
      height: 1,
    })

    // Message history (main area)
    this.history = new MessageRenderer(this.ctx, {
      id: "history",
      width: "100%",
      showTimestamps: this.state.showTimestamps,
      compactMode: this.state.compactMode,
    })
    this.history.flexGrow = 7

    // Input area
    this.input = new MultilineInput(this.ctx, {
      id: "input",
      width: "100%",
      height: 8,
      placeholder: "Type your message... (Ctrl+Enter to submit, F1 for help)",
    })
    this.input.flexGrow = 2

    // Status bar
    this.statusBar = new StatusBar(this.ctx, {
      id: "status-bar",
      model: this.options.model,
    })

    // Help overlay (absolute positioned, hidden by default)
    this.helpOverlay = new HelpOverlay(this.ctx, {
      id: "help-overlay",
    })

    // Add components in order
    this.add(header)
    this.add(this.history)
    this.add(this.input)
    this.add(this.statusBar)
    this.add(this.helpOverlay)

    // Focus input by default
    this.input.focus()
  }

  private setupKeyHandler(): void {
    this.keyHandler = (key: KeyEvent) => {
      // Don't process if help is visible (it handles its own keys)
      if (this.helpOverlay.isVisible()) {
        return
      }

      // Global shortcuts (work regardless of focus)

      // F1 = Toggle help (Ctrl+H is backspace in terminals)
      if (key.name === "f1") {
        this.toggleHelp()
        return
      }

      // Ctrl+L = Clear conversation
      if (key.ctrl && key.name === "l") {
        this.clear()
        return
      }

      // Ctrl+S = Save session
      if (key.ctrl && key.name === "s") {
        this.saveSession()
        return
      }

      // Ctrl+N = New session
      if (key.ctrl && key.name === "n") {
        this.newSession()
        return
      }

      // Ctrl+T = Toggle timestamps
      if (key.ctrl && key.name === "t") {
        this.toggleTimestamps()
        return
      }

      // Ctrl+M = Toggle compact mode (but not during processing)
      if (key.ctrl && key.name === "m" && !this.state.streaming.active) {
        this.toggleCompactMode()
        return
      }

      // Focus-specific shortcuts

      // Escape when in history = return to input
      if (key.name === "escape" && this.state.focusState === "history") {
        this.setFocus("input")
        return
      }

      // History scrolling when history is focused
      if (this.state.focusState === "history") {
        if (key.name === "up" || key.name === "pageup") {
          this.scrollHistory("up", key.name === "pageup" ? 10 : 1)
          return
        }
        if (key.name === "down" || key.name === "pagedown") {
          this.scrollHistory("down", key.name === "pagedown" ? 10 : 1)
          return
        }
        if (key.ctrl && key.name === "home") {
          this.scrollToTop()
          return
        }
        if (key.ctrl && key.name === "end") {
          this.scrollToBottom()
          return
        }
      }
    }

    this.ctx.keyInput.on("keypress", this.keyHandler)
  }

  private setupEventHandlers(): void {
    // Handle input submission
    this.input.on("submit", async (value: string) => {
      await this.handleSubmit(value)
    })
  }

  // === Message Handling ===

  private async handleSubmit(userInput: string): Promise<void> {
    if (this.state.streaming.active) {
      console.warn("Already processing a message...")
      return
    }

    const trimmedInput = userInput.trim()
    if (!trimmedInput) {
      return
    }

    // Check for slash commands
    if (trimmedInput.startsWith("/")) {
      await this.handleCommand(trimmedInput)
      return
    }

    // Create user message
    const userMessage: AgentMessage = {
      id: `msg_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
      role: "user",
      content: [{ type: "text", text: trimmedInput }],
      timestamp: Date.now(),
    }

    // Add to state and UI
    this.state.messages.push(userMessage)
    this.history.addMessage(userMessage)

    // Update session
    if (this.state.currentSession) {
      this.state.currentSession = this.sessionManager.addMessageToSession(
        this.state.currentSession,
        userMessage
      )
      this.sessionManager.updatePendingSession(this.state.currentSession)
    }

    // Clear input
    this.input.clear()

    // Update status
    this.state.streaming.active = true
    this.state.streaming.startTime = Date.now()
    this.statusBar.setStatus("Thinking...")

    try {
      // Call the LLM
      const response = await this.options.onSubmit(trimmedInput)

      // Format response if formatter provided
      const formattedResponse = this.options.formatMessage
        ? this.options.formatMessage(response)
        : response

      // Ensure the response has an ID and timestamp
      if (!formattedResponse.id) {
        formattedResponse.id = `msg_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`
      }
      if (!formattedResponse.timestamp) {
        formattedResponse.timestamp = Date.now()
      }

      // Add to state and UI
      this.state.messages.push(formattedResponse)
      this.history.addMessage(formattedResponse)

      // Update token counts
      if (formattedResponse.usage) {
        this.state.totalTokens.input += formattedResponse.usage.input_tokens
        this.state.totalTokens.output += formattedResponse.usage.output_tokens
        if (formattedResponse.usage.cache_read_input_tokens) {
          this.state.totalTokens.cached += formattedResponse.usage.cache_read_input_tokens
        }
        this.statusBar.setTokens(this.state.totalTokens)
      }

      // Update session
      if (this.state.currentSession) {
        this.state.currentSession = this.sessionManager.addMessageToSession(
          this.state.currentSession,
          formattedResponse
        )
        if (formattedResponse.usage) {
          this.state.currentSession = this.sessionManager.updateSessionTokens(
            this.state.currentSession,
            {
              input: formattedResponse.usage.input_tokens,
              output: formattedResponse.usage.output_tokens,
              cached: formattedResponse.usage.cache_read_input_tokens ?? 0,
            }
          )
        }
        this.sessionManager.updatePendingSession(this.state.currentSession)
      }

      this.statusBar.setStatus("Ready")
      this.state.error = null

    } catch (error) {
      console.error("Query failed:", error)
      const errorMessage = error instanceof Error ? error.message : String(error)
      this.history.addError(errorMessage, "error")
      this.statusBar.setStatus("Error")
      this.state.error = errorMessage
    } finally {
      this.state.streaming.active = false
      this.input.focus()
      this.history.scrollToBottom()
    }
  }

  // === Command Handling ===

  private async handleCommand(input: string): Promise<void> {
    const parts = input.slice(1).split(/\s+/)
    const command = parts[0].toLowerCase()
    const args = parts.slice(1)

    switch (command) {
      case "help":
      case "h":
        this.toggleHelp()
        break

      case "clear":
      case "c":
        this.clear()
        break

      case "save":
      case "s":
        this.saveSession()
        break

      case "load":
      case "l":
        await this.loadSession(args[0])
        break

      case "new":
      case "n":
        this.newSession()
        break

      case "sessions":
      case "list":
        this.listSessions()
        break

      case "timestamps":
      case "ts":
        this.toggleTimestamps()
        break

      case "compact":
        this.toggleCompactMode()
        break

      case "model":
        if (args[0]) {
          this.statusBar.setModel(args[0])
        }
        break

      default:
        this.history.addError(`Unknown command: /${command}\nType /help for available commands.`, "warning")
    }

    this.input.clear()
  }

  // === Focus Management ===

  private setFocus(focus: FocusState): void {
    this.state.focusState = focus

    if (focus === "input") {
      this.input.focus()
      this.statusBar.setFocus("input")
    } else if (focus === "history") {
      this.input.blur()
      this.statusBar.setFocus("history")
    }

    this.markDirty()
  }

  private toggleFocus(): void {
    if (this.state.focusState === "input") {
      this.setFocus("history")
    } else {
      this.setFocus("input")
    }
  }

  // === Scrolling ===

  private scrollHistory(direction: "up" | "down", amount: number): void {
    // Use ScrollBoxRenderable's scrollBy method
    const delta = direction === "up" ? -amount : amount
    this.history.scrollBy(delta)
  }

  private scrollToTop(): void {
    // Scroll to top using ScrollBoxRenderable's scrollTo
    this.history.scrollTo(0)
  }

  private scrollToBottom(): void {
    this.history.scrollToBottom()
  }

  // === Status Helper ===

  private setStatusWithReset(message: string, delay: number = 2000): void {
    if (this.statusResetTimer) {
      clearTimeout(this.statusResetTimer)
    }
    this.statusBar.setStatus(message)
    this.statusResetTimer = setTimeout(() => {
      this.statusBar.setStatus("Ready")
      this.statusResetTimer = null
    }, delay)
  }

  // === Session Management ===

  private saveSession(): void {
    if (this.state.currentSession && this.state.messages.length > 0) {
      this.sessionManager.saveSession(this.state.currentSession)
      this.setStatusWithReset("Session saved")
    } else {
      this.setStatusWithReset("Nothing to save")
    }
  }

  private async loadSession(sessionId?: string): Promise<void> {
    if (!sessionId) {
      // Show recent sessions
      this.listSessions()
      return
    }

    const session = this.sessionManager.loadSession(sessionId)
    if (session) {
      this.state.currentSession = session
      this.state.messages = session.messages
      this.state.totalTokens = session.totalTokens

      // Rebuild history UI
      this.history.clear()
      for (const msg of session.messages) {
        this.history.addMessage(msg)
      }

      this.statusBar.setTokens(session.totalTokens)
      this.setStatusWithReset(`Loaded: ${session.name}`)
    } else {
      this.history.addError(`Session not found: ${sessionId}`, "error")
    }
  }

  private newSession(): void {
    // Save current session if it has messages
    if (this.state.currentSession && this.state.messages.length > 0) {
      this.sessionManager.saveSession(this.state.currentSession)
    }

    // Create new session
    this.state.currentSession = this.sessionManager.createSession()
    this.state.messages = []
    this.state.totalTokens = { input: 0, output: 0, cached: 0 }
    this.state.error = null

    // Clear UI
    this.history.clear()
    this.statusBar.reset()

    // Update auto-save
    if (this.state.currentSession) {
      this.sessionManager.updatePendingSession(this.state.currentSession)
    }

    this.setStatusWithReset("New session")
  }

  private listSessions(): void {
    const sessions = this.sessionManager.getRecentSessions(10)

    if (sessions.length === 0) {
      this.history.addError("No saved sessions found.", "warning")
      return
    }

    let listText = "Recent Sessions:\n"
    for (const session of sessions) {
      const date = new Date(session.updatedAt).toLocaleString()
      listText += `\n  ${session.id.slice(0, 20)}...\n`
      listText += `    ${session.name} (${session.messageCount} messages)\n`
      listText += `    ${date}\n`
      listText += `    "${session.lastMessage}"\n`
    }
    listText += "\nUse /load <session_id> to load a session"

    // Add as a system message
    const infoMessage: AgentMessage = {
      id: `msg_${Date.now()}`,
      role: "system",
      content: [{ type: "text", text: listText }],
      timestamp: Date.now(),
    }
    this.history.addMessage(infoMessage)
  }

  // === UI Toggles ===

  private toggleHelp(): void {
    if (this.helpOverlay.isVisible()) {
      this.helpOverlay.hide()
      this.setFocus("input")
    } else {
      this.helpOverlay.show(() => {
        this.setFocus("input")
      })
      this.state.focusState = "help"
    }
  }

  private toggleTimestamps(): void {
    this.state.showTimestamps = !this.state.showTimestamps
    this.history.setShowTimestamps(this.state.showTimestamps)

    const status = this.state.showTimestamps ? "Timestamps ON" : "Timestamps OFF"
    this.setStatusWithReset(status)
  }

  private toggleCompactMode(): void {
    this.state.compactMode = !this.state.compactMode
    this.history.setCompactMode(this.state.compactMode)

    const status = this.state.compactMode ? "Compact mode ON" : "Compact mode OFF"
    this.setStatusWithReset(status)
  }

  // === Clear ===

  public clear(): void {
    this.state.messages = []
    this.state.totalTokens = { input: 0, output: 0, cached: 0 }
    this.state.error = null

    this.history.clear()
    this.input.clear()
    this.statusBar.reset()

    // Update session
    if (this.state.currentSession) {
      this.state.currentSession = this.sessionManager.clearSessionMessages(this.state.currentSession)
      this.sessionManager.updatePendingSession(this.state.currentSession)
    }

    this.setStatusWithReset("Cleared")
  }

  // === Public API ===

  public getState(): ConversationState {
    return { ...this.state }
  }

  public getMessages(): AgentMessage[] {
    return [...this.state.messages]
  }

  public getCurrentSession(): Session | null {
    return this.state.currentSession
  }

  public addSystemMessage(text: string): void {
    const message: AgentMessage = {
      id: `msg_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
      role: "system",
      content: [{ type: "text", text }],
      timestamp: Date.now(),
    }
    this.state.messages.push(message)
    this.history.addMessage(message)
  }

  public setModel(model: string): void {
    this.statusBar.setModel(model)
  }

  public destroy(): void {
    // Save session before exiting
    if (this.state.currentSession && this.state.messages.length > 0) {
      this.sessionManager.saveSession(this.state.currentSession)
    }

    // Cleanup timers
    if (this.statusResetTimer) {
      clearTimeout(this.statusResetTimer)
      this.statusResetTimer = null
    }

    // Cleanup managers
    this.sessionManager.destroy()

    if (this.keyHandler) {
      this.ctx.keyInput.off("keypress", this.keyHandler)
      this.keyHandler = null
    }

    this.input.destroy()
    this.helpOverlay.destroy()

    super.destroy()
  }
}
