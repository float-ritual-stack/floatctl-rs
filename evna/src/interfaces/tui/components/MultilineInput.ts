/**
 * Enhanced Multi-line Input Component
 * Full-featured text editor with word navigation, history, and clipboard support
 */

import {
  BoxRenderable,
  type RenderContext,
  type KeyEvent,
  RGBA,
  OptimizedBuffer,
} from "@opentui/core"
import EventEmitter from "events"
import type { CursorPosition, TextSelection, ThemeColors, DEFAULT_THEME } from "../types.js"

export interface MultilineInputOptions {
  id: string
  width?: number | "auto" | `${number}%`
  height?: number
  position?: "absolute" | "relative"
  left?: number
  top?: number
  placeholder?: string
  theme?: Partial<ThemeColors>
  maxLines?: number
  historySize?: number
}

interface InputHistory {
  entries: string[]
  index: number
  tempEntry: string  // Current input before browsing history
}

export class MultilineInput extends BoxRenderable {
  private lines: string[] = [""]
  private cursor: CursorPosition = { line: 0, col: 0 }
  private selection: TextSelection = { start: { line: 0, col: 0 }, end: { line: 0, col: 0 }, active: false }
  private scrollOffset: number = 0
  private horizontalScroll: number = 0
  private keyHandler: ((key: KeyEvent) => void) | null = null
  private emitter = new EventEmitter()
  private history: InputHistory = { entries: [], index: -1, tempEntry: "" }
  private undoStack: string[][] = []
  private redoStack: string[][] = []
  private lastSaveTime: number = 0
  private readonly UNDO_INTERVAL = 500  // Group changes within 500ms

  // Configuration
  private placeholder: string
  private maxLines: number
  private historySize: number

  // Colors (using theme)
  private textColor: RGBA
  private cursorColor: RGBA
  private placeholderColor: RGBA
  private selectionColor: RGBA
  private lineNumberColor: RGBA

  constructor(ctx: RenderContext, options: MultilineInputOptions) {
    super(ctx, {
      id: options.id,
      width: options.width ?? 80,
      height: options.height ?? 10,
      position: options.position ?? "relative",
      left: options.left,
      top: options.top,
      backgroundColor: "#1e1e2e",
      borderColor: "#404060",
      borderStyle: "rounded",
      border: true,
    })

    this.placeholder = options.placeholder ?? "Type your message... (ESC or Ctrl+Enter to submit)"
    this.maxLines = options.maxLines ?? 1000
    this.historySize = options.historySize ?? 100

    // Theme colors
    this.textColor = RGBA.fromHex("#e0e0e0")
    this.cursorColor = RGBA.fromHex("#00ff88")
    this.placeholderColor = RGBA.fromHex("#606080")
    this.selectionColor = RGBA.fromHex("#3a3a6e")
    this.lineNumberColor = RGBA.fromHex("#505070")

    this.setupKeyHandler()
    this.saveUndoState()
  }

  private setupKeyHandler(): void {
    this.keyHandler = (key: KeyEvent) => {
      if (!this._focused) return
      this.handleKeypress(key)
    }
    this.ctx.keyInput.on("keypress", this.keyHandler)
  }

  private handleKeypress(key: KeyEvent): void {
    // === SUBMISSION ===
    // Escape with content = submit
    if (key.name === "escape" && this.getValue().trim().length > 0) {
      this.submitAndSaveHistory()
      return
    }

    // Ctrl+Enter, Alt+Enter, or Cmd+Enter = submit
    if ((key.ctrl || key.meta || key.option) && key.name === "return") {
      this.submitAndSaveHistory()
      return
    }

    // Ctrl+D = submit
    if (key.ctrl && key.name === "d" && this.getValue().trim().length > 0) {
      this.submitAndSaveHistory()
      return
    }

    // === HISTORY NAVIGATION ===
    // Ctrl+Up = previous history
    if (key.ctrl && key.name === "up") {
      this.navigateHistory(-1)
      return
    }

    // Ctrl+Down = next history
    if (key.ctrl && key.name === "down") {
      this.navigateHistory(1)
      return
    }

    // === UNDO/REDO ===
    if (key.ctrl && key.name === "z") {
      if (key.shift) {
        this.redo()
      } else {
        this.undo()
      }
      return
    }

    if (key.ctrl && key.name === "y") {
      this.redo()
      return
    }

    // === SELECTION ===
    // Shift+Arrow = extend selection
    if (key.shift && ["up", "down", "left", "right", "home", "end"].includes(key.name ?? "")) {
      this.extendSelection(key.name as string, key.ctrl)
      return
    }

    // Ctrl+A = select all
    if (key.ctrl && key.name === "a") {
      this.selectAll()
      return
    }

    // Clear selection on non-shift navigation
    if (this.selection.active && !key.shift && ["up", "down", "left", "right", "home", "end"].includes(key.name ?? "")) {
      this.clearSelection()
    }

    // === CLIPBOARD ===
    // Ctrl+C = copy (if selection)
    if (key.ctrl && key.name === "c" && this.selection.active) {
      this.copy()
      return
    }

    // Ctrl+X = cut (if selection)
    if (key.ctrl && key.name === "x" && this.selection.active) {
      this.cut()
      return
    }

    // Ctrl+V = paste
    if (key.ctrl && key.name === "v") {
      this.paste()
      return
    }

    // === LINE OPERATIONS ===
    // Ctrl+K = kill to end of line
    if (key.ctrl && key.name === "k") {
      this.killLine()
      return
    }

    // Ctrl+U = kill entire line
    if (key.ctrl && key.name === "u") {
      this.killFullLine()
      return
    }

    // Ctrl+W = delete word backward
    if (key.ctrl && key.name === "w") {
      this.deleteWordBackward()
      return
    }

    // Ctrl+Backspace / Alt+Backspace = delete word backward
    if ((key.ctrl || key.option) && key.name === "backspace") {
      this.deleteWordBackward()
      return
    }

    // Ctrl+Delete = delete word forward
    if (key.ctrl && key.name === "delete") {
      this.deleteWordForward()
      return
    }

    // === WORD NAVIGATION ===
    // Ctrl+Left = word left
    if (key.ctrl && key.name === "left") {
      this.moveWordLeft()
      return
    }

    // Ctrl+Right = word right
    if (key.ctrl && key.name === "right") {
      this.moveWordRight()
      return
    }

    // === BASIC EDITING ===
    // Newline
    if (key.name === "return") {
      this.insertNewline()
      return
    }

    // Tab
    if (key.name === "tab" && !key.shift) {
      this.insertTab()
      return
    }

    // Shift+Tab = outdent
    if (key.name === "tab" && key.shift) {
      this.outdentLine()
      return
    }

    // Backspace
    if (key.name === "backspace") {
      this.handleBackspace()
      return
    }

    // Delete
    if (key.name === "delete") {
      this.handleDelete()
      return
    }

    // === NAVIGATION ===
    if (key.name === "up") {
      this.moveCursorUp()
      return
    }

    if (key.name === "down") {
      this.moveCursorDown()
      return
    }

    if (key.name === "left") {
      this.moveCursorLeft()
      return
    }

    if (key.name === "right") {
      this.moveCursorRight()
      return
    }

    if (key.name === "home") {
      if (key.ctrl) {
        this.moveCursorToStart()
      } else {
        this.moveCursorToLineStart()
      }
      return
    }

    if (key.name === "end") {
      if (key.ctrl) {
        this.moveCursorToEnd()
      } else {
        this.moveCursorToLineEnd()
      }
      return
    }

    if (key.name === "pageup") {
      this.pageUp()
      return
    }

    if (key.name === "pagedown") {
      this.pageDown()
      return
    }

    // === CHARACTER INPUT ===
    if (key.sequence && key.sequence.length === 1 && !key.ctrl && !key.meta) {
      this.insertChar(key.sequence)
    }
  }

  // === INPUT METHODS ===

  private insertChar(char: string): void {
    this.deleteSelectionIfActive()
    this.maybeSaveUndo()

    const line = this.lines[this.cursor.line]
    this.lines[this.cursor.line] =
      line.slice(0, this.cursor.col) + char + line.slice(this.cursor.col)
    this.cursor.col++
    this.markDirty()
  }

  private insertNewline(): void {
    this.deleteSelectionIfActive()
    this.maybeSaveUndo()

    const line = this.lines[this.cursor.line]
    const beforeCursor = line.slice(0, this.cursor.col)
    const afterCursor = line.slice(this.cursor.col)

    // Auto-indent: match leading whitespace of current line
    const indent = beforeCursor.match(/^[\t ]*/)?.[0] ?? ""

    this.lines[this.cursor.line] = beforeCursor
    this.lines.splice(this.cursor.line + 1, 0, indent + afterCursor)
    this.cursor.line++
    this.cursor.col = indent.length
    this.updateScrollOffset()
    this.markDirty()
  }

  private insertTab(): void {
    this.deleteSelectionIfActive()
    this.maybeSaveUndo()

    // Insert 2 spaces (configurable)
    const tabStr = "  "
    const line = this.lines[this.cursor.line]
    this.lines[this.cursor.line] =
      line.slice(0, this.cursor.col) + tabStr + line.slice(this.cursor.col)
    this.cursor.col += tabStr.length
    this.markDirty()
  }

  private outdentLine(): void {
    this.maybeSaveUndo()
    const line = this.lines[this.cursor.line]

    // Remove up to 2 leading spaces or 1 tab
    if (line.startsWith("  ")) {
      this.lines[this.cursor.line] = line.slice(2)
      this.cursor.col = Math.max(0, this.cursor.col - 2)
    } else if (line.startsWith("\t")) {
      this.lines[this.cursor.line] = line.slice(1)
      this.cursor.col = Math.max(0, this.cursor.col - 1)
    } else if (line.startsWith(" ")) {
      this.lines[this.cursor.line] = line.slice(1)
      this.cursor.col = Math.max(0, this.cursor.col - 1)
    }
    this.markDirty()
  }

  private handleBackspace(): void {
    if (this.deleteSelectionIfActive()) return
    this.maybeSaveUndo()

    if (this.cursor.col > 0) {
      const line = this.lines[this.cursor.line]
      this.lines[this.cursor.line] =
        line.slice(0, this.cursor.col - 1) + line.slice(this.cursor.col)
      this.cursor.col--
    } else if (this.cursor.line > 0) {
      const currentLine = this.lines[this.cursor.line]
      this.cursor.line--
      this.cursor.col = this.lines[this.cursor.line].length
      this.lines[this.cursor.line] += currentLine
      this.lines.splice(this.cursor.line + 1, 1)
      this.updateScrollOffset()
    }
    this.markDirty()
  }

  private handleDelete(): void {
    if (this.deleteSelectionIfActive()) return
    this.maybeSaveUndo()

    const line = this.lines[this.cursor.line]
    if (this.cursor.col < line.length) {
      this.lines[this.cursor.line] =
        line.slice(0, this.cursor.col) + line.slice(this.cursor.col + 1)
    } else if (this.cursor.line < this.lines.length - 1) {
      this.lines[this.cursor.line] += this.lines[this.cursor.line + 1]
      this.lines.splice(this.cursor.line + 1, 1)
    }
    this.markDirty()
  }

  // === WORD OPERATIONS ===

  private moveWordLeft(): void {
    if (this.cursor.col === 0 && this.cursor.line > 0) {
      this.cursor.line--
      this.cursor.col = this.lines[this.cursor.line].length
    } else {
      const line = this.lines[this.cursor.line]
      let col = this.cursor.col

      // Skip whitespace
      while (col > 0 && /\s/.test(line[col - 1])) col--
      // Skip word characters
      while (col > 0 && /\w/.test(line[col - 1])) col--

      this.cursor.col = col
    }
    this.updateScrollOffset()
    this.markDirty()
  }

  private moveWordRight(): void {
    const line = this.lines[this.cursor.line]
    if (this.cursor.col >= line.length && this.cursor.line < this.lines.length - 1) {
      this.cursor.line++
      this.cursor.col = 0
    } else {
      let col = this.cursor.col

      // Skip word characters
      while (col < line.length && /\w/.test(line[col])) col++
      // Skip whitespace
      while (col < line.length && /\s/.test(line[col])) col++

      this.cursor.col = col
    }
    this.updateScrollOffset()
    this.markDirty()
  }

  private deleteWordBackward(): void {
    this.maybeSaveUndo()
    const startCol = this.cursor.col

    if (this.cursor.col === 0 && this.cursor.line > 0) {
      // Join with previous line
      const currentLine = this.lines[this.cursor.line]
      this.cursor.line--
      this.cursor.col = this.lines[this.cursor.line].length
      this.lines[this.cursor.line] += currentLine
      this.lines.splice(this.cursor.line + 1, 1)
    } else {
      const line = this.lines[this.cursor.line]
      let col = this.cursor.col

      // Skip whitespace
      while (col > 0 && /\s/.test(line[col - 1])) col--
      // Skip word characters
      while (col > 0 && /\w/.test(line[col - 1])) col--

      this.lines[this.cursor.line] = line.slice(0, col) + line.slice(startCol)
      this.cursor.col = col
    }
    this.updateScrollOffset()
    this.markDirty()
  }

  private deleteWordForward(): void {
    this.maybeSaveUndo()
    const line = this.lines[this.cursor.line]
    let endCol = this.cursor.col

    // Skip word characters
    while (endCol < line.length && /\w/.test(line[endCol])) endCol++
    // Skip whitespace
    while (endCol < line.length && /\s/.test(line[endCol])) endCol++

    if (endCol === this.cursor.col && this.cursor.line < this.lines.length - 1) {
      // Join with next line
      this.lines[this.cursor.line] += this.lines[this.cursor.line + 1]
      this.lines.splice(this.cursor.line + 1, 1)
    } else {
      this.lines[this.cursor.line] = line.slice(0, this.cursor.col) + line.slice(endCol)
    }
    this.markDirty()
  }

  private killLine(): void {
    this.maybeSaveUndo()
    const line = this.lines[this.cursor.line]
    if (this.cursor.col < line.length) {
      this.lines[this.cursor.line] = line.slice(0, this.cursor.col)
    } else if (this.cursor.line < this.lines.length - 1) {
      // Kill newline, join with next line
      this.lines[this.cursor.line] += this.lines[this.cursor.line + 1]
      this.lines.splice(this.cursor.line + 1, 1)
    }
    this.markDirty()
  }

  private killFullLine(): void {
    this.maybeSaveUndo()
    if (this.lines.length > 1) {
      this.lines.splice(this.cursor.line, 1)
      if (this.cursor.line >= this.lines.length) {
        this.cursor.line = this.lines.length - 1
      }
      this.cursor.col = Math.min(this.cursor.col, this.lines[this.cursor.line].length)
    } else {
      this.lines[0] = ""
      this.cursor.col = 0
    }
    this.updateScrollOffset()
    this.markDirty()
  }

  // === CURSOR NAVIGATION ===

  private moveCursorUp(): void {
    if (this.cursor.line > 0) {
      this.cursor.line--
      this.cursor.col = Math.min(this.cursor.col, this.lines[this.cursor.line].length)
      this.updateScrollOffset()
      this.markDirty()
    }
  }

  private moveCursorDown(): void {
    if (this.cursor.line < this.lines.length - 1) {
      this.cursor.line++
      this.cursor.col = Math.min(this.cursor.col, this.lines[this.cursor.line].length)
      this.updateScrollOffset()
      this.markDirty()
    }
  }

  private moveCursorLeft(): void {
    if (this.cursor.col > 0) {
      this.cursor.col--
    } else if (this.cursor.line > 0) {
      this.cursor.line--
      this.cursor.col = this.lines[this.cursor.line].length
      this.updateScrollOffset()
    }
    this.markDirty()
  }

  private moveCursorRight(): void {
    const line = this.lines[this.cursor.line]
    if (this.cursor.col < line.length) {
      this.cursor.col++
    } else if (this.cursor.line < this.lines.length - 1) {
      this.cursor.line++
      this.cursor.col = 0
      this.updateScrollOffset()
    }
    this.markDirty()
  }

  private moveCursorToLineStart(): void {
    // Smart home: go to first non-whitespace, or to column 0
    const line = this.lines[this.cursor.line]
    const firstNonWhitespace = line.search(/\S/)
    if (firstNonWhitespace > 0 && this.cursor.col !== firstNonWhitespace) {
      this.cursor.col = firstNonWhitespace
    } else {
      this.cursor.col = 0
    }
    this.markDirty()
  }

  private moveCursorToLineEnd(): void {
    this.cursor.col = this.lines[this.cursor.line].length
    this.markDirty()
  }

  private moveCursorToStart(): void {
    this.cursor = { line: 0, col: 0 }
    this.scrollOffset = 0
    this.markDirty()
  }

  private moveCursorToEnd(): void {
    this.cursor.line = this.lines.length - 1
    this.cursor.col = this.lines[this.cursor.line].length
    this.updateScrollOffset()
    this.markDirty()
  }

  private pageUp(): void {
    const visibleHeight = this.height - 2
    this.cursor.line = Math.max(0, this.cursor.line - visibleHeight)
    this.cursor.col = Math.min(this.cursor.col, this.lines[this.cursor.line].length)
    this.updateScrollOffset()
    this.markDirty()
  }

  private pageDown(): void {
    const visibleHeight = this.height - 2
    this.cursor.line = Math.min(this.lines.length - 1, this.cursor.line + visibleHeight)
    this.cursor.col = Math.min(this.cursor.col, this.lines[this.cursor.line].length)
    this.updateScrollOffset()
    this.markDirty()
  }

  // === SELECTION ===

  private selectAll(): void {
    this.selection = {
      start: { line: 0, col: 0 },
      end: { line: this.lines.length - 1, col: this.lines[this.lines.length - 1].length },
      active: true,
    }
    this.markDirty()
  }

  private extendSelection(direction: string, ctrl: boolean = false): void {
    if (!this.selection.active) {
      this.selection = {
        start: { ...this.cursor },
        end: { ...this.cursor },
        active: true,
      }
    }

    // Move cursor and update selection end
    const prevCursor = { ...this.cursor }
    switch (direction) {
      case "up":
        this.moveCursorUp()
        break
      case "down":
        this.moveCursorDown()
        break
      case "left":
        ctrl ? this.moveWordLeft() : this.moveCursorLeft()
        break
      case "right":
        ctrl ? this.moveWordRight() : this.moveCursorRight()
        break
      case "home":
        ctrl ? this.moveCursorToStart() : this.moveCursorToLineStart()
        break
      case "end":
        ctrl ? this.moveCursorToEnd() : this.moveCursorToLineEnd()
        break
    }
    this.selection.end = { ...this.cursor }
    this.markDirty()
  }

  private clearSelection(): void {
    this.selection.active = false
    this.markDirty()
  }

  private deleteSelectionIfActive(): boolean {
    if (!this.selection.active) return false

    this.maybeSaveUndo()

    const [start, end] = this.normalizeSelection()

    if (start.line === end.line) {
      const line = this.lines[start.line]
      this.lines[start.line] = line.slice(0, start.col) + line.slice(end.col)
    } else {
      const startLine = this.lines[start.line].slice(0, start.col)
      const endLine = this.lines[end.line].slice(end.col)
      this.lines.splice(start.line, end.line - start.line + 1, startLine + endLine)
    }

    this.cursor = { ...start }
    this.selection.active = false
    this.updateScrollOffset()
    this.markDirty()
    return true
  }

  private normalizeSelection(): [CursorPosition, CursorPosition] {
    const { start, end } = this.selection
    if (start.line < end.line || (start.line === end.line && start.col <= end.col)) {
      return [start, end]
    }
    return [end, start]
  }

  private getSelectedText(): string {
    if (!this.selection.active) return ""

    const [start, end] = this.normalizeSelection()
    if (start.line === end.line) {
      return this.lines[start.line].slice(start.col, end.col)
    }

    const parts: string[] = []
    parts.push(this.lines[start.line].slice(start.col))
    for (let i = start.line + 1; i < end.line; i++) {
      parts.push(this.lines[i])
    }
    parts.push(this.lines[end.line].slice(0, end.col))
    return parts.join("\n")
  }

  // === CLIPBOARD ===

  private copy(): void {
    const text = this.getSelectedText()
    if (text) {
      // Store in internal clipboard (process.env doesn't work in all terminals)
      (globalThis as any).__clipboardContent = text
      this.emitter.emit("copy", text)
    }
  }

  private cut(): void {
    const text = this.getSelectedText()
    if (text) {
      (globalThis as any).__clipboardContent = text
      this.emitter.emit("copy", text)
      this.deleteSelectionIfActive()
    }
  }

  private paste(): void {
    const text = (globalThis as any).__clipboardContent as string | undefined
    if (text) {
      this.deleteSelectionIfActive()
      this.maybeSaveUndo()

      const pasteLines = text.split("\n")
      if (pasteLines.length === 1) {
        // Insert text directly (insertChar is for single chars)
        const line = this.lines[this.cursor.line]
        this.lines[this.cursor.line] =
          line.slice(0, this.cursor.col) + pasteLines[0] + line.slice(this.cursor.col)
        this.cursor.col += pasteLines[0].length
      } else {
        const line = this.lines[this.cursor.line]
        const before = line.slice(0, this.cursor.col)
        const after = line.slice(this.cursor.col)

        this.lines[this.cursor.line] = before + pasteLines[0]
        for (let i = 1; i < pasteLines.length; i++) {
          this.lines.splice(this.cursor.line + i, 0, pasteLines[i])
        }
        this.lines[this.cursor.line + pasteLines.length - 1] += after
        this.cursor.line += pasteLines.length - 1
        this.cursor.col = pasteLines[pasteLines.length - 1].length
      }
      this.updateScrollOffset()
      this.markDirty()
    }
  }

  // === UNDO/REDO ===

  private saveUndoState(): void {
    this.undoStack.push([...this.lines])
    this.lastSaveTime = Date.now()
    // Limit undo stack size
    if (this.undoStack.length > 100) {
      this.undoStack.shift()
    }
    // Clear redo stack on new change
    this.redoStack = []
  }

  private maybeSaveUndo(): void {
    const now = Date.now()
    if (now - this.lastSaveTime > this.UNDO_INTERVAL) {
      this.saveUndoState()
    }
  }

  private undo(): void {
    if (this.undoStack.length > 1) {
      this.redoStack.push([...this.lines])
      this.undoStack.pop()
      this.lines = [...this.undoStack[this.undoStack.length - 1]]
      this.cursor = { line: 0, col: 0 }
      this.selection.active = false
      this.updateScrollOffset()
      this.markDirty()
    }
  }

  private redo(): void {
    if (this.redoStack.length > 0) {
      const state = this.redoStack.pop()!
      this.undoStack.push([...this.lines])
      this.lines = state
      this.cursor = { line: 0, col: 0 }
      this.selection.active = false
      this.updateScrollOffset()
      this.markDirty()
    }
  }

  // === INPUT HISTORY ===

  private navigateHistory(direction: number): void {
    if (this.history.entries.length === 0) return

    // Save current input if starting to browse
    if (this.history.index === -1) {
      this.history.tempEntry = this.getValue()
    }

    const newIndex = this.history.index + direction
    if (newIndex < -1) return
    if (newIndex >= this.history.entries.length) return

    this.history.index = newIndex

    if (newIndex === -1) {
      // Back to current input
      this.setValue(this.history.tempEntry)
    } else {
      this.setValue(this.history.entries[this.history.entries.length - 1 - newIndex])
    }
    this.moveCursorToEnd()
  }

  private submitAndSaveHistory(): void {
    const value = this.getValue().trim()
    if (!value) return

    // Add to history if different from last entry
    const lastEntry = this.history.entries[this.history.entries.length - 1]
    if (value !== lastEntry) {
      this.history.entries.push(value)
      if (this.history.entries.length > this.historySize) {
        this.history.entries.shift()
      }
    }
    this.history.index = -1
    this.history.tempEntry = ""

    this.emitter.emit("submit", this.getValue())
  }

  // === SCROLLING ===

  private updateScrollOffset(): void {
    const visibleHeight = this.height - 2

    // Scroll down if cursor below visible area
    if (this.cursor.line >= this.scrollOffset + visibleHeight) {
      this.scrollOffset = this.cursor.line - visibleHeight + 1
    }

    // Scroll up if cursor above visible area
    if (this.cursor.line < this.scrollOffset) {
      this.scrollOffset = this.cursor.line
    }
  }

  // === RENDERING ===

  protected renderSelf(buffer: OptimizedBuffer): void {
    super.renderSelf(buffer)

    const visibleHeight = this.height - 2
    const visibleWidth = this.width - 2
    const lineNumWidth = Math.max(3, String(this.lines.length).length + 1)
    const contentWidth = visibleWidth - lineNumWidth

    // Show placeholder if empty and unfocused
    if (this.lines.length === 1 && this.lines[0] === "" && !this._focused) {
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

      const lineY = this.y + 1 + i

      // Line number
      const lineNum = String(lineIndex + 1).padStart(lineNumWidth - 1, " ")
      buffer.drawText(lineNum, this.x + 1, lineY, this.lineNumberColor)

      // Line content
      const line = this.lines[lineIndex]
      const displayLine = this.expandTabs(line)

      // Render text first
      buffer.drawText(
        displayLine.slice(0, contentWidth),
        this.x + lineNumWidth + 1,
        lineY,
        this.textColor
      )

      // Render selection on top (with text preserved)
      if (this.selection.active) {
        this.renderSelectionLine(buffer, lineIndex, lineY, lineNumWidth, displayLine)
      }

      // Render cursor if focused and on this line
      if (this._focused && lineIndex === this.cursor.line) {
        const cursorCol = this.expandTabs(line.slice(0, this.cursor.col)).length
        const cursorX = this.x + lineNumWidth + 1 + cursorCol

        if (cursorX < this.x + this.width - 1) {
          const charUnderCursor = line[this.cursor.col] ?? " "
          buffer.setCell(cursorX, lineY, charUnderCursor, RGBA.fromHex("#000000"), this.cursorColor)
        }
      }
    }

    // Scroll indicator if needed
    if (this.lines.length > visibleHeight) {
      const scrollPercent = this.scrollOffset / Math.max(1, this.lines.length - visibleHeight)
      const indicatorY = this.y + 1 + Math.floor(scrollPercent * (visibleHeight - 1))
      buffer.drawText("│", this.x + this.width - 1, indicatorY, this.cursorColor)
    }
  }

  private renderSelectionLine(buffer: OptimizedBuffer, lineIndex: number, lineY: number, lineNumWidth: number, displayLine: string): void {
    const [start, end] = this.normalizeSelection()

    if (lineIndex < start.line || lineIndex > end.line) return

    const line = this.lines[lineIndex]
    let selStart = 0
    let selEnd = line.length

    if (lineIndex === start.line) selStart = start.col
    if (lineIndex === end.line) selEnd = end.col

    const displayStart = this.expandTabs(line.slice(0, selStart)).length
    const displayEnd = this.expandTabs(line.slice(0, selEnd)).length

    for (let col = displayStart; col < displayEnd && col < this.width - lineNumWidth - 2; col++) {
      const x = this.x + lineNumWidth + 1 + col
      // Preserve the actual character, just change the background
      const char = displayLine[col] ?? " "
      buffer.setCell(x, lineY, char, this.textColor, this.selectionColor)
    }
  }

  private expandTabs(text: string, tabWidth: number = 2): string {
    return text.replace(/\t/g, " ".repeat(tabWidth))
  }

  // === PUBLIC API ===

  public focus(): void {
    this._focused = true
    this.borderColor = RGBA.fromHex("#00ff88")
    this.markDirty()
  }

  public blur(): void {
    this._focused = false
    this.borderColor = RGBA.fromHex("#404060")
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
    this.selection.active = false
    this.saveUndoState()
    this.markDirty()
  }

  public clear(): void {
    this.lines = [""]
    this.cursor = { line: 0, col: 0 }
    this.scrollOffset = 0
    this.selection.active = false
    this.saveUndoState()
    this.markDirty()
  }

  public getCursor(): CursorPosition {
    return { ...this.cursor }
  }

  public getLineCount(): number {
    return this.lines.length
  }

  public on(event: "submit" | "copy", handler: (value: string) => void): this {
    this.emitter.on(event, handler)
    return this
  }

  public destroy(): void {
    if (this.keyHandler) {
      this.ctx.keyInput.off("keypress", this.keyHandler)
      this.keyHandler = null
    }
    super.destroy()
  }
}
