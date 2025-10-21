/**
 * Message renderer for Agent SDK responses
 * Formats user/assistant messages, tool calls, and results with proper styling
 */

import {
  BoxRenderable,
  TextRenderable,
  type CliRenderer,
  RGBA,
  TextAttributes,
  t,
  bold,
  fg,
} from "@opentui/core"
import type { AgentMessage, ContentBlock, ToolUseBlock, ToolResultBlock } from "../types"

export interface MessageRendererOptions {
  id: string
  width?: number | string
  position?: "absolute" | "relative"
  left?: number
  top?: number
}

const COLORS = {
  user: "#00FF00",        // Green
  assistant: "#00AAFF",   // Blue
  system: "#FFAA00",      // Orange
  tool_use: "#FF00FF",    // Magenta
  tool_success: "#00FF00", // Green
  tool_error: "#FF0000",  // Red
  thinking: "#888888",    // Gray
  border: "#555555",
}

export class MessageRenderer extends BoxRenderable {
  private messageCount: number = 0

  constructor(
    renderer: CliRenderer,
    options: MessageRendererOptions
  ) {
    super(renderer, {
      id: options.id,
      width: options.width ?? "100%",
      position: options.position ?? "relative",
      left: options.left,
      top: options.top,
      flexDirection: "column",
      overflow: "scroll",
    })
  }

  public addMessage(message: AgentMessage): void {
    const messageId = `message-${this.messageCount++}`

    // Message container
    const container = new BoxRenderable(this.ctx, {
      id: messageId,
      position: "relative",
      width: "100%",
      minHeight: 3,
      backgroundColor: this.getBackgroundColor(message.role),
      borderColor: RGBA.fromHex(COLORS[message.role] || COLORS.border),
      borderStyle: "rounded",
      border: true,
      marginBottom: 1,
    })

    // Role header
    const roleText = new TextRenderable(this.ctx, {
      id: `${messageId}-role`,
      content: t`${bold(fg(COLORS[message.role] || "#FFFFFF")(this.getRoleEmoji(message.role) + " " + message.role.toUpperCase()))}`,
      position: "relative",
      paddingLeft: 1,
      paddingTop: 0,
    })
    container.add(roleText)

    // Render content blocks
    message.content.forEach((block, idx) => {
      const blockElement = this.renderContentBlock(block, `${messageId}-block-${idx}`)
      if (blockElement) {
        container.add(blockElement)
      }
    })

    // Usage stats footer
    if (message.usage) {
      const usageText = new TextRenderable(this.ctx, {
        id: `${messageId}-usage`,
        content: t`${fg("#888888")(`📊 Tokens: ${message.usage.input_tokens}↑ ${message.usage.output_tokens}↓`)}`,
        position: "relative",
        paddingLeft: 1,
        paddingBottom: 0,
      })
      container.add(usageText)
    }

    this.add(container)
  }

  private renderContentBlock(block: ContentBlock, id: string): TextRenderable | null {
    switch (block.type) {
      case "text":
        return new TextRenderable(this.ctx, {
          id,
          content: block.text,
          position: "relative",
          paddingLeft: 1,
          paddingRight: 1,
          fg: "#FFFFFF",
        })

      case "tool_use":
        return this.renderToolUse(block, id)

      case "tool_result":
        return this.renderToolResult(block, id)

      case "thinking":
        return new TextRenderable(this.ctx, {
          id,
          content: t`${fg(COLORS.thinking)("🤔 " + block.thinking)}`,
          position: "relative",
          paddingLeft: 1,
          paddingRight: 1,
          attributes: TextAttributes.ITALIC,
        })

      default:
        return null
    }
  }

  private renderToolUse(toolUse: ToolUseBlock, id: string): TextRenderable {
    const inputStr = this.formatJson(toolUse.input)
    const content = t`${bold(fg(COLORS.tool_use)("🔧 Tool: " + toolUse.name))}
${fg("#CCCCCC")("Input:")}
${fg("#AAAAAA")(inputStr)}
${fg("#666666")("ID: " + toolUse.id)}`

    return new TextRenderable(this.ctx, {
      id,
      content,
      position: "relative",
      paddingLeft: 2,
      paddingRight: 1,
      paddingTop: 0.5,
      paddingBottom: 0.5,
    })
  }

  private renderToolResult(result: ToolResultBlock, id: string): TextRenderable {
    const statusEmoji = result.is_error ? "❌" : "✅"
    const statusColor = result.is_error ? COLORS.tool_error : COLORS.tool_success
    const contentStr = this.formatToolContent(result.content)

    const content = t`${bold(fg(statusColor)(statusEmoji + " Result:"))}
${fg("#CCCCCC")(contentStr)}`

    return new TextRenderable(this.ctx, {
      id,
      content,
      position: "relative",
      paddingLeft: 2,
      paddingRight: 1,
      paddingTop: 0.5,
      paddingBottom: 0.5,
    })
  }

  private formatToolContent(content: any): string {
    if (typeof content === "string") {
      return content
    }
    if (Array.isArray(content)) {
      return content
        .map((block) => {
          if (typeof block === "object" && block.type === "text") {
            return block.text
          }
          return JSON.stringify(block, null, 2)
        })
        .join("\n")
    }
    return this.formatJson(content)
  }

  private formatJson(obj: any, indent: number = 2): string {
    return JSON.stringify(obj, null, indent)
  }

  private getRoleEmoji(role: string): string {
    switch (role) {
      case "user":
        return "👤"
      case "assistant":
        return "🤖"
      case "system":
        return "⚙️"
      default:
        return "💬"
    }
  }

  private getBackgroundColor(role: string): RGBA {
    switch (role) {
      case "user":
        return RGBA.fromHex("#1a2a1a")
      case "assistant":
        return RGBA.fromHex("#1a1a2a")
      case "system":
        return RGBA.fromHex("#2a2a1a")
      default:
        return RGBA.fromHex("#1a1a1a")
    }
  }

  public addError(error: string, severity: "error" | "warning" = "error"): void {
    const errorId = `error-${this.messageCount++}`
    const emoji = severity === "error" ? "❌" : "⚠️"
    const color = severity === "error" ? COLORS.tool_error : COLORS.system

    const errorBox = new BoxRenderable(this.ctx, {
      id: errorId,
      position: "relative",
      width: "100%",
      minHeight: 3,
      backgroundColor: RGBA.fromHex("#2a1a1a"),
      borderColor: RGBA.fromHex(color),
      borderStyle: "rounded",
      border: true,
      marginBottom: 1,
    })

    const errorText = new TextRenderable(this.ctx, {
      id: `${errorId}-text`,
      content: t`${bold(fg(color)(emoji + " Error:"))}
${fg("#FFFFFF")(error)}`,
      position: "relative",
      paddingLeft: 1,
      paddingRight: 1,
    })

    errorBox.add(errorText)
    this.add(errorBox)
  }

  public clear(): void {
    // Remove all child renderables
    const children = this.getChildren()
    children.forEach((child) => {
      this.remove(child.id)
    })
    this.messageCount = 0
  }

  public scrollToBottom(): void {
    // Trigger scroll to end
    // (OpenTUI auto-scrolls on new content in scroll containers)
    this.markDirty()
  }
}
