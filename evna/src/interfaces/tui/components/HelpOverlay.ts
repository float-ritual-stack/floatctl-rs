/**
 * Help Overlay Component
 * Displays keyboard shortcuts and help information
 */

import {
  BoxRenderable,
  TextRenderable,
  type RenderContext,
  type KeyEvent,
  t,
  bold,
  fg,
} from "@opentui/core"
import type { KeyboardShortcut } from "../types.js"

// ============================================================================
// Help Overlay Options
// ============================================================================

export interface HelpOverlayOptions {
  id: string
  width?: number
  height?: number
}

// ============================================================================
// Help Section
// ============================================================================

interface HelpSection {
  title: string
  shortcuts: KeyboardShortcut[]
}

const HELP_SECTIONS: HelpSection[] = [
  {
    title: "Submission",
    shortcuts: [
      { key: "Ctrl+Enter", description: "Submit message", action: "submit" },
      { key: "Ctrl+D", description: "Submit message", action: "submit" },
    ],
  },
  {
    title: "Navigation",
    shortcuts: [
      { key: "↑/↓", description: "Move cursor / Scroll history", action: "navigate" },
      { key: "←/→", description: "Move cursor", action: "navigate" },
      { key: "Ctrl+←/→", description: "Move by word", action: "word_nav" },
      { key: "Home/End", description: "Start/end of line", action: "line_nav" },
      { key: "Ctrl+Home/End", description: "Start/end of document", action: "doc_nav" },
      { key: "PgUp/PgDn", description: "Page up/down", action: "page_nav" },
    ],
  },
  {
    title: "Editing",
    shortcuts: [
      { key: "Enter", description: "New line", action: "newline" },
      { key: "Tab", description: "Insert indent", action: "indent" },
      { key: "Shift+Tab", description: "Outdent", action: "outdent" },
      { key: "Ctrl+K", description: "Kill to end of line", action: "kill_line" },
      { key: "Ctrl+U", description: "Kill entire line", action: "kill_full_line" },
      { key: "Ctrl+W", description: "Delete word backward", action: "delete_word" },
      { key: "Ctrl+Z", description: "Undo", action: "undo" },
      { key: "Ctrl+Shift+Z", description: "Redo", action: "redo" },
    ],
  },
  {
    title: "Selection & Clipboard",
    shortcuts: [
      { key: "Shift+Arrows", description: "Extend selection", action: "select" },
      { key: "Ctrl+A", description: "Select all", action: "select_all" },
      { key: "Ctrl+C", description: "Copy selection", action: "copy" },
      { key: "Ctrl+X", description: "Cut selection", action: "cut" },
      { key: "Ctrl+V", description: "Paste", action: "paste" },
    ],
  },
  {
    title: "Input History",
    shortcuts: [
      { key: "Ctrl+↑", description: "Previous input", action: "history_prev" },
      { key: "Ctrl+↓", description: "Next input", action: "history_next" },
    ],
  },
  {
    title: "Session & Display",
    shortcuts: [
      { key: "Ctrl+L", description: "Clear conversation", action: "clear" },
      { key: "Ctrl+S", description: "Save session", action: "save" },
      { key: "Ctrl+O", description: "Load session", action: "load" },
      { key: "Ctrl+N", description: "New session", action: "new_session" },
      { key: "Ctrl+T", description: "Toggle timestamps", action: "toggle_timestamps" },
      { key: "Ctrl+M", description: "Toggle compact mode", action: "toggle_compact" },
      { key: "F1", description: "Toggle this help", action: "toggle_help" },
    ],
  },
  {
    title: "Exit",
    shortcuts: [
      { key: "Ctrl+C", description: "Exit application", action: "exit" },
    ],
  },
]

// ============================================================================
// Help Overlay Component
// ============================================================================

export class HelpOverlay extends BoxRenderable {
  private _isOverlayVisible: boolean = false
  private keyHandler: ((key: KeyEvent) => void) | null = null
  private onClose: (() => void) | null = null

  constructor(ctx: RenderContext, options: HelpOverlayOptions) {
    const termCols = process.stdout.columns ?? 80
    const termRows = process.stdout.rows ?? 24

    // Size to fit terminal with some margin
    const width = Math.min(options.width ?? 70, termCols - 4)
    const height = Math.min(options.height ?? 35, termRows - 4)

    // Center in terminal, ensure non-negative
    const left = Math.max(0, Math.floor((termCols - width) / 2))
    const top = Math.max(0, Math.floor((termRows - height) / 2))

    super(ctx, {
      id: options.id,
      width,
      height,
      position: "absolute",
      left,
      top,
      flexDirection: "column",
      backgroundColor: "#1a1a2e",
      borderColor: "#00ff88",
      borderStyle: "double",
      border: true,
      zIndex: 100,
      overflow: "scroll",
      padding: 1,
    })

    this.buildContent()
    this.setupKeyHandler()

    // Start hidden
    this.hide()
  }

  private buildContent(): void {
    // Build all help content as plain text - no ANSI codes
    const lines: string[] = []

    lines.push("  EVNA Chat - Keyboard Shortcuts")
    lines.push("  " + "─".repeat(40))
    lines.push("")

    for (let s = 0; s < HELP_SECTIONS.length; s++) {
      const section = HELP_SECTIONS[s]
      if (s > 0) lines.push("")
      lines.push("  " + section.title)

      for (const shortcut of section.shortcuts) {
        lines.push("    " + shortcut.key.padEnd(16) + shortcut.description)
      }
    }

    lines.push("")
    lines.push("  Press any key to close")

    const content = new TextRenderable(this.ctx, {
      id: `${this.id}-content`,
      content: lines.join("\n"),
      position: "relative",
      fg: "#e0e0e0",
    })
    this.add(content)
  }

  private setupKeyHandler(): void {
    this.keyHandler = (key: KeyEvent) => {
      if (this.visible) {
        // Any key closes the help overlay
        this.hide()
        if (this.onClose) {
          this.onClose()
        }
      }
    }

    this.ctx.keyInput.on("keypress", this.keyHandler)
  }

  // === Public API ===

  public show(onClose?: () => void): void {
    this._isOverlayVisible = true
    this.onClose = onClose ?? null

    // Recalculate size AND position - terminal size may have changed or wasn't accurate at construction
    const termCols = process.stdout.columns ?? 80
    const termRows = process.stdout.rows ?? 24

    // Resize to fit current terminal
    const newWidth = Math.min(70, termCols - 4)
    const newHeight = Math.min(35, termRows - 4)
    this.width = newWidth
    this.height = newHeight

    // Center with bounds checking
    this.left = Math.max(0, Math.floor((termCols - newWidth) / 2))
    this.top = Math.max(0, Math.floor((termRows - newHeight) / 2))

    // Make visible using base class property
    this.visible = true
    this.markDirty()
  }

  public hide(): void {
    this._isOverlayVisible = false
    this.visible = false
    this.markDirty()
  }

  public toggle(onClose?: () => void): void {
    if (this._isOverlayVisible) {
      this.hide()
    } else {
      this.show(onClose)
    }
  }

  public isVisible(): boolean {
    return this._isOverlayVisible
  }

  public destroy(): void {
    if (this.keyHandler) {
      this.ctx.keyInput.off("keypress", this.keyHandler)
      this.keyHandler = null
    }
    super.destroy()
  }
}
