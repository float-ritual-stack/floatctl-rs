/**
 * EVNA-Next TUI: Interactive chat loop with OpenTUI
 * Real Agent SDK integration with brain_boot, semantic_search, active_context
 */

// Load .env with fallback chain: ./.env → ~/.floatctl/.env → existing env vars
import { loadEnvWithFallback } from "../../lib/env-loader.js";
loadEnvWithFallback();
import { createCliRenderer, ConsolePosition } from "@opentui/core"
import { query, type SDKUserMessage } from "@anthropic-ai/claude-agent-sdk"
import { ConversationLoop } from "./components/ConversationLoop.js"
import type { AgentMessage } from "./types.js"

// Import shared config and MCP server
import { evnaNextMcpServer } from "../mcp.js"
import { createQueryOptions } from "../../core/config.js"

async function main() {
  console.log("🧠 EVNA-Next TUI")
  console.log("================")
  console.log("Interactive chat loop with brain_boot, semantic_search, active_context")
  console.log()
  console.log("⌨️  HOW TO SUBMIT:")
  console.log("   1. Type your message (Enter = newline, Tab = indent)")
  console.log("   2. Press ESC to submit")
  console.log("   (or try numpad Enter, or Ctrl+D)")
  console.log()
  console.log("📝 Debug: tail -f /tmp/evna-keys.log")
  console.log()

  // Create OpenTUI renderer
  const renderer = await createCliRenderer({
    exitOnCtrlC: true,
    consoleOptions: {
      position: ConsolePosition.BOTTOM,
      sizePercent: 30,
      colorInfo: "#00FFFF",
      colorWarn: "#FFFF00",
      colorError: "#FF0000",
      startInDebugMode: false,
    },
  })

  console.log("✅ Renderer initialized")
  console.log("📋 Tools: brain_boot, semantic_search, active_context")
  console.log("⌨️  Press ` to toggle console, Ctrl+C to exit")
  console.log()

  // Create conversation loop
  const loop = new ConversationLoop(renderer, {
    onSubmit: async (userInput: string) => {
      console.log(`[TUI] User input received (${userInput.length} chars)`)

      // Convert user input to async generator (Agent SDK pattern)
      async function* generateMessages(): AsyncGenerator<SDKUserMessage> {
        yield {
          type: "user" as const,
          session_id: "", // Filled by SDK
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
          return assistantMessage.message
        }

        // Log the full message stream for debugging
        console.error("[TUI] No assistant message found in response stream")
        console.error("[TUI] Received message types:", messages.map(m => m.type).join(', '))

        // Check for tool errors
        const toolErrors = messages.filter(m =>
          m.type === "tool_result" && m.is_error
        )
        if (toolErrors.length > 0) {
          console.error("[TUI] Tool errors detected:", toolErrors)
          return {
            role: "assistant" as const,
            content: [
              {
                type: "text" as const,
                text: `❌ **Tool Execution Failed**\n\n${toolErrors.map(e => `- ${e.content}`).join('\n')}\n\nPlease check your query and try again.`,
              },
            ],
          }
        }

        // This indicates a protocol/SDK issue - don't hide it
        throw new Error(
          `Agent SDK returned no assistant message. This is unexpected.\n` +
          `Received ${messages.length} messages of types: ${messages.map(m => m.type).join(', ')}\n` +
          `This may indicate an SDK bug, streaming issue, or malformed response.\n` +
          `See console output above for full message details.`
        )
      } catch (error) {
        console.error("[TUI] Query error:", error)
        throw error
      }
    },
    formatMessage: (response: any): AgentMessage => {
      // Agent SDK responses should already match AgentMessage interface
      return response
    },
    enableConsole: true,
  })

  renderer.root.add(loop)

  // Start the render loop
  renderer.start()
}

// Run the TUI
main().catch((error) => {
  console.error("Fatal error:", error)
  process.exit(1)
})
