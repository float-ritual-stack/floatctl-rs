/**
 * Master Stream Context Injection
 *
 * Fetches recent entries from float-box:/opt/float/logs/master_stream.jsonl
 * to give EVNA visibility into cross-machine activity stream
 */

import { execFile } from "child_process";
import { promisify } from "util";
import { debug } from "./logger.js";

const execFileAsync = promisify(execFile);

/**
 * Escape shell argument for safe use in SSH commands
 * Uses single quotes and escapes any embedded single quotes
 */
function escapeShellArg(arg: string): string {
  // Replace single quotes with '\'' (end quote, escaped quote, start quote)
  return `'${arg.replace(/'/g, "'\\''")}'`;
}

/**
 * Validate remote path to prevent command injection
 * Only allows safe path characters
 */
function validateRemotePath(path: string): boolean {
  // Allow alphanumeric, /, ., -, _, and ~
  return /^[a-zA-Z0-9/.\_~-]+$/.test(path);
}

export interface MasterStreamEntry {
  timestamp: string;
  source?: string;     // open-webui, etc
  machine?: string;    // from ctx queue entries
  user?: string;
  role?: string;
  message: string;
  type?: string;
}

export interface MasterStreamContextOptions {
  remoteHost?: string;   // SSH host (default: float-box)
  remotePath?: string;   // Remote log path
  tailLines?: number;    // How many lines to fetch (default: 50)
  timeout?: number;      // SSH timeout in ms (default: 5000)
}

/**
 * Fetch recent entries from master_stream.jsonl via SSH
 */
export async function getMasterStreamContext(
  options: MasterStreamContextOptions = {}
): Promise<MasterStreamEntry[]> {
  const {
    remoteHost = process.env.FLOATCTL_CTX_REMOTE_HOST || "float-box",
    remotePath = process.env.FLOATCTL_CTX_REMOTE_PATH || "/opt/float/logs/master_stream.jsonl",
    tailLines = 50,
    timeout = 5000,
  } = options;

  debug("master-stream-context", `Fetching last ${tailLines} lines from ${remoteHost}:${remotePath}`, { timeout });

  // Validate remote path to prevent command injection
  if (!validateRemotePath(remotePath)) {
    debug("master-stream-context", `Invalid remote path rejected: ${remotePath}`);
    return [];
  }

  try {
    // Use escapeShellArg to safely pass the path to tail command
    const { stdout } = await execFileAsync(
      "ssh",
      [remoteHost, `tail -n ${tailLines} ${escapeShellArg(remotePath)}`],
      {
        timeout,
        maxBuffer: 1024 * 1024, // 1MB max
      }
    );

    if (!stdout || !stdout.trim()) {
      debug("master-stream-context", "No output from remote");
      return [];
    }

    const entries: MasterStreamEntry[] = [];
    const lines = stdout.split("\n").filter(l => l.trim());

    debug("master-stream-context", `Received ${lines.length} lines from remote`);

    for (const line of lines) {
      try {
        const parsed = JSON.parse(line);

        // Extract message field (handle both formats)
        const message = parsed.message || parsed.content || "";
        if (!message) continue;

        entries.push({
          timestamp: parsed.timestamp,
          source: parsed.source,
          machine: parsed.machine,
          user: parsed.user,
          role: parsed.role,
          message: typeof message === "string" ? message : JSON.stringify(message),
          type: parsed.type,
        });
      } catch (error) {
        debug("master-stream-context", "Failed to parse line", { error, line: line.substring(0, 100) });
      }
    }

    debug("master-stream-context", `Parsed ${entries.length} valid entries`);
    return entries;
  } catch (error: any) {
    // SSH failures are expected when float-box is unreachable
    if (error.code === 'ETIMEDOUT' || error.killed) {
      debug("master-stream-context", `SSH timeout to ${remoteHost}`, { timeout });
    } else {
      debug("master-stream-context", `SSH error to ${remoteHost}`, { error: error.message });
    }
    return [];
  }
}

/**
 * Format master stream entries as markdown for system prompt injection
 */
export function formatMasterStreamForPrompt(entries: MasterStreamEntry[]): string {
  if (entries.length === 0) {
    return "";
  }

  debug("master-stream-context", `Formatting ${entries.length} entries for prompt injection`);

  // Group entries and extract key information with semantic structure
  const formatted = entries.map(entry => {
    const source = entry.source || entry.machine || "unknown";
    const timestamp = entry.timestamp;
    const role = entry.role || "";
    const type = entry.type || "";

    // Extract ctx:: annotations if present
    const ctxMatch = entry.message.match(/ctx::([^\n]+)/);
    const ctxAnnotation = ctxMatch ? ctxMatch[1].trim() : null;

    // Extract project:: annotations
    const projectMatch = entry.message.match(/project::([^\s\]]+)/);
    const projectAnnotation = projectMatch ? projectMatch[1].trim() : null;

    // Extract mode:: annotations
    const modeMatch = entry.message.match(/mode::([^\s\]]+)/);
    const modeAnnotation = modeMatch ? modeMatch[1].trim() : null;

    // Truncate long messages but preserve structure
    let message = entry.message.length > 600
      ? entry.message.substring(0, 600) + "\n  ...(truncated)"
      : entry.message;

    // Clean up message indentation
    message = message.split("\n").map(l => l.trim()).join("\n  ");

    // Build structured entry
    let structuredEntry = `### ${timestamp}`;

    const metadata: string[] = [];
    if (source) metadata.push(`source: ${source}`);
    if (role) metadata.push(`role: ${role}`);
    if (type) metadata.push(`type: ${type}`);
    if (projectAnnotation) metadata.push(`project: ${projectAnnotation}`);
    if (modeAnnotation) metadata.push(`mode: ${modeAnnotation}`);

    if (metadata.length > 0) {
      structuredEntry += `\n**Meta**: ${metadata.join(" | ")}`;
    }

    if (ctxAnnotation) {
      structuredEntry += `\n**Context**: ${ctxAnnotation}`;
    }

    structuredEntry += `\n\n  ${message}`;

    return structuredEntry;
  }).join("\n\n---\n\n");

  return `
---

## Recent Activity Stream (master_stream.jsonl)

<!--
WHAT THIS IS: Cross-machine activity stream from float-box
SOURCES: ctx:: captures (from floatctl ctx), open-webui conversations, system events
FORMAT: Chronological entries with extracted metadata (project::, mode::, ctx:: annotations)
HOW TO USE: Check for recent work context, system state, cross-machine coordination
-->

The following shows the last ${entries.length} entries from your master activity stream.
This provides ambient awareness of work happening across machines and interfaces.

${formatted}

---
`;
}

/**
 * Get formatted master stream context for injection
 */
export async function getMasterStreamContextInjection(
  options: MasterStreamContextOptions = {}
): Promise<string> {
  const entries = await getMasterStreamContext({
    tailLines: 15,  // Last 15 entries (~2100 tokens, balances recency with token cost)
    ...options,
  });

  return formatMasterStreamForPrompt(entries);
}
