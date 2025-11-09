/**
 * floatctl-claude integration
 * Shells out to floatctl claude commands for Claude Code session log querying
 */

import { exec } from "child_process";
import { promisify } from "util";

const execAsync = promisify(exec);

export interface ClaudeSession {
  timestamp: string;
  project: string;
  branch?: string;
  display: string;
}

export interface ClaudeMessage {
  role: string;
  content: string;
  truncated: boolean;
}

export interface ClaudeSessionContext {
  project: string;
  branch?: string;
  first_messages: ClaudeMessage[];
  last_messages: ClaudeMessage[];
  stats: {
    turn_count: number;
    tool_calls: number;
  };
}

export class FloatctlClaudeTool {
  /**
   * List recent Claude Code sessions
   */
  async listRecentSessions(options: {
    n?: number;
    project?: string;
  }): Promise<string> {
    const { n = 10, project } = options;

    const args = [
      "claude",
      "list-sessions",
      "--format",
      "json",
      "--limit",
      String(n),
    ];

    if (project) {
      args.push("--project", project);
    }

    try {
      const { stdout } = await execAsync(`floatctl ${args.join(" ")}`);
      const sessions = JSON.parse(stdout) as ClaudeSession[];

      // Format for display
      if (sessions.length === 0) {
        return "No Claude Code sessions found.";
      }

      const formatted = sessions
        .map((s, idx) => {
          const timestamp = new Date(s.timestamp).toLocaleString();
          const branchInfo = s.branch ? ` (${s.branch})` : "";
          return `${idx + 1}. **${timestamp}**\n   Project: ${s.project}${branchInfo}\n   ${s.display || "(No title)"}`;
        })
        .join("\n\n");

      return `# Recent Claude Code Sessions (${sessions.length})\n\n${formatted}`;
    } catch (error) {
      if (error instanceof Error) {
        // Check if it's a floatctl not found error
        if (error.message.includes("command not found") || error.message.includes("ENOENT")) {
          return "Error: floatctl command not found. Please ensure floatctl is installed and in your PATH.";
        }
        // Check if history.jsonl doesn't exist
        if (error.message.includes("No such file or directory")) {
          return "Error: Claude Code history file not found at ~/.claude/history.jsonl";
        }
        return `Error listing sessions: ${error.message}`;
      }
      return `Error listing sessions: ${String(error)}`;
    }
  }

  /**
   * Read recent Claude Code context (first/last messages)
   */
  async readRecentContext(options: {
    sessions?: number;
    first?: number;
    last?: number;
    truncate?: number;
    project?: string;
  }): Promise<string> {
    const {
      sessions = 3,
      first = 3,
      last = 3,
      truncate = 400,
      project,
    } = options;

    const args = [
      "claude",
      "recent-context",
      "--format",
      "json",
      "--sessions",
      String(sessions),
      "--first",
      String(first),
      "--last",
      String(last),
      "--truncate",
      String(truncate),
    ];

    if (project) {
      args.push("--project", project);
    }

    try {
      const { stdout } = await execAsync(`floatctl ${args.join(" ")}`);
      const data = JSON.parse(stdout) as {
        sessions: ClaudeSessionContext[];
      };

      if (data.sessions.length === 0) {
        return "No Claude Code sessions found.";
      }

      // Format for display
      const formatted = data.sessions
        .map((s) => {
          const branchInfo = s.branch ? ` (${s.branch})` : "";
          const firstMsgs = s.first_messages
            .map(
              (m) =>
                `  [${m.role}]: ${m.content}${m.truncated ? "..." : ""}`
            )
            .join("\n");

          const lastMsgs = s.last_messages
            .map(
              (m) =>
                `  [${m.role}]: ${m.content}${m.truncated ? "..." : ""}`
            )
            .join("\n");

          return (
            `## Session: ${s.project}${branchInfo}\n\n` +
            `**First messages:**\n${firstMsgs}\n\n` +
            `**Last messages:**\n${lastMsgs}\n\n` +
            `**Stats:** ${s.stats.turn_count} turns, ${s.stats.tool_calls} tool calls`
          );
        })
        .join("\n\n---\n\n");

      return `# Recent Claude Code Context (${data.sessions.length} sessions)\n\n${formatted}`;
    } catch (error) {
      if (error instanceof Error) {
        // Check if it's a floatctl not found error
        if (error.message.includes("command not found") || error.message.includes("ENOENT")) {
          return "Error: floatctl command not found. Please ensure floatctl is installed and in your PATH.";
        }
        // Check if history.jsonl doesn't exist
        if (error.message.includes("No such file or directory")) {
          return "Error: Claude Code history file not found at ~/.claude/history.jsonl";
        }
        return `Error reading context: ${error.message}`;
      }
      return `Error reading context: ${String(error)}`;
    }
  }
}
