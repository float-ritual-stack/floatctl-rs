/**
 * EVNA-Next MCP Server
 * Standalone MCP server for Claude Desktop integration
 * Exposes both tools (brain_boot, semantic_search, active_context) and resources (daily notes, etc.)
 */

// Load .env with fallback chain: ./.env → ~/.floatctl/.env → existing env vars
import { loadEnvWithFallback } from "./lib/env-loader.js";
loadEnvWithFallback();
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { readFile, readdir } from "fs/promises";
import { join } from "path";
import { homedir } from "os";
import { execFile } from "child_process";
import { promisify } from "util";

const execFileAsync = promisify(execFile);
// Import tool instances and business logic from shared module
import { brainBoot, search, activeContext, r2Sync, askEvna, github } from "./tools/index.js";
import { AskEvnaAgent } from "./tools/ask-evna-agent.js";
import { toMcpTools } from "./tools/registry-zod.js";
import { updateSystemPrompt, readSystemPrompt } from "./tools/update-system-prompt.js";
import { startBridgeSyncTrigger } from "./lib/bridge-sync-trigger.js";
import { loadFloatConfig } from "./lib/floatctl-config.js";
// NOTE: System status injection removed Dec 2025 - see ListToolsRequestSchema comment
// import { getSystemStatus, formatStatusBlock } from "./lib/system-status.js";

// Detect instance type from environment variable
// Maps EVNA_INSTANCE to client_type for active context tagging
const INSTANCE_MAP: Record<string, 'desktop' | 'claude_code'> = {
  daddy: 'desktop',      // Claude Desktop
  kitty: 'claude_code',  // Float Hub / Claude Code
  cowboy: 'claude_code', // Other console sessions
};

const evnaInstance = process.env.EVNA_INSTANCE;
const detectedClientType = evnaInstance ? INSTANCE_MAP[evnaInstance] : undefined;

// Log instance detection on startup (stderr safe for MCP)
if (evnaInstance) {
  console.error(`[evna] Instance detected: ${evnaInstance} → client_type: ${detectedClientType || 'unknown'}`);
}

// Create MCP server
const server = new Server(
  {
    name: "evna-next",
    version: "1.0.0",
  },
  {
    capabilities: {
      tools: {},
      resources: {},
    },
  }
);

// Register tools handler - auto-wired from Zod schemas (converted to JSON)
// NOTE: Dynamic status injection removed Dec 2025 - was causing Desktop to request
// approval repeatedly (tool description changing = looks like "new" tool to Claude)
// Status now flows via active_context capture responses, not tool descriptions
server.setRequestHandler(ListToolsRequestSchema, async () => {
  const tools = toMcpTools();
  return { tools };
});

// Register call tool handler
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args = {} } = request.params;

  // Note: No console logging during tool execution - MCP uses stderr for JSON-RPC

  try {
    if (name === "brain_boot") {
      const result = await brainBoot.boot({
        query: args.query as string,
        project: args.project as string | undefined,
        lookbackDays: (args.lookbackDays as number | undefined) ?? 7,
        maxResults: (args.maxResults as number | undefined) ?? 10,
        githubUsername: args.githubUsername as string | undefined,
      });

      return {
        content: [
          {
            type: "text",
            text: result.summary,
          },
        ],
      };
    } else if (name === "semantic_search") {
      const results = await search.search({
        query: args.query as string,
        limit: (args.limit as number | undefined) ?? 10,
        project: args.project as string | undefined,
        since: args.since as string | undefined,
        threshold: (args.threshold as number | undefined) ?? 0.5,
      });

      const formatted = search.formatResults(results);

      return {
        content: [
          {
            type: "text",
            text: formatted,
          },
        ],
      };
    } else if (name === "active_context") {
      const captureParam = args.capture as string | undefined;

      const result = await activeContext.query({
        query: args.query as string | undefined,
        capture: captureParam,
        limit: (args.limit as number | undefined) ?? 10,
        project: args.project as string | undefined,
        // Use explicit arg > detected instance > heuristic fallback
        client_type: (args.client_type as 'desktop' | 'claude_code' | undefined) ?? detectedClientType,
        include_cross_client: (args.include_cross_client as boolean | undefined) ?? true,
        synthesize: (args.synthesize as boolean | undefined) ?? true,
      });

      // Pipe ctx:: markers to floatctl (if capture contains ctx::)
      if (captureParam && captureParam.includes('ctx::')) {
        try {
          const { spawn } = await import('child_process');
          const proc = spawn('floatctl', ['ctx']);
          proc.stdin.write(captureParam);
          proc.stdin.end();
        } catch (error) {
          // Silent fail
        }
      }

      return {
        content: [
          {
            type: "text",
            text: result,
          },
        ],
      };
    } else if (name === "r2_sync") {
      const operation = args.operation as string;
      let result: string;

      switch (operation) {
        case "status":
          result = await r2Sync.status({
            daemon_type: args.daemon_type as 'daily' | 'dispatch' | 'all' | undefined,
          });
          break;
        case "trigger":
          result = await r2Sync.trigger({
            daemon_type: args.daemon_type as 'daily' | 'dispatch' | 'all' | undefined,
            wait: args.wait as boolean | undefined,
          });
          break;
        case "start":
          result = await r2Sync.start({
            daemon_type: args.daemon_type as 'daily' | 'dispatch' | 'all' | undefined,
          });
          break;
        case "stop":
          result = await r2Sync.stop({
            daemon_type: args.daemon_type as 'daily' | 'dispatch' | 'all' | undefined,
          });
          break;
        case "logs":
          result = await r2Sync.logs({
            daemon_type: args.daemon_type as 'daily' | 'dispatch',
            lines: args.lines as number | undefined,
          });
          break;
        default:
          throw new Error(`Unknown operation: ${operation}`);
      }

      return {
        content: [
          {
            type: "text",
            text: result,
          },
        ],
      };
    } else if (name === "ask_evna") {
      // Default to 120 second timeout for MCP calls (complex queries with AutoRAG need time)
      const timeout_ms = (args.timeout_ms as number | undefined) ?? 120000;
      const query = args.query as string;

      const result = await askEvna.ask({
        query,
        session_id: args.session_id as string | undefined,
        fork_session: args.fork_session as boolean | undefined,
        timeout_ms,
        include_projects_context: args.include_projects_context as boolean | undefined,
        all_projects: args.all_projects as boolean | undefined,
      });

      // Pipe ctx:: markers to floatctl (if query contains ctx::)
      if (query.includes('ctx::')) {
        try {
          const { spawn } = await import('child_process');
          const proc = spawn('floatctl', ['ctx']);
          proc.stdin.write(query);
          proc.stdin.end();
        } catch (error) {
          // Silent fail
        }
      }

      return {
        content: [
          {
            type: "text",
            text: AskEvnaAgent.formatMcpResponse(result),
          },
        ],
        // Add metadata flag if timed out (clients can detect this)
        ...(result.timed_out ? { _meta: { timed_out: true } } : {}),
      };
    } else if (name === "peek_session") {
      // Read-only peek at session progress using floatctl
      const session_id = args.session_id as string;
      const message_count = (args.message_count as number | undefined) ?? 5;
      const include_tools = (args.include_tools as boolean | undefined) ?? false;

      try {
        const floatctlBin = process.env.FLOATCTL_BIN ?? 'floatctl';
        const floatctlArgs = [
          'claude', 'show', session_id,
          '--last', message_count.toString(),
          '--format', 'text'
        ];

        if (!include_tools) {
          floatctlArgs.push('--no-tools');
        }

        const { stdout } = await execFileAsync(floatctlBin, floatctlArgs, {
          timeout: 10000, // 10s max
          maxBuffer: 2 * 1024 * 1024, // 2MB max
          env: { ...process.env, RUST_LOG: 'off' },
        });

        // Extract just the message content (filter headers/formatting)
        const lines = stdout.split('\n');
        const contentLines = lines.filter(l =>
          !l.includes('Session:') &&
          !l.includes('Project:') &&
          !l.includes('Branch:') &&
          !l.includes('Started:') &&
          !l.includes('Ended:') &&
          !l.includes('Summary') &&
          !l.includes('Tokens:') &&
          !l.includes('Tool calls:') &&
          !l.includes('Cache efficiency:') &&
          !l.includes('╭─') &&
          !l.includes('╰─') &&
          !l.includes('┌─') &&
          !l.includes('└─') &&
          l.trim().length > 0
        );

        const content = contentLines.join('\n').trim();

        return {
          content: [
            {
              type: "text",
              text: content || "(Session exists but no messages found matching criteria)",
            },
          ],
        };
      } catch (error) {
        return {
          content: [
            {
              type: "text",
              text: `Unable to peek at session ${session_id}: ${error instanceof Error ? error.message : String(error)}\n\nSession may not exist or floatctl may not be available.`,
            },
          ],
        };
      }
    } else if (name === "update_system_prompt") {
      const result = await updateSystemPrompt({
        content: args.content as string,
        backup: (args.backup as boolean | undefined) ?? true,
      });

      return {
        content: [
          {
            type: "text",
            text: result,
          },
        ],
      };
    } else if (name === "read_system_prompt") {
      const result = await readSystemPrompt();

      return {
        content: [
          {
            type: "text",
            text: result,
          },
        ],
      };
    } else {
      throw new Error(`Unknown tool: ${name}`);
    }
  } catch (error) {
    // Note: No console logging here - return error via MCP protocol
    return {
      content: [
        {
          type: "text",
          text: `Error: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
      isError: true,
    };
  }
});

// Register resources handler
server.setRequestHandler(ListResourcesRequestSchema, async () => {
  return {
    resources: [
      {
        uri: "daily://today",
        name: "Today's Daily Note",
        description: "Returns today's daily note (YYYY-MM-DD.md) from configured daily_notes path",
        mimeType: "text/markdown",
      },
      {
        uri: "daily://recent",
        name: "Recent Daily Notes (Last 3 Days)",
        description: "Returns last 3 days of daily notes concatenated with date headers",
        mimeType: "text/markdown",
      },
      {
        uri: "daily://week",
        name: "This Week's Daily Notes (Last 7 Days)",
        description: "Returns last 7 days of daily notes concatenated with date headers",
        mimeType: "text/markdown",
      },
      {
        uri: "daily://list",
        name: "Available Daily Notes",
        description: "Returns JSON list of available daily notes (last 30 days)",
        mimeType: "application/json",
      },
      // TODO: Future resources:
      // - notes://{path} - template for any note (e.g., notes://bridges/restoration.md)
      // - tldr://recent - TLDR summaries
      // - bridges://recent - recent bridge documents
    ],
  };
});

// Register read resource handler
server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
  const uri = request.params.uri;

  // Load floatctl config to get correct daily notes path
  let dailyDir: string;
  try {
    const config = loadFloatConfig();
    dailyDir = config.paths.daily_notes;
  } catch (error) {
    // Fallback to legacy path if config not available
    dailyDir = join(homedir(), '.evans-notes', 'daily');
  }

  try {
    // Static: daily://today
    if (uri === "daily://today") {
      const today = new Date().toISOString().split('T')[0]; // YYYY-MM-DD
      const notePath = join(dailyDir, `${today}.md`);
      const content = await readFile(notePath, 'utf-8');

      return {
        contents: [
          {
            uri,
            mimeType: "text/markdown",
            text: content,
          },
        ],
      };
    }

    // Static: daily://recent (last 3 days concatenated)
    if (uri === "daily://recent") {
      const recentDates: string[] = [];
      for (let i = 0; i < 3; i++) {
        const d = new Date();
        d.setDate(d.getDate() - i);
        recentDates.push(d.toISOString().split('T')[0]);
      }

      const sections = await Promise.all(
        recentDates.map(async (date) => {
          const notePath = join(dailyDir, `${date}.md`);
          try {
            const content = await readFile(notePath, 'utf-8');
            return `# ${date}\n\n${content}`;
          } catch (err) {
            return `# ${date}\n\n*(No note found)*`;
          }
        })
      );

      const combined = sections.join('\n\n---\n\n');

      return {
        contents: [
          {
            uri,
            mimeType: "text/markdown",
            text: combined,
          },
        ],
      };
    }

    // Static: daily://week (last 7 days concatenated)
    if (uri === "daily://week") {
      const weekDates: string[] = [];
      for (let i = 0; i < 7; i++) {
        const d = new Date();
        d.setDate(d.getDate() - i);
        weekDates.push(d.toISOString().split('T')[0]);
      }

      const sections = await Promise.all(
        weekDates.map(async (date) => {
          const notePath = join(dailyDir, `${date}.md`);
          try {
            const content = await readFile(notePath, 'utf-8');
            return `# ${date}\n\n${content}`;
          } catch (err) {
            return `# ${date}\n\n*(No note found)*`;
          }
        })
      );

      const combined = sections.join('\n\n---\n\n');

      return {
        contents: [
          {
            uri,
            mimeType: "text/markdown",
            text: combined,
          },
        ],
      };
    }

    // Static: daily://list (JSON of last 30 days)
    if (uri === "daily://list") {
      const files = await readdir(dailyDir);
      const noteFiles = files
        .filter((f) => /^\d{4}-\d{2}-\d{2}\.md$/.test(f))
        .map((f) => f.replace('.md', ''))
        .sort()
        .reverse()
        .slice(0, 30);

      return {
        contents: [
          {
            uri,
            mimeType: "application/json",
            text: JSON.stringify({ notes: noteFiles }, null, 2),
          },
        ],
      };
    }

    throw new Error(`Unknown resource URI: ${uri}`);
  } catch (error) {
    throw new Error(`Failed to read resource ${uri}: ${error instanceof Error ? error.message : String(error)}`);
  }
});

// Start the server
async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("🧠 EVNA-Next MCP Server ready (tools + resources)");
  
  // Start bridge sync trigger (watches for file changes, triggers R2 sync)
  // Debounces writes (5s) to batch rapid changes, then syncs to make AutoRAG current
  startBridgeSyncTrigger({
    enabled: process.env.EVNA_AUTO_SYNC !== "false",  // Opt-out via env
    debounce_ms: 5000,  // 5 second debounce (batch rapid writes)
  });
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
