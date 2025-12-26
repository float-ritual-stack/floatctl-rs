/**
 * Message Renderer Component
 * Renders individual chat messages with markdown support, tool visualization, and streaming
 */

import {
  BoxRenderable,
  TextRenderable,
  ScrollBoxRenderable,
  type RenderContext,
  RGBA,
  TextAttributes,
  t,
  bold,
  fg,
} from "@opentui/core"
import type {
  AgentMessage,
  ContentBlock,
  ToolUseBlock,
  ToolResultBlock,
  ThinkingBlock,
  TextBlock,
} from "../types.js"

// ============================================================================
// Theme Colors
// ============================================================================

const COLORS = {
  // Role colors
  user: "#00ff88",
  assistant: "#00aaff",
  system: "#ffaa00",
  tool: "#ff66ff",

  // Content colors
  text: "#e0e0e0",
  textMuted: "#808090",
  textDim: "#606070",

  // Code colors
  code: "#ffd700",
  codeBackground: "#1a1a2e",
  codeBorder: "#303050",

  // Tool colors
  toolName: "#ff66ff",
  toolInput: "#66ccff",
  toolSuccess: "#00ff88",
  toolError: "#ff4444",

  // Thinking
  thinking: "#888888",

  // Streaming
  streamingCursor: "#00ff88",

  // Backgrounds
  bgUser: "#1a2a1a",
  bgAssistant: "#1a1a2a",
  bgSystem: "#2a2a1a",
  bgTool: "#2a1a2a",
  bgError: "#2a1a1a",

  // Borders
  border: "#404050",
}

// ============================================================================
// Markdown Parser (Simple)
// ============================================================================

interface ParsedSegment {
  text: string
  style: "normal" | "bold" | "italic" | "code" | "codeBlock" | "heading" | "link" | "list"
  language?: string
}

// Cache parsed markdown to avoid re-parsing on every render
// Key: raw text content, Value: parsed segments
const markdownCache = new Map<string, ParsedSegment[]>()
const MAX_CACHE_SIZE = 100 // Limit cache growth for long sessions

function parseMarkdown(text: string): ParsedSegment[] {
  // Check cache first
  const cached = markdownCache.get(text)
  if (cached) return cached

  const segments: ParsedSegment[] = []
  const lines = text.split("\n")
  let inCodeBlock = false
  let codeBlockLang = ""
  let codeBlockContent: string[] = []

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]

    // Code block start/end
    if (line.startsWith("```")) {
      if (inCodeBlock) {
        // End code block
        segments.push({
          text: codeBlockContent.join("\n"),
          style: "codeBlock",
          language: codeBlockLang,
        })
        inCodeBlock = false
        codeBlockContent = []
        codeBlockLang = ""
      } else {
        // Start code block
        inCodeBlock = true
        codeBlockLang = line.slice(3).trim()
      }
      continue
    }

    if (inCodeBlock) {
      codeBlockContent.push(line)
      continue
    }

    // Headings
    if (line.startsWith("# ")) {
      segments.push({ text: line.slice(2), style: "heading" })
      if (i < lines.length - 1) segments.push({ text: "\n", style: "normal" })
      continue
    }
    if (line.startsWith("## ")) {
      segments.push({ text: line.slice(3), style: "heading" })
      if (i < lines.length - 1) segments.push({ text: "\n", style: "normal" })
      continue
    }
    if (line.startsWith("### ")) {
      segments.push({ text: line.slice(4), style: "heading" })
      if (i < lines.length - 1) segments.push({ text: "\n", style: "normal" })
      continue
    }

    // List items
    if (line.match(/^[\-\*]\s/)) {
      segments.push({ text: "  • " + line.slice(2), style: "list" })
      if (i < lines.length - 1) segments.push({ text: "\n", style: "normal" })
      continue
    }
    if (line.match(/^\d+\.\s/)) {
      const match = line.match(/^(\d+)\.\s(.*)/)
      if (match) {
        segments.push({ text: `  ${match[1]}. ${match[2]}`, style: "list" })
        if (i < lines.length - 1) segments.push({ text: "\n", style: "normal" })
        continue
      }
    }

    // Parse inline formatting
    let remaining = line
    while (remaining.length > 0) {
      // Bold **text**
      const boldMatch = remaining.match(/^\*\*(.+?)\*\*/)
      if (boldMatch) {
        segments.push({ text: boldMatch[1], style: "bold" })
        remaining = remaining.slice(boldMatch[0].length)
        continue
      }

      // Italic *text*
      const italicMatch = remaining.match(/^\*(.+?)\*/)
      if (italicMatch) {
        segments.push({ text: italicMatch[1], style: "italic" })
        remaining = remaining.slice(italicMatch[0].length)
        continue
      }

      // Inline code `text`
      const codeMatch = remaining.match(/^`([^`]+)`/)
      if (codeMatch) {
        segments.push({ text: codeMatch[1], style: "code" })
        remaining = remaining.slice(codeMatch[0].length)
        continue
      }

      // Link [text](url)
      const linkMatch = remaining.match(/^\[([^\]]+)\]\(([^)]+)\)/)
      if (linkMatch) {
        segments.push({ text: `${linkMatch[1]} (${linkMatch[2]})`, style: "link" })
        remaining = remaining.slice(linkMatch[0].length)
        continue
      }

      // Normal text (up to next special character)
      const normalMatch = remaining.match(/^[^*`\[\n]+/)
      if (normalMatch) {
        segments.push({ text: normalMatch[0], style: "normal" })
        remaining = remaining.slice(normalMatch[0].length)
        continue
      }

      // Single special character that wasn't matched
      segments.push({ text: remaining[0], style: "normal" })
      remaining = remaining.slice(1)
    }

    // Add newline between lines
    if (i < lines.length - 1) {
      segments.push({ text: "\n", style: "normal" })
    }
  }

  // Handle unclosed code block
  if (inCodeBlock && codeBlockContent.length > 0) {
    segments.push({
      text: codeBlockContent.join("\n"),
      style: "codeBlock",
      language: codeBlockLang,
    })
  }

  // Cache result (with size limit to prevent unbounded growth)
  if (markdownCache.size >= MAX_CACHE_SIZE) {
    // Remove oldest entry (first key in Map iteration order)
    const firstKey = markdownCache.keys().next().value
    if (firstKey !== undefined) {
      markdownCache.delete(firstKey)
    }
  }
  markdownCache.set(text, segments)

  return segments
}

// ============================================================================
// Message Renderer Options
// ============================================================================

export interface MessageRendererOptions {
  id: string
  width?: number | "auto" | `${number}%`
  position?: "absolute" | "relative"
  left?: number
  top?: number
  showTimestamps?: boolean
  compactMode?: boolean
}

// ============================================================================
// Message Renderer Component
// ============================================================================

export class MessageRenderer extends ScrollBoxRenderable {
  private messageCount: number = 0
  private showTimestamps: boolean
  private compactMode: boolean

  constructor(ctx: RenderContext, options: MessageRendererOptions) {
    super(ctx, {
      id: options.id,
      rootOptions: {
        width: options.width ?? "100%",
        position: options.position ?? "relative",
        left: options.left,
        top: options.top,
        backgroundColor: RGBA.fromHex("#0d0d1a"),
      },
      contentOptions: {
        flexDirection: "column",
        gap: 1,
        padding: 1,
        backgroundColor: RGBA.fromHex("#0d0d1a"),
      },
      scrollbarOptions: {
        showArrows: false,
        trackOptions: {
          foregroundColor: RGBA.fromHex("#00ff88"),
          backgroundColor: RGBA.fromHex("#1a1a2e"),
        },
      },
    })

    this.showTimestamps = options.showTimestamps ?? false
    this.compactMode = options.compactMode ?? false
  }

  public addMessage(message: AgentMessage): void {
    const messageId = `message-${this.messageCount++}`

    // Message container
    const container = new BoxRenderable(this.ctx, {
      id: messageId,
      position: "relative",
      width: "100%",
      minHeight: this.compactMode ? 2 : 3,
      backgroundColor: this.getBackgroundColor(message.role),
      borderColor: RGBA.fromHex(COLORS[message.role] || COLORS.border),
      borderStyle: "rounded",
      border: true,
      marginBottom: this.compactMode ? 0 : 1,
      paddingLeft: 1,
      paddingRight: 1,
    })

    // Role header with timestamp
    const roleHeader = this.createRoleHeader(message, messageId)
    container.add(roleHeader)

    // Content blocks
    for (let idx = 0; idx < message.content.length; idx++) {
      const block = message.content[idx]
      const blockElements = this.renderContentBlock(block, `${messageId}-block-${idx}`)
      for (const element of blockElements) {
        container.add(element)
      }
    }

    // Usage stats footer (if available)
    if (message.usage && !this.compactMode) {
      const usageText = new TextRenderable(this.ctx, {
        id: `${messageId}-usage`,
        content: t`${fg(COLORS.textDim)(`📊 ${message.usage.input_tokens}↑ ${message.usage.output_tokens}↓`)}${message.usage.cache_read_input_tokens ? fg(COLORS.textDim)(` 💾${message.usage.cache_read_input_tokens}`) : ""}`,
        position: "relative",
        paddingTop: 1,
      })
      container.add(usageText)
    }

    this.add(container)
  }

  private createRoleHeader(message: AgentMessage, messageId: string): TextRenderable {
    const emoji = this.getRoleEmoji(message.role)
    const roleName = message.role.toUpperCase()
    const color = COLORS[message.role] || "#FFFFFF"

    // Build header parts - use escape sequences for color/style instead of nested template literals
    const roleLabel = bold(fg(color)(`${emoji} ${roleName}`))

    // Build timestamp suffix if enabled
    let timeSuffix = ""
    if (this.showTimestamps && message.timestamp) {
      const time = new Date(message.timestamp).toLocaleTimeString("en-US", {
        hour: "2-digit",
        minute: "2-digit",
      })
      timeSuffix = ` \x1b[38;5;245m${time}\x1b[39m`  // Use ANSI escape for dim
    }

    // Build model suffix if available
    let modelSuffix = ""
    if (message.model && !this.compactMode) {
      const modelShort = message.model.replace("claude-", "").split("-").slice(0, 2).join("-")
      modelSuffix = ` \x1b[38;5;245m[${modelShort}]\x1b[39m`  // Use ANSI escape for dim
    }

    // Combine into single template literal
    const headerContent = t`${roleLabel}${timeSuffix}${modelSuffix}`

    return new TextRenderable(this.ctx, {
      id: `${messageId}-role`,
      content: headerContent,
      position: "relative",
      paddingBottom: 1,
    })
  }

  private renderContentBlock(block: ContentBlock, id: string): TextRenderable[] {
    switch (block.type) {
      case "text":
        return this.renderTextBlock(block as TextBlock, id)

      case "streaming_text":
        return this.renderStreamingText(block, id)

      case "tool_use":
        return this.renderToolUse(block as ToolUseBlock, id)

      case "tool_result":
        return this.renderToolResult(block as ToolResultBlock, id)

      case "thinking":
        return this.renderThinking(block as ThinkingBlock, id)

      default:
        return []
    }
  }

  private renderTextBlock(block: TextBlock, id: string): TextRenderable[] {
    const segments = parseMarkdown(block.text)
    const elements: TextRenderable[] = []

    // Group segments into lines for rendering
    let currentLine = ""
    let lineNum = 0

    for (const segment of segments) {
      if (segment.style === "codeBlock") {
        // Flush current line
        if (currentLine) {
          elements.push(
            new TextRenderable(this.ctx, {
              id: `${id}-line-${lineNum++}`,
              content: currentLine,
              position: "relative",
              fg: COLORS.text,
            })
          )
          currentLine = ""
        }

        // Render code block
        const langLabel = segment.language ? ` [${segment.language}]` : ""
        elements.push(
          new TextRenderable(this.ctx, {
            id: `${id}-code-${lineNum++}`,
            content: t`${fg(COLORS.codeBorder)("─".repeat(40))}${fg(COLORS.code)(langLabel)}\n${fg(COLORS.code)(segment.text)}\n${fg(COLORS.codeBorder)("─".repeat(40))}`,
            position: "relative",
            paddingTop: 1,
            paddingBottom: 1,
          })
        )
      } else if (segment.text === "\n") {
        // Flush line and start new
        if (currentLine) {
          elements.push(
            new TextRenderable(this.ctx, {
              id: `${id}-line-${lineNum++}`,
              content: currentLine,
              position: "relative",
              fg: COLORS.text,
            })
          )
          currentLine = ""
        }
      } else {
        // Build styled inline content
        currentLine += this.styleSegment(segment)
      }
    }

    // Flush remaining
    if (currentLine) {
      elements.push(
        new TextRenderable(this.ctx, {
          id: `${id}-line-${lineNum}`,
          content: currentLine,
          position: "relative",
          fg: COLORS.text,
        })
      )
    }

    return elements.length > 0 ? elements : [
      new TextRenderable(this.ctx, {
        id,
        content: block.text,
        position: "relative",
        fg: COLORS.text,
      })
    ]
  }

  private styleSegment(segment: ParsedSegment): string {
    switch (segment.style) {
      case "bold":
        return `\x1b[1m${segment.text}\x1b[22m`
      case "italic":
        return `\x1b[3m${segment.text}\x1b[23m`
      case "code":
        return `\x1b[33m${segment.text}\x1b[39m`  // Yellow for inline code
      case "heading":
        return `\x1b[1;36m${segment.text}\x1b[0m`  // Bold cyan for headings
      case "link":
        return `\x1b[4;34m${segment.text}\x1b[0m`  // Underline blue for links
      case "list":
        return segment.text
      default:
        return segment.text
    }
  }

  private renderStreamingText(block: { text: string; complete: boolean }, id: string): TextRenderable[] {
    const cursor = block.complete ? "" : "▊"
    return [
      new TextRenderable(this.ctx, {
        id,
        content: t`${fg(COLORS.text)(block.text)}${fg(COLORS.streamingCursor)(cursor)}`,
        position: "relative",
      })
    ]
  }

  private renderToolUse(toolUse: ToolUseBlock, id: string): TextRenderable[] {
    const inputStr = this.formatJson(toolUse.input, 60)

    return [
      new TextRenderable(this.ctx, {
        id: `${id}-header`,
        content: t`${bold(fg(COLORS.toolName)(`🔧 ${toolUse.name}`))}`,
        position: "relative",
        paddingTop: 1,
      }),
      new TextRenderable(this.ctx, {
        id: `${id}-input`,
        content: t`${fg(COLORS.textMuted)("Input: ")}${fg(COLORS.toolInput)(inputStr)}`,
        position: "relative",
        paddingLeft: 2,
      }),
      new TextRenderable(this.ctx, {
        id: `${id}-id`,
        content: t`${fg(COLORS.textDim)(`ID: ${toolUse.id.slice(0, 20)}...`)}`,
        position: "relative",
        paddingLeft: 2,
        paddingBottom: 1,
      }),
    ]
  }

  private renderToolResult(result: ToolResultBlock, id: string): TextRenderable[] {
    const statusEmoji = result.is_error ? "❌" : "✅"
    const statusColor = result.is_error ? COLORS.toolError : COLORS.toolSuccess
    const contentStr = this.formatToolContent(result.content, 200)

    return [
      new TextRenderable(this.ctx, {
        id: `${id}-header`,
        content: t`${bold(fg(statusColor)(`${statusEmoji} Result`))}`,
        position: "relative",
        paddingTop: 1,
      }),
      new TextRenderable(this.ctx, {
        id: `${id}-content`,
        content: t`${fg(result.is_error ? COLORS.toolError : COLORS.text)(contentStr)}`,
        position: "relative",
        paddingLeft: 2,
        paddingBottom: 1,
      }),
    ]
  }

  private renderThinking(thinking: ThinkingBlock, id: string): TextRenderable[] {
    // Truncate long thinking blocks
    const maxLen = 500
    let thinkText = thinking.thinking
    if (thinkText.length > maxLen) {
      thinkText = thinkText.slice(0, maxLen) + "..."
    }

    return [
      new TextRenderable(this.ctx, {
        id,
        content: t`${fg(COLORS.thinking)(`💭 ${thinkText}`)}`,
        position: "relative",
        paddingTop: 1,
        paddingBottom: 1,
        attributes: TextAttributes.ITALIC,
      })
    ]
  }

  private formatToolContent(content: unknown, maxLen: number): string {
    if (typeof content === "string") {
      return content.length > maxLen ? content.slice(0, maxLen) + "..." : content
    }
    if (Array.isArray(content)) {
      const text = content
        .map((block) => {
          if (typeof block === "object" && block !== null && "type" in block && block.type === "text" && "text" in block) {
            return block.text
          }
          return JSON.stringify(block)
        })
        .join("\n")
      return text.length > maxLen ? text.slice(0, maxLen) + "..." : text
    }
    const json = this.formatJson(content, maxLen)
    return json
  }

  private formatJson(obj: unknown, maxLen: number = 100): string {
    try {
      const str = JSON.stringify(obj, null, 2)
      if (str.length <= maxLen) return str

      // Try compact format
      const compact = JSON.stringify(obj)
      if (compact.length <= maxLen) return compact

      // Truncate
      return compact.slice(0, maxLen) + "..."
    } catch {
      return String(obj)
    }
  }

  private getRoleEmoji(role: string): string {
    switch (role) {
      case "user":
        return "👤"
      case "assistant":
        return "🤖"
      case "system":
        return "⚙️"
      case "tool":
        return "🔧"
      default:
        return "💬"
    }
  }

  private getBackgroundColor(role: string): RGBA {
    switch (role) {
      case "user":
        return RGBA.fromHex(COLORS.bgUser)
      case "assistant":
        return RGBA.fromHex(COLORS.bgAssistant)
      case "system":
        return RGBA.fromHex(COLORS.bgSystem)
      case "tool":
        return RGBA.fromHex(COLORS.bgTool)
      default:
        return RGBA.fromHex("#1a1a1a")
    }
  }

  public addError(error: string, severity: "error" | "warning" = "error"): void {
    const errorId = `error-${this.messageCount++}`
    const emoji = severity === "error" ? "❌" : "⚠️"
    const color = severity === "error" ? COLORS.toolError : COLORS.system

    const errorBox = new BoxRenderable(this.ctx, {
      id: errorId,
      position: "relative",
      width: "100%",
      minHeight: 3,
      backgroundColor: RGBA.fromHex(COLORS.bgError),
      borderColor: RGBA.fromHex(color),
      borderStyle: "rounded",
      border: true,
      marginBottom: 1,
      paddingLeft: 1,
      paddingRight: 1,
    })

    const errorText = new TextRenderable(this.ctx, {
      id: `${errorId}-text`,
      content: t`${bold(fg(color)(`${emoji} Error`))}\n${fg(COLORS.text)(error)}`,
      position: "relative",
    })

    errorBox.add(errorText)
    this.add(errorBox)
  }

  public addStreamingMessage(messageId: string): BoxRenderable {
    const container = new BoxRenderable(this.ctx, {
      id: messageId,
      position: "relative",
      width: "100%",
      minHeight: 3,
      backgroundColor: RGBA.fromHex(COLORS.bgAssistant),
      borderColor: RGBA.fromHex(COLORS.assistant),
      borderStyle: "rounded",
      border: true,
      marginBottom: 1,
      paddingLeft: 1,
      paddingRight: 1,
    })

    const header = new TextRenderable(this.ctx, {
      id: `${messageId}-role`,
      content: t`${bold(fg(COLORS.assistant)("🤖 ASSISTANT"))} ${fg(COLORS.textDim)("...")}`,
      position: "relative",
      paddingBottom: 1,
    })

    const streamText = new TextRenderable(this.ctx, {
      id: `${messageId}-stream`,
      content: t`${fg(COLORS.streamingCursor)("▊")}`,
      position: "relative",
    })

    container.add(header)
    container.add(streamText)
    this.add(container)

    return container
  }

  public updateStreamingText(messageId: string, text: string): void {
    // Find the streaming text element by iterating children
    const children = this.getChildren()
    const container = children.find((c) => c.id === messageId) as BoxRenderable | undefined
    if (!container) return

    const containerChildren = container.getChildren()
    const streamTextEl = containerChildren.find((c) => c.id === `${messageId}-stream`) as TextRenderable | undefined
    if (streamTextEl) {
      streamTextEl.content = t`${fg(COLORS.text)(text)}${fg(COLORS.streamingCursor)("▊")}`
    }
  }

  public finalizeStreamingMessage(messageId: string, message: AgentMessage): void {
    // Remove streaming container and add final message
    this.remove(messageId)
    this.addMessage(message)
  }

  // Note: These only affect NEW messages. Existing messages retain their original formatting.
  // A full refresh would require re-rendering all messages from stored AgentMessage objects.
  public setShowTimestamps(show: boolean): void {
    this.showTimestamps = show
  }

  public setCompactMode(compact: boolean): void {
    this.compactMode = compact
  }

  public clear(): void {
    const children = this.getChildren()
    children.forEach((child) => {
      this.remove(child.id)
    })
    this.messageCount = 0
  }

  public scrollToBottom(): void {
    // Scroll to maximum offset (bottom of content)
    this.scrollTo(Infinity)
  }

  public getMessageCount(): number {
    return this.messageCount
  }
}
