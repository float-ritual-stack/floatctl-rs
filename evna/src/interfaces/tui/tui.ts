/**
 * EVNA Chat TUI - Fully Featured Agentic Chat Interface
 *
 * Features:
 * - Multi-line input with word navigation, selection, clipboard, undo/redo
 * - Markdown rendering with code blocks and syntax highlighting
 * - Tool call visualization with collapsible results
 * - Session persistence with auto-save
 * - Keyboard shortcuts (Ctrl+H for help)
 * - Token usage and cost tracking
 * - Message timestamps and compact mode
 * - Input history navigation
 *
 * Built with OpenTUI (@opentui/core)
 */

// Load .env with fallback chain: ./.env → ~/.floatctl/.env → existing env vars
import { loadEnvWithFallback } from "../../lib/env-loader.js"
loadEnvWithFallback()

import { createCliRenderer, ConsolePosition } from "@opentui/core"
import { query, type SDKUserMessage } from "@anthropic-ai/claude-agent-sdk"
import { ConversationLoop } from "./components/ConversationLoop.js"
import type { AgentMessage, ContentBlock } from "./types.js"

// Import shared config and MCP server
import { evnaNextMcpServer } from "../mcp.js"
import { createQueryOptions, DEFAULT_MODEL } from "../../core/config.js"

// ============================================================================
// Response Transformer
// ============================================================================

/**
 * Transform Agent SDK response to our AgentMessage format
 */
function transformResponse(response: any): AgentMessage {
  // Handle already-formatted messages
  if (response.id && response.role && response.content) {
    return {
      ...response,
      timestamp: response.timestamp ?? Date.now(),
    }
  }

  // Transform Agent SDK message format
  const content: ContentBlock[] = []

  if (response.content && Array.isArray(response.content)) {
    for (const block of response.content) {
      if (block.type === "text") {
        content.push({ type: "text", text: block.text })
      } else if (block.type === "tool_use") {
        content.push({
          type: "tool_use",
          id: block.id,
          name: block.name,
          input: block.input,
        })
      } else if (block.type === "tool_result") {
        content.push({
          type: "tool_result",
          tool_use_id: block.tool_use_id,
          content: block.content,
          is_error: block.is_error ?? false,
        })
      } else if (block.type === "thinking") {
        content.push({
          type: "thinking",
          thinking: block.thinking,
        })
      }
    }
  } else if (typeof response.content === "string") {
    content.push({ type: "text", text: response.content })
  }

  return {
    id: response.id ?? `msg_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
    role: response.role ?? "assistant",
    content,
    usage: response.usage,
    stop_reason: response.stop_reason,
    model: response.model,
    timestamp: Date.now(),
  }
}

// ============================================================================
// Main Entry Point
// ============================================================================

async function main() {
  // Startup banner (goes to stderr so it doesn't interfere with TUI)
  console.error("🧠 EVNA Chat TUI")
  console.error("================")
  console.error("Fully featured agentic chat interface")
  console.error("")
  console.error("⌨️  Controls:")
  console.error("   ESC or Ctrl+Enter = Submit message")
  console.error("   Ctrl+H = Show help")
  console.error("   Ctrl+L = Clear conversation")
  console.error("   Ctrl+S = Save session")
  console.error("   Ctrl+C = Exit")
  console.error("")

  // Create OpenTUI renderer
  const renderer = await createCliRenderer({
    exitOnCtrlC: true,
    consoleOptions: {
      position: ConsolePosition.BOTTOM,
      sizePercent: 25,
      colorInfo: "#00FFFF",
      colorWarn: "#FFFF00",
      colorError: "#FF0000",
      startInDebugMode: false,
    },
  })

  console.error("✅ Renderer initialized")
  console.error(`📋 Model: ${DEFAULT_MODEL}`)
  console.error("🔧 Tools: brain_boot, semantic_search, active_context, ask_evna")
  console.error("")

  // Create conversation loop with Agent SDK integration
  const loop = new ConversationLoop(renderer, {
    model: DEFAULT_MODEL,
    enableAutoSave: true,
    showTimestamps: false,
    compactMode: false,

    onSubmit: async (userInput: string): Promise<AgentMessage> => {
      console.log(`[TUI] User input received (${userInput.length} chars)`)

      // Convert user input to async generator (Agent SDK pattern)
      async function* generateMessages(): AsyncGenerator<SDKUserMessage> {
        yield {
          type: "user" as const,
          session_id: "",
          message: {
            role: "user" as const,
            content: userInput,
          },
          parent_tool_use_id: null,
        }
      }

      try {
        // Query Agent SDK with evna tools (using shared config)
        const result = await query({
          prompt: generateMessages(),
          options: createQueryOptions(evnaNextMcpServer),
        })

        // Collect all messages from async iterator
        const messages: any[] = []
        for await (const message of result) {
          console.log(`[TUI] Message received: ${message.type}`)
          messages.push(message)
        }

        // Find assistant response (last message with role=assistant)
        const assistantMessage = messages
          .filter((m) => m.type === "assistant" && m.message?.role === "assistant")
          .pop()

        if (assistantMessage?.message) {
          return transformResponse(assistantMessage.message)
        }

        // Log for debugging
        console.error("[TUI] No assistant message found in response stream")
        console.error("[TUI] Received message types:", messages.map((m) => m.type).join(", "))

        // Check for tool errors
        const toolErrors = messages.filter((m) => m.type === "tool_result" && m.is_error)
        if (toolErrors.length > 0) {
          console.error("[TUI] Tool errors detected:", toolErrors)
          return {
            id: `msg_${Date.now()}`,
            role: "assistant",
            content: [
              {
                type: "text",
                text: `**Tool Execution Failed**\n\n${toolErrors.map((e) => `- ${e.content}`).join("\n")}\n\nPlease check your query and try again.`,
              },
            ],
            timestamp: Date.now(),
          }
        }

        // Unexpected case - protocol/SDK issue
        throw new Error(
          `Agent SDK returned no assistant message. This is unexpected.\n` +
            `Received ${messages.length} messages of types: ${messages.map((m) => m.type).join(", ")}`
        )
      } catch (error) {
        console.error("[TUI] Query error:", error)
        throw error
      }
    },

    formatMessage: (response: any): AgentMessage => {
      return transformResponse(response)
    },
  })

  // Add the conversation loop to the renderer
  renderer.root.add(loop)

  // Handle cleanup on exit
  process.on("SIGINT", () => {
    console.error("\n👋 Saving session and exiting...")
    loop.destroy()
    process.exit(0)
  })

  process.on("SIGTERM", () => {
    loop.destroy()
    process.exit(0)
  })

  // Start the render loop
  renderer.start()
}

// Run the TUI
main().catch((error) => {
  console.error("Fatal error:", error)
  process.exit(1)
})
