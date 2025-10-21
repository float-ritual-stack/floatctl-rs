/**
 * Conversation loop orchestrator
 * Manages the full REPL experience: input, message rendering, focus states
 */

import {
  BoxRenderable,
  TextRenderable,
  type CliRenderer,
  type KeyEvent,
  RGBA,
  TextAttributes,
  t,
  bold,
  fg,
} from "@opentui/core"
import { MultilineInput } from "./MultilineInput"
import { MessageRenderer } from "./MessageRenderer"
import type { AgentMessage, ConversationState } from "../types"

export interface ConversationLoopOptions {
  onSubmit: (userInput: string) => Promise<any>
  formatMessage?: (msg: any) => AgentMessage
  enableConsole?: boolean
}

export class ConversationLoop extends BoxRenderable {
  private state: ConversationState = {
    messages: [],
    isProcessing: false,
    focusState: "input",
    totalTokens: { input: 0, output: 0 },
  }

  private input: MultilineInput
  private history: MessageRenderer
  private statusBar: TextRenderable
  private keyHandler: ((key: KeyEvent) => void) | null = null

  private options: ConversationLoopOptions

  constructor(
    renderer: CliRenderer,
    options: ConversationLoopOptions
  ) {
    super(renderer, {
      id: "conversation-loop",
      width: "100%",
      height: "100%",
      flexDirection: "column",
    })

    this.options = options

    // Setup UI components
    this.setupUI()
    this.setupKeyHandler()
    this.setupEventHandlers()
  }

  private setupUI(): void {
    // Message history (top 70%)
    this.history = new MessageRenderer(this.ctx, {
      id: "history",
      width: "100%",
    })
    this.history.flexGrow = 7

    // Input box (bottom 30%)
    this.input = new MultilineInput(this.ctx, {
      id: "input",
      width: "100%",
      height: 12,
      placeholder: "Enter your message... (Ctrl+Enter to submit, Escape to toggle focus)",
      backgroundColor: "#2a2a2a",
      borderColor: "#555555",
    })
    this.input.flexGrow = 3

    // Status bar (fixed bottom)
    this.statusBar = new TextRenderable(this.ctx, {
      id: "status-bar",
      content: this.getStatusText(),
      position: "relative",
      height: 1,
      backgroundColor: "#1a1a1a",
      fg: "#00FF00",
      paddingLeft: 1,
      attributes: TextAttributes.BOLD,
    })

    this.add(this.history)
    this.add(this.input)
    this.add(this.statusBar)

    // Focus input by default
    this.input.focus()
  }

  private setupKeyHandler(): void {
    this.keyHandler = (key: KeyEvent) => {
      // Toggle focus with Escape
      if (key.name === "escape") {
        this.toggleFocus()
        return
      }

      // Clear conversation with Ctrl+L
      if (key.ctrl && key.name === "l") {
        this.clear()
        return
      }

      // Toggle console with backtick
      if (key.sequence === "`" && this.options.enableConsole !== false) {
        this.ctx.console.toggle()
        return
      }

      // Scroll history when focused
      if (this.state.focusState === "history") {
        if (key.name === "up") {
          // TODO: Implement history scroll
        }
        if (key.name === "down") {
          // TODO: Implement history scroll
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

  private async handleSubmit(userInput: string): Promise<void> {
    if (this.state.isProcessing) {
      console.warn("Already processing a message...")
      return
    }

    if (!userInput.trim()) {
      return
    }

    this.state.isProcessing = true
    this.updateStatus("🤔 Thinking...")

    // Add user message to history
    const userMessage: AgentMessage = {
      role: "user",
      content: [{ type: "text", text: userInput }],
    }
    this.state.messages.push(userMessage)
    this.history.addMessage(userMessage)

    // Clear input
    this.input.clear()

    try {
      // Call Agent SDK
      const response = await this.options.onSubmit(userInput)

      // Format response if formatter provided
      const formattedResponse = this.options.formatMessage
        ? this.options.formatMessage(response)
        : response

      // Add to history
      this.state.messages.push(formattedResponse)
      this.history.addMessage(formattedResponse)

      // Update token counts
      if (formattedResponse.usage) {
        this.state.totalTokens.input += formattedResponse.usage.input_tokens
        this.state.totalTokens.output += formattedResponse.usage.output_tokens
      }

      this.updateStatus("✅ Ready")
    } catch (error) {
      console.error("Query failed:", error)
      this.history.addError(
        error instanceof Error ? error.message : String(error),
        "error"
      )
      this.updateStatus("❌ Error")
    } finally {
      this.state.isProcessing = false
      this.updateStatus("Ready")

      // Re-focus input
      this.input.focus()
      this.history.scrollToBottom()
    }
  }

  private toggleFocus(): void {
    if (this.state.focusState === "input") {
      this.input.blur()
      // TODO: Implement history focus
      this.state.focusState = "history"
      this.updateStatus("📜 History focused (Escape to return to input)")
    } else {
      this.input.focus()
      this.state.focusState = "input"
      this.updateStatus("⌨️  Input focused")
    }
  }

  private updateStatus(message: string): void {
    this.statusBar.content = this.getStatusText(message)
  }

  private getStatusText(message?: string): string {
    const status = message || "Ready"
    const tokens = this.state.totalTokens
    const cost = this.calculateCost(tokens.input, tokens.output)

    return t`${fg("#00FF00")(status)} | ${fg("#FFAA00")(`Tokens: ${tokens.input}↑ ${tokens.output}↓`)} | ${fg("#FF00FF")(`Cost: $${cost.toFixed(4)}`)}`
  }

  private calculateCost(inputTokens: number, outputTokens: number): number {
    // Sonnet 4.5 pricing (example - adjust for your model)
    const inputCost = (inputTokens / 1_000_000) * 3.0
    const outputCost = (outputTokens / 1_000_000) * 15.0
    return inputCost + outputCost
  }

  public clear(): void {
    this.state.messages = []
    this.state.totalTokens = { input: 0, output: 0 }
    this.history.clear()
    this.input.clear()
    this.updateStatus("🗑️  Conversation cleared")

    setTimeout(() => {
      this.updateStatus("Ready")
    }, 2000)
  }

  public getState(): ConversationState {
    return { ...this.state }
  }

  public destroy(): void {
    if (this.keyHandler) {
      this.ctx.keyInput.off("keypress", this.keyHandler)
      this.keyHandler = null
    }
    this.input.destroy()
    super.destroy()
  }
}
