/**
 * Multi-line input component with tab support
 * Handles text editing, cursor navigation, and submission via Ctrl+Enter
 */

import {
  BoxRenderable,
  type CliRenderer,
  type KeyEvent,
  RGBA,
  OptimizedBuffer,
} from "@opentui/core"
import EventEmitter from "events"
import { appendFileSync } from "fs"
import type { CursorPosition } from "../types"

const KEY_LOG_FILE = "/tmp/evna-keys.log"

export interface MultilineInputOptions {
  id: string
  width?: number | string
  height?: number
  position?: "absolute" | "relative"
  left?: number
  top?: number
  placeholder?: string
  backgroundColor?: string | RGBA
  textColor?: string | RGBA
  cursorColor?: string | RGBA
  placeholderColor?: string | RGBA
  borderColor?: string | RGBA
}

export class MultilineInput extends BoxRenderable {
  private lines: string[] = [""]
  private cursor: CursorPosition = { line: 0, col: 0 }
  private scrollOffset: number = 0
  private focused: boolean = false
  private keyHandler: ((key: KeyEvent) => void) | null = null
  private emitter = new EventEmitter()

  // Styling
  private textColor: RGBA
  private cursorColor: RGBA
  private placeholderColor: RGBA
  private placeholder: string

  constructor(
    renderer: CliRenderer,
    options: MultilineInputOptions
  ) {
    super(renderer, {
      id: options.id,
      width: options.width ?? 80,
      height: options.height ?? 10,
      position: options.position ?? "relative",
      left: options.left,
      top: options.top,
      backgroundColor: options.backgroundColor ?? "#2a2a2a",
      borderColor: options.borderColor ?? "#555555",
      borderStyle: "rounded",
      border: true,
    })

    this.placeholder = options.placeholder ?? "Type your message..."
    this.textColor = this.parseColor(options.textColor ?? "#FFFFFF")
    this.cursorColor = this.parseColor(options.cursorColor ?? "#00FF00")
    this.placeholderColor = this.parseColor(options.placeholderColor ?? "#666666")

    this.setupKeyHandler()
  }

  private parseColor(color: string | RGBA): RGBA {
    if (typeof color === "string") {
      return RGBA.fromHex(color)
    }
    return color
  }

  private setupKeyHandler(): void {
    this.keyHandler = (key: KeyEvent) => {
      if (!this.focused) return
      this.handleKeypress(key)
    }

    this.ctx.keyInput.on("keypress", this.keyHandler)
  }

  private handleKeypress(key: KeyEvent): void {
    // Debug: Log all keys to file
    const logEntry = `${new Date().toISOString()} | name: "${key.name}" | seq: "${key.sequence}" | ctrl: ${key.ctrl} | shift: ${key.shift} | meta: ${key.meta} | option: ${key.option}\n`
    try {
      appendFileSync(KEY_LOG_FILE, logEntry)
    } catch (e) {
      // Ignore write errors
    }

    // Submit on Escape key (simple, works everywhere!)
    if (key.name === "escape" && this.getValue().trim().length > 0) {
      console.log("[MultilineInput] Submitting via ESCAPE")
      this.emitter.emit("submit", this.getValue())
      return
    }

    // Submit on sequence "enter" (numpad enter key)
    if (key.sequence === "enter" && this.getValue().trim().length > 0) {
      console.log("[MultilineInput] Submitting via numpad ENTER")
      this.emitter.emit("submit", this.getValue())
      return
    }

    // Submit on Ctrl+Enter, Alt+Enter, or Cmd+Enter (macOS compatibility)
    if ((key.ctrl || key.meta || key.option) && key.name === "return") {
      console.log("[MultilineInput] Submitting via modifier+return")
      this.emitter.emit("submit", this.getValue())
      return
    }

    // Submit on Ctrl+D (alternative for macOS)
    if (key.ctrl && key.name === "d") {
      console.log("[MultilineInput] Submitting via Ctrl+D")
      this.emitter.emit("submit", this.getValue())
      return
    }

    // Newline on plain Enter
    if (key.name === "return") {
      const currentLine = this.lines[this.cursor.line]
      const beforeCursor = currentLine.slice(0, this.cursor.col)
      const afterCursor = currentLine.slice(this.cursor.col)

      this.lines[this.cursor.line] = beforeCursor
      this.lines.splice(this.cursor.line + 1, 0, afterCursor)
      this.cursor.line++
      this.cursor.col = 0
      this.updateScrollOffset()
      this.markDirty()
      return
    }

    // Tab insertion (literal \t character)
    if (key.name === "tab" && !key.shift) {
      const currentLine = this.lines[this.cursor.line]
      this.lines[this.cursor.line] =
        currentLine.slice(0, this.cursor.col) +
        "\t" +
        currentLine.slice(this.cursor.col)
      this.cursor.col++
      this.markDirty()
      return
    }

    // Backspace
    if (key.name === "backspace") {
      if (this.cursor.col > 0) {
        // Delete character before cursor
        const currentLine = this.lines[this.cursor.line]
        this.lines[this.cursor.line] =
          currentLine.slice(0, this.cursor.col - 1) +
          currentLine.slice(this.cursor.col)
        this.cursor.col--
      } else if (this.cursor.line > 0) {
        // Join with previous line
        const currentLine = this.lines[this.cursor.line]
        this.cursor.line--
        this.cursor.col = this.lines[this.cursor.line].length
        this.lines[this.cursor.line] += currentLine
        this.lines.splice(this.cursor.line + 1, 1)
        this.updateScrollOffset()
      }
      this.markDirty()
      return
    }

    // Delete key
    if (key.name === "delete") {
      const currentLine = this.lines[this.cursor.line]
      if (this.cursor.col < currentLine.length) {
        this.lines[this.cursor.line] =
          currentLine.slice(0, this.cursor.col) +
          currentLine.slice(this.cursor.col + 1)
      } else if (this.cursor.line < this.lines.length - 1) {
        // Join with next line
        this.lines[this.cursor.line] += this.lines[this.cursor.line + 1]
        this.lines.splice(this.cursor.line + 1, 1)
      }
      this.markDirty()
      return
    }

    // Arrow keys
    if (key.name === "up" && this.cursor.line > 0) {
      this.cursor.line--
      this.cursor.col = Math.min(this.cursor.col, this.lines[this.cursor.line].length)
      this.updateScrollOffset()
      this.markDirty()
      return
    }

    if (key.name === "down" && this.cursor.line < this.lines.length - 1) {
      this.cursor.line++
      this.cursor.col = Math.min(this.cursor.col, this.lines[this.cursor.line].length)
      this.updateScrollOffset()
      this.markDirty()
      return
    }

    if (key.name === "left") {
      if (this.cursor.col > 0) {
        this.cursor.col--
      } else if (this.cursor.line > 0) {
        // Move to end of previous line
        this.cursor.line--
        this.cursor.col = this.lines[this.cursor.line].length
        this.updateScrollOffset()
      }
      this.markDirty()
      return
    }

    if (key.name === "right") {
      const currentLine = this.lines[this.cursor.line]
      if (this.cursor.col < currentLine.length) {
        this.cursor.col++
      } else if (this.cursor.line < this.lines.length - 1) {
        // Move to start of next line
        this.cursor.line++
        this.cursor.col = 0
        this.updateScrollOffset()
      }
      this.markDirty()
      return
    }

    // Home/End
    if (key.name === "home") {
      this.cursor.col = 0
      this.markDirty()
      return
    }

    if (key.name === "end") {
      this.cursor.col = this.lines[this.cursor.line].length
      this.markDirty()
      return
    }

    // Ctrl+A (select all - for now just move to start)
    if (key.ctrl && key.name === "a") {
      this.cursor = { line: 0, col: 0 }
      this.updateScrollOffset()
      this.markDirty()
      return
    }

    // Ctrl+E (move to end)
    if (key.ctrl && key.name === "e") {
      this.cursor.line = this.lines.length - 1
      this.cursor.col = this.lines[this.cursor.line].length
      this.updateScrollOffset()
      this.markDirty()
      return
    }

    // Regular character input
    if (key.sequence && key.sequence.length === 1 && !key.ctrl) {
      const currentLine = this.lines[this.cursor.line]
      this.lines[this.cursor.line] =
        currentLine.slice(0, this.cursor.col) +
        key.sequence +
        currentLine.slice(this.cursor.col)
      this.cursor.col++
      this.markDirty()
    }
  }

  protected renderSelf(buffer: OptimizedBuffer): void {
    super.renderSelf(buffer)

    const visibleHeight = this.height - 2  // Account for border
    const visibleWidth = this.width - 2

    // Show placeholder if empty
    if (this.lines.length === 1 && this.lines[0] === "" && !this.focused) {
      buffer.drawText(
        this.placeholder.slice(0, visibleWidth),
        this.x + 1,
        this.y + 1,
        this.placeholderColor
      )
      return
    }

    // Render visible lines
    for (let i = 0; i < visibleHeight; i++) {
      const lineIndex = this.scrollOffset + i
      if (lineIndex >= this.lines.length) break

      const line = this.lines[lineIndex]
      const displayLine = this.expandTabs(line)

      // Render line text
      buffer.drawText(
        displayLine.slice(0, visibleWidth),
        this.x + 1,
        this.y + 1 + i,
        this.textColor
      )

      // Render cursor if focused and on this line
      if (this.focused && lineIndex === this.cursor.line) {
        const cursorX = this.x + 1 + this.expandTabs(line.slice(0, this.cursor.col)).length
        const cursorY = this.y + 1 + i

        // Ensure cursor is within bounds
        if (cursorX < this.x + this.width - 1) {
          buffer.setCell(cursorX, cursorY, "█", this.cursorColor, this.backgroundColor)
        }
      }
    }
  }

  private expandTabs(text: string, tabWidth: number = 4): string {
    return text.replace(/\t/g, " ".repeat(tabWidth))
  }

  private updateScrollOffset(): void {
    const visibleHeight = this.height - 2

    // Scroll down if cursor is below visible area
    if (this.cursor.line >= this.scrollOffset + visibleHeight) {
      this.scrollOffset = this.cursor.line - visibleHeight + 1
    }

    // Scroll up if cursor is above visible area
    if (this.cursor.line < this.scrollOffset) {
      this.scrollOffset = this.cursor.line
    }
  }

  // Public API

  public focus(): void {
    this.focused = true
    this.borderColor = RGBA.fromHex("#00FF00")
    this.markDirty()
  }

  public blur(): void {
    this.focused = false
    this.borderColor = RGBA.fromHex("#555555")
    this.markDirty()
  }

  public getValue(): string {
    return this.lines.join("\n")
  }

  public setValue(value: string): void {
    this.lines = value.split("\n")
    if (this.lines.length === 0) this.lines = [""]
    this.cursor = { line: 0, col: 0 }
    this.scrollOffset = 0
    this.markDirty()
  }

  public clear(): void {
    this.lines = [""]
    this.cursor = { line: 0, col: 0 }
    this.scrollOffset = 0
    this.markDirty()
  }

  public getCursor(): CursorPosition {
    return { ...this.cursor }
  }

  public on(event: "submit", handler: (value: string) => void): void {
    this.emitter.on(event, handler)
  }

  public destroy(): void {
    if (this.keyHandler) {
      this.ctx.keyInput.off("keypress", this.keyHandler)
      this.keyHandler = null
    }
    super.destroy()
  }
}
