/**
 * Session Manager
 * Handles conversation persistence, save/load, and session history
 */

import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync, unlinkSync } from "fs"
import { join } from "path"
import { homedir } from "os"
import type {
  Session,
  SessionSummary,
  AgentMessage,
  TokenStats,
} from "../types.js"

// ============================================================================
// Session Manager Configuration
// ============================================================================

const SESSION_DIR = join(homedir(), ".evna", "sessions")
const MAX_SESSIONS = 50  // Keep last 50 sessions

// Basic session validation (guards against malformed data)
function isValidSession(obj: unknown): obj is Session {
  if (!obj || typeof obj !== "object") return false
  const s = obj as Record<string, unknown>
  return (
    typeof s.id === "string" &&
    typeof s.name === "string" &&
    Array.isArray(s.messages) &&
    typeof s.createdAt === "string"
  )
}

// ============================================================================
// Session Manager Class
// ============================================================================

export class SessionManager {
  private sessionsDir: string

  constructor(sessionsDir?: string) {
    this.sessionsDir = sessionsDir ?? SESSION_DIR
    this.ensureSessionDir()
  }

  private ensureSessionDir(): void {
    if (!existsSync(this.sessionsDir)) {
      mkdirSync(this.sessionsDir, { recursive: true })
    }
  }

  // === Session CRUD ===

  public createSession(name?: string): Session {
    const id = `session_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`
    const now = Date.now()

    const session: Session = {
      id,
      name: name ?? this.generateSessionName(),
      messages: [],
      createdAt: now,
      updatedAt: now,
      totalTokens: { input: 0, output: 0, cached: 0 },
    }

    return session
  }

  public saveSession(session: Session): void {
    const filePath = join(this.sessionsDir, `${session.id}.json`)
    session.updatedAt = Date.now()

    try {
      writeFileSync(filePath, JSON.stringify(session, null, 2))
      this.pruneOldSessions()
    } catch (error) {
      console.error(`Failed to save session: ${error}`)
      throw error
    }
  }

  public loadSession(sessionId: string): Session | null {
    const filePath = join(this.sessionsDir, `${sessionId}.json`)

    if (!existsSync(filePath)) {
      return null
    }

    try {
      const content = readFileSync(filePath, "utf-8")
      const parsed = JSON.parse(content)
      if (!isValidSession(parsed)) {
        console.error(`Invalid session format: ${sessionId}`)
        return null
      }
      return parsed
    } catch (error) {
      console.error(`Failed to load session ${sessionId}: ${error}`)
      return null
    }
  }

  public deleteSession(sessionId: string): boolean {
    const filePath = join(this.sessionsDir, `${sessionId}.json`)

    if (!existsSync(filePath)) {
      return false
    }

    try {
      unlinkSync(filePath)
      return true
    } catch (error) {
      console.error(`Failed to delete session ${sessionId}: ${error}`)
      return false
    }
  }

  // === Session Listing ===

  public listSessions(): SessionSummary[] {
    this.ensureSessionDir()

    const files = readdirSync(this.sessionsDir)
      .filter((f) => f.endsWith(".json"))
      .sort()
      .reverse()

    const summaries: SessionSummary[] = []

    for (const file of files) {
      try {
        const content = readFileSync(join(this.sessionsDir, file), "utf-8")
        const session = JSON.parse(content) as Session

        // Extract last message preview
        let lastMessage = "(empty)"
        if (session.messages.length > 0) {
          const lastMsg = session.messages[session.messages.length - 1]
          const textContent = lastMsg.content.find((b) => b.type === "text")
          if (textContent && "text" in textContent) {
            lastMessage = textContent.text.slice(0, 50)
            if (textContent.text.length > 50) lastMessage += "..."
          }
        }

        summaries.push({
          id: session.id,
          name: session.name,
          messageCount: session.messages.length,
          lastMessage,
          updatedAt: session.updatedAt,
        })
      } catch {
        // Skip invalid files
        continue
      }
    }

    return summaries
  }

  public getRecentSessions(limit: number = 10): SessionSummary[] {
    return this.listSessions().slice(0, limit)
  }

  // === Session Search ===

  public searchSessions(query: string): SessionSummary[] {
    const queryLower = query.toLowerCase()
    const all = this.listSessions()

    return all.filter((s) => {
      // Search in name
      if (s.name.toLowerCase().includes(queryLower)) return true

      // Search in last message
      if (s.lastMessage.toLowerCase().includes(queryLower)) return true

      return false
    })
  }

  // === Auto-save ===

  private autoSaveTimer: NodeJS.Timeout | null = null
  private pendingSession: Session | null = null

  public enableAutoSave(session: Session, intervalMs: number = 30000): void {
    this.pendingSession = session

    if (this.autoSaveTimer) {
      clearInterval(this.autoSaveTimer)
    }

    this.autoSaveTimer = setInterval(() => {
      if (this.pendingSession && this.pendingSession.messages.length > 0) {
        this.saveSession(this.pendingSession)
      }
    }, intervalMs)
  }

  public disableAutoSave(): void {
    if (this.autoSaveTimer) {
      clearInterval(this.autoSaveTimer)
      this.autoSaveTimer = null
    }
    this.pendingSession = null
  }

  public updatePendingSession(session: Session): void {
    this.pendingSession = session
  }

  // === Utilities ===

  private generateSessionName(): string {
    const now = new Date()
    const date = now.toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
    })
    const time = now.toLocaleTimeString("en-US", {
      hour: "numeric",
      minute: "2-digit",
      hour12: true,
    })
    return `Chat ${date} ${time}`
  }

  private pruneOldSessions(): void {
    const sessions = this.listSessions()

    if (sessions.length > MAX_SESSIONS) {
      const toDelete = sessions.slice(MAX_SESSIONS)
      for (const session of toDelete) {
        this.deleteSession(session.id)
      }
    }
  }

  // === Export/Import ===

  public exportSession(session: Session): string {
    return JSON.stringify(session, null, 2)
  }

  public importSession(json: string): Session | null {
    try {
      const session = JSON.parse(json) as Session

      // Validate required fields
      if (!session.id || !session.messages || !Array.isArray(session.messages)) {
        return null
      }

      // Generate new ID to avoid conflicts
      session.id = `session_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`
      session.updatedAt = Date.now()

      return session
    } catch {
      return null
    }
  }

  // === Message Helpers ===

  public addMessageToSession(session: Session, message: AgentMessage): Session {
    return {
      ...session,
      messages: [...session.messages, message],
      updatedAt: Date.now(),
    }
  }

  public updateSessionTokens(session: Session, tokens: TokenStats): Session {
    return {
      ...session,
      totalTokens: {
        input: session.totalTokens.input + tokens.input,
        output: session.totalTokens.output + tokens.output,
        cached: session.totalTokens.cached + tokens.cached,
      },
      updatedAt: Date.now(),
    }
  }

  public clearSessionMessages(session: Session): Session {
    return {
      ...session,
      messages: [],
      totalTokens: { input: 0, output: 0, cached: 0 },
      updatedAt: Date.now(),
    }
  }

  // === Cleanup ===

  public destroy(): void {
    this.disableAutoSave()
  }
}

// Export singleton instance
export const sessionManager = new SessionManager()
