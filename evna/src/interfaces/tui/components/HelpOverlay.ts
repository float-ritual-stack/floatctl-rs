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
      { key: "ESC", description: "Submit message", action: "submit" },
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
      { key: "Ctrl+H", description: "Toggle this help", action: "toggle_help" },
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
  private visible: boolean = false
  private keyHandler: ((key: KeyEvent) => void) | null = null
  private onClose: (() => void) | null = null

  constructor(ctx: RenderContext, options: HelpOverlayOptions) {
    const width = options.width ?? 70
    const height = options.height ?? 35

    super(ctx, {
      id: options.id,
      width,
      height,
      position: "absolute",
      // Center the overlay (default 80x24 if not a TTY)
      left: Math.floor(((process.stdout.columns ?? 80) - width) / 2),
      top: Math.floor(((process.stdout.rows ?? 24) - height) / 2),
      backgroundColor: "#1a1a2e",
      borderColor: "#00ff88",
      borderStyle: "double",
      border: true,
      zIndex: 100,
    })

    this.buildContent()
    this.setupKeyHandler()

    // Start hidden
    this.hide()
  }

  private buildContent(): void {
    // Title
    const title = new TextRenderable(this.ctx, {
      id: `${this.id}-title`,
      content: t`${bold(fg("#00ff88")("  EVNA Chat - Keyboard Shortcuts  "))}`,
      position: "relative",
      paddingBottom: 1,
    })
    this.add(title)

    // Separator
    const sep = new TextRenderable(this.ctx, {
      id: `${this.id}-sep`,
      content: t`${fg("#404060")("─".repeat(66))}`,
      position: "relative",
      paddingBottom: 1,
    })
    this.add(sep)

    // Sections
    let sectionNum = 0
    for (const section of HELP_SECTIONS) {
      // Section title
      const sectionTitle = new TextRenderable(this.ctx, {
        id: `${this.id}-section-${sectionNum}`,
        content: t`${bold(fg("#00aaff")(section.title))}`,
        position: "relative",
        paddingTop: sectionNum > 0 ? 1 : 0,
      })
      this.add(sectionTitle)

      // Shortcuts
      for (let i = 0; i < section.shortcuts.length; i++) {
        const shortcut = section.shortcuts[i]
        const keyStr = shortcut.key.padEnd(16)
        const shortcutText = new TextRenderable(this.ctx, {
          id: `${this.id}-shortcut-${sectionNum}-${i}`,
          content: t`  ${fg("#ffd700")(keyStr)} ${fg("#b0b0b0")(shortcut.description)}`,
          position: "relative",
        })
        this.add(shortcutText)
      }

      sectionNum++
    }

    // Footer
    const footer = new TextRenderable(this.ctx, {
      id: `${this.id}-footer`,
      content: t`\n${fg("#606080")("Press any key to close")}`,
      position: "relative",
      paddingTop: 1,
    })
    this.add(footer)
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
    this.visible = true
    this.onClose = onClose ?? null

    // Recenter in case terminal size changed (default 80x24 if not a TTY)
    const width = this.width as number
    const height = this.height as number
    this.left = Math.floor(((process.stdout.columns ?? 80) - width) / 2)
    this.top = Math.floor(((process.stdout.rows ?? 24) - height) / 2)

    // Make visible
    super.show?.()
    this.markDirty()
  }

  public hide(): void {
    this.visible = false
    super.hide?.()
    this.markDirty()
  }

  public toggle(onClose?: () => void): void {
    if (this.visible) {
      this.hide()
    } else {
      this.show(onClose)
    }
  }

  public isVisible(): boolean {
    return this.visible
  }

  public destroy(): void {
    if (this.keyHandler) {
      this.ctx.keyInput.off("keypress", this.keyHandler)
      this.keyHandler = null
    }
    super.destroy()
  }
}
