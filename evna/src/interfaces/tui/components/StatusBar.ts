/**
 * Status Bar Component
 * Displays session info, token usage, cost, and current status
 */

import {
  BoxRenderable,
  TextRenderable,
  type RenderContext,
  type StyledText,
  RGBA,
  t,
  bold,
  fg,
} from "@opentui/core"
import type { TokenStats, FocusState } from "../types.js"

// ============================================================================
// Model Pricing (per million tokens)
// ============================================================================

interface ModelPricing {
  input: number
  output: number
  cacheRead?: number
  cacheWrite?: number
}

// Pricing as of December 2025 (per million tokens)
const MODEL_PRICING: Record<string, ModelPricing> = {
  "claude-3-5-haiku-20241022": { input: 0.25, output: 1.25, cacheRead: 0.025, cacheWrite: 0.30 },
  "claude-3-5-sonnet-20241022": { input: 3.00, output: 15.00, cacheRead: 0.30, cacheWrite: 3.75 },
  "claude-sonnet-4-20250514": { input: 3.00, output: 15.00, cacheRead: 0.30, cacheWrite: 3.75 },
  "claude-opus-4-20250514": { input: 15.00, output: 75.00, cacheRead: 1.50, cacheWrite: 18.75 },
  // Defaults for unknown models
  default: { input: 3.00, output: 15.00 },
}

// ============================================================================
// Status Bar Options
// ============================================================================

export interface StatusBarOptions {
  id: string
  width?: number | "auto" | `${number}%`
  position?: "absolute" | "relative"
  model?: string
}

// ============================================================================
// Status Bar Component
// ============================================================================

export class StatusBar extends BoxRenderable {
  private statusText: TextRenderable
  private tokenText: TextRenderable
  private costText: TextRenderable
  private modelText: TextRenderable
  private helpText: TextRenderable

  private currentStatus: string = "Ready"
  private currentTokens: TokenStats = { input: 0, output: 0, cached: 0 }
  private currentModel: string = "claude-3-5-haiku-20241022"
  private currentFocus: FocusState = "input"
  private sessionName: string = "New Session"
  private messageCount: number = 0

  constructor(ctx: RenderContext, options: StatusBarOptions) {
    super(ctx, {
      id: options.id,
      width: options.width ?? "100%",
      height: 1,
      position: options.position ?? "relative",
      flexDirection: "row",
      backgroundColor: "#1a1a2e",
      paddingLeft: 1,
      paddingRight: 1,
    })

    if (options.model) {
      this.currentModel = options.model
    }

    // Status indicator (leftmost)
    this.statusText = new TextRenderable(this.ctx, {
      id: `${options.id}-status`,
      content: this.formatStatus(),
      position: "relative",
    })

    // Token usage
    this.tokenText = new TextRenderable(this.ctx, {
      id: `${options.id}-tokens`,
      content: this.formatTokens(),
      position: "relative",
      paddingLeft: 2,
    })

    // Cost
    this.costText = new TextRenderable(this.ctx, {
      id: `${options.id}-cost`,
      content: this.formatCost(),
      position: "relative",
      paddingLeft: 2,
    })

    // Model
    this.modelText = new TextRenderable(this.ctx, {
      id: `${options.id}-model`,
      content: this.formatModel(),
      position: "relative",
      paddingLeft: 2,
    })

    // Help hint (rightmost)
    this.helpText = new TextRenderable(this.ctx, {
      id: `${options.id}-help`,
      content: t`${fg("#606080")("F1: Help")}`,
      position: "relative",
      paddingLeft: 2,
    })

    this.add(this.statusText)
    this.add(this.tokenText)
    this.add(this.costText)
    this.add(this.modelText)
    this.add(this.helpText)
  }

  private formatStatus(): StyledText {
    let statusColor: string
    let statusIcon: string

    switch (this.currentStatus.toLowerCase()) {
      case "ready":
        statusColor = "#00ff88"
        statusIcon = "●"
        break
      case "thinking...":
      case "processing...":
        statusColor = "#ffaa00"
        statusIcon = "◐"
        break
      case "streaming...":
        statusColor = "#00aaff"
        statusIcon = "◌"
        break
      case "error":
        statusColor = "#ff4444"
        statusIcon = "✗"
        break
      default:
        statusColor = "#808090"
        statusIcon = "○"
    }

    return t`${fg(statusColor)(statusIcon)} ${this.currentStatus}`
  }

  private formatTokens(): StyledText {
    const { input, output, cached } = this.currentTokens
    const base = `📊 ${this.formatNumber(input)}↑ ${this.formatNumber(output)}↓`
    if (cached > 0) {
      return t`${fg("#808090")(base)} ${fg("#808090")(`💾${this.formatNumber(cached)}`)}`
    }
    return t`${fg("#808090")(base)}`
  }

  private formatCost(): StyledText {
    const cost = this.calculateCost()
    const costStr = cost < 0.01 ? cost.toFixed(4) : cost.toFixed(2)
    return t`${fg("#808090")(`💰 $${costStr}`)}`
  }

  private formatModel(): StyledText {
    // Shorten model name for display
    let shortName = this.currentModel
      .replace("claude-", "")
      .replace("-20241022", "")
      .replace("-20250514", "")

    if (shortName.length > 15) {
      shortName = shortName.slice(0, 12) + "..."
    }

    return t`${fg("#606070")(`[${shortName}]`)}`
  }

  private formatNumber(n: number): string {
    if (n >= 1000000) {
      return (n / 1000000).toFixed(1) + "M"
    }
    if (n >= 1000) {
      return (n / 1000).toFixed(1) + "K"
    }
    return n.toString()
  }

  private calculateCost(): number {
    const pricing = MODEL_PRICING[this.currentModel] || MODEL_PRICING.default
    const { input, output, cached } = this.currentTokens

    const inputCost = (input / 1_000_000) * pricing.input
    const outputCost = (output / 1_000_000) * pricing.output
    const cacheCost = pricing.cacheRead ? (cached / 1_000_000) * pricing.cacheRead : 0

    return inputCost + outputCost + cacheCost
  }

  // === Public API ===

  public setStatus(status: string): void {
    this.currentStatus = status
    this.statusText.content = this.formatStatus()
    this.markDirty()
  }

  public setTokens(tokens: TokenStats): void {
    this.currentTokens = tokens
    this.tokenText.content = this.formatTokens()
    this.costText.content = this.formatCost()
    this.markDirty()
  }

  public addTokens(input: number, output: number, cached: number = 0): void {
    this.currentTokens.input += input
    this.currentTokens.output += output
    this.currentTokens.cached += cached
    this.tokenText.content = this.formatTokens()
    this.costText.content = this.formatCost()
    this.markDirty()
  }

  public setModel(model: string): void {
    this.currentModel = model
    this.modelText.content = this.formatModel()
    this.markDirty()
  }

  public setFocus(focus: FocusState): void {
    this.currentFocus = focus
    // Could update help text based on focus
    this.markDirty()
  }

  public setSessionName(name: string): void {
    this.sessionName = name
    this.markDirty()
  }

  public setMessageCount(count: number): void {
    this.messageCount = count
    this.markDirty()
  }

  public reset(): void {
    this.currentStatus = "Ready"
    this.currentTokens = { input: 0, output: 0, cached: 0 }
    this.statusText.content = this.formatStatus()
    this.tokenText.content = this.formatTokens()
    this.costText.content = this.formatCost()
    this.markDirty()
  }

  public getTokens(): TokenStats {
    return { ...this.currentTokens }
  }

  public getCost(): number {
    return this.calculateCost()
  }
}
