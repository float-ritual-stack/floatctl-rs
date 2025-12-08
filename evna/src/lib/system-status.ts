/**
 * System Status Module
 *
 * Provides system-wide status information that gets injected into MCP tool descriptions.
 * This is the "room's ambient state" - what you see when you enter the shared space.
 *
 * Status sources:
 * - [BBS] - Unread inbox count (from floatctl bbs inbox)
 * - [FOCUS] - Current work focus (from ~/.floatctl/status/focus.json)
 * - [NOTICE] - Sysop notices like break warnings (from ~/.floatctl/status/notice.json)
 *
 * Status file format (JSON):
 * {
 *   "content": "Issue #656 - GP node patterns",
 *   "set_at": "2025-12-07T14:30:00-05:00",  // Toronto time
 *   "set_by": "kitty"  // optional - who set it
 * }
 *
 * Philosophy: CLI for execution, MCP tool descriptions for ambient awareness.
 * Dynamic tool descriptions = notification channel for shared agent spaces.
 *
 * CLI commands: `floatctl status focus|notice|clear|show`
 */

import { readFile } from "fs/promises";
import { join } from "path";
import { homedir } from "os";
import { execFile } from "child_process";
import { promisify } from "util";

const execFileAsync = promisify(execFile);

/**
 * Status entry with timestamp metadata
 */
export interface StatusEntry {
  content: string;
  set_at: string;   // ISO 8601 timestamp (Toronto time)
  set_by?: string;  // Who set it (optional)
}

export interface SystemStatus {
  focus?: StatusEntry;    // Current work focus
  bbsUnread?: number;     // Unread BBS messages for this persona
  notice?: StatusEntry;   // Sysop notices (break warnings, meeting status)
  currentTime?: string;   // Current Toronto time for context
}

// Cache to avoid hammering BBS API on every tool list request
let statusCache: SystemStatus = {};
let cacheTime = 0;
const CACHE_TTL = 30000; // 30 seconds - balance freshness vs latency

/**
 * Get current Toronto time formatted nicely
 */
function getTorontoTime(): string {
  return new Date().toLocaleString('en-US', {
    timeZone: 'America/Toronto',
    weekday: 'short',
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
    hour12: true,
  });
}

/**
 * Format relative time (e.g., "15min ago", "3h ago")
 */
function formatTimeAgo(isoTimestamp: string): string {
  try {
    const setTime = new Date(isoTimestamp).getTime();
    const now = Date.now();
    const diffMs = now - setTime;
    const diffMins = Math.floor(diffMs / 60000);

    if (diffMins < 1) return 'just now';
    if (diffMins < 60) return `${diffMins}min ago`;
    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    const diffDays = Math.floor(diffHours / 24);
    return `${diffDays}d ago`;
  } catch {
    return '';
  }
}

/**
 * Read a status entry from JSON file (new format) or plain text (legacy)
 */
async function readStatusEntry(filePath: string): Promise<StatusEntry | null> {
  try {
    const content = await readFile(filePath, 'utf-8');
    const trimmed = content.trim();
    if (!trimmed) return null;

    // Try JSON first (new format)
    if (trimmed.startsWith('{')) {
      const parsed = JSON.parse(trimmed) as StatusEntry;
      if (parsed.content) return parsed;
    }

    // Fall back to plain text (legacy - just content, no timestamp)
    return {
      content: trimmed,
      set_at: new Date().toISOString(), // Unknown, use now
    };
  } catch {
    return null;
  }
}

/**
 * Fetch system status from multiple sources.
 * Gracefully handles failures - any source can fail without breaking the whole thing.
 */
export async function getSystemStatus(): Promise<SystemStatus> {
  const now = Date.now();

  // Return cached if still fresh
  if (now - cacheTime < CACHE_TTL && Object.keys(statusCache).length > 0) {
    return statusCache;
  }

  const status: SystemStatus = {};
  const statusDir = join(homedir(), '.floatctl', 'status');

  // Add current Toronto time
  status.currentTime = getTorontoTime();

  // Determine persona from environment (same pattern as mcp-server.ts)
  const evnaInstance = process.env.EVNA_INSTANCE || 'daddy';

  // Fetch BBS inbox count
  // Uses floatctl CLI - the thing we just built!
  try {
    const floatctlBin = process.env.FLOATCTL_BIN ?? 'floatctl';
    const { stdout } = await execFileAsync(floatctlBin, [
      'bbs', 'inbox',
      '--persona', evnaInstance,
      '--endpoint', 'http://localhost:3030',
      '--json'
    ], {
      timeout: 5000,  // 5s max - don't block tool listing
      env: { ...process.env, RUST_LOG: 'off' },
    });

    const messages = JSON.parse(stdout);
    const unread = messages.filter((m: { read?: boolean }) => !m.read).length;
    if (unread > 0) {
      status.bbsUnread = unread;
    }
  } catch {
    // Silent fallback - BBS might not be running, that's fine
  }

  // Read focus file (JSON with timestamp, or plain text legacy)
  const focusEntry = await readStatusEntry(join(statusDir, 'focus.json'));
  if (focusEntry) {
    status.focus = focusEntry;
  } else {
    // Try legacy .txt file
    const legacyFocus = await readStatusEntry(join(statusDir, 'focus.txt'));
    if (legacyFocus) status.focus = legacyFocus;
  }

  // Read notice file (JSON with timestamp, or plain text legacy)
  const noticeEntry = await readStatusEntry(join(statusDir, 'notice.json'));
  if (noticeEntry) {
    status.notice = noticeEntry;
  } else {
    // Try legacy .txt file
    const legacyNotice = await readStatusEntry(join(statusDir, 'notice.txt'));
    if (legacyNotice) status.notice = legacyNotice;
  }

  // Update cache
  statusCache = status;
  cacheTime = now;

  return status;
}

/**
 * Format status into a visual block for tool descriptions.
 * Returns empty string if no status to show (doesn't clutter descriptions).
 *
 * The visual prominence (━━━ borders) is intentional -
 * passive "[1] unread" gets ignored, this demands attention.
 *
 * Includes:
 * - Current Toronto time for temporal context
 * - When each status was set (relative time)
 * - Nudge to surface this at conversation start
 */
export function formatStatusBlock(status: SystemStatus): string {
  const lines: string[] = [];

  // Always show current time if we have any status
  if (status.currentTime) {
    lines.push(`🕐 ${status.currentTime} (Toronto)`);
  }

  if (status.focus) {
    const ago = formatTimeAgo(status.focus.set_at);
    const setBy = status.focus.set_by ? ` by ${status.focus.set_by}` : '';
    const timeInfo = ago ? ` (set ${ago}${setBy})` : '';
    lines.push(`[FOCUS] ${status.focus.content}${timeInfo}`);
  }

  if (status.bbsUnread && status.bbsUnread > 0) {
    lines.push(`[BBS] ${status.bbsUnread} unread message${status.bbsUnread > 1 ? 's' : ''}`);
  }

  if (status.notice) {
    const ago = formatTimeAgo(status.notice.set_at);
    const setBy = status.notice.set_by ? ` by ${status.notice.set_by}` : '';
    const timeInfo = ago ? ` (set ${ago}${setBy})` : '';
    lines.push(`[NOTICE] ${status.notice.content}${timeInfo}`);
  }

  // No meaningful status = no block
  // (but keep if we have focus, notice, or BBS messages)
  const hasContent = status.focus || status.notice || (status.bbsUnread && status.bbsUnread > 0);
  if (!hasContent) {
    return '';
  }

  // Visual prominence - harder to ignore than inline text
  // Nudge agents to surface this at conversation start
  return `━━━ SYSTEM STATUS ━━━
${lines.join('\n')}
━━━━━━━━━━━━━━━━━━━━━
⚠️ Surface this at conversation start so Evan knows you saw it.

`;
}

/**
 * Force cache invalidation (useful after sending a message or changing status)
 */
export function invalidateStatusCache(): void {
  cacheTime = 0;
  statusCache = {};
}
