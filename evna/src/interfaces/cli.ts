/**
 * Agent CLI Interface for EVNA
 * Conversational interface with Agent SDK orchestration
 *
 * This is the LLM-powered conversational interface. For direct tool access,
 * use src/cli.ts instead (faster, cheaper).
 *
 * Usage:
 *   bun run agent "your query here"
 *   bun run agent "help me catch up on yesterday's work" --verbose
 *   bun run agent "continue debugging" --session abc-123
 */

// Load .env with fallback chain: ./.env → ~/.floatctl/.env → existing env vars
import { loadEnvWithFallback } from "../lib/env-loader.js";
loadEnvWithFallback();
import { homedir } from "os";
import { join } from "path";
import { writeFileSync, readFileSync, existsSync, mkdirSync } from "fs";
import type { SDKUserMessage } from "@anthropic-ai/claude-agent-sdk";

// Lazy imports to avoid requiring env vars for --help
let query: any;
let evnaNextMcpServer: any;
let createQueryOptions: any;
let DEFAULT_MODEL: any = "claude-sonnet-4-5";
let DEFAULT_MAX_TURNS: any = 25;

async function lazyImports() {
  if (!query) {
    const agentSdk = await import("@anthropic-ai/claude-agent-sdk");
    query = agentSdk.query;

    const mcpModule = await import("./mcp.js");
    evnaNextMcpServer = mcpModule.evnaNextMcpServer;

    const configModule = await import("../core/config.js");
    createQueryOptions = configModule.createQueryOptions;
    DEFAULT_MODEL = configModule.DEFAULT_MODEL;
    DEFAULT_MAX_TURNS = configModule.DEFAULT_MAX_TURNS;
  }
}

// ANSI color codes
const colors = {
  reset: '\x1b[0m',
  bright: '\x1b[1m',
  dim: '\x1b[2m',
  cyan: '\x1b[36m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  gray: '\x1b[90m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
};

function bold(text: string): string {
  return `${colors.bright}${text}${colors.reset}`;
}

function cyan(text: string): string {
  return `${colors.cyan}${text}${colors.reset}`;
}

function green(text: string): string {
  return `${colors.green}${text}${colors.reset}`;
}

function yellow(text: string): string {
  return `${colors.yellow}${text}${colors.reset}`;
}

function red(text: string): string {
  return `${colors.red}${text}${colors.reset}`;
}

function gray(text: string): string {
  return `${colors.gray}${text}${colors.reset}`;
}

function blue(text: string): string {
  return `${colors.blue}${text}${colors.reset}`;
}

function magenta(text: string): string {
  return `${colors.magenta}${text}${colors.reset}`;
}

interface ParsedArgs {
  query?: string;
  session?: string;
  model?: string;
  maxTurns?: number;
  verbose?: boolean;
  quiet?: boolean;
  stream?: boolean;
  saveSession?: boolean;
  help?: boolean;
  notifyIssue?: string;
}

/**
 * Show help message
 */
function showHelp(): void {
  console.log(`
${bold('EVNA Agent CLI')} - Conversational interface with LLM orchestration

${bold('USAGE:')}
  ${cyan('bun run agent')} ${yellow('<query>')} ${gray('[options]')}

${bold('DESCRIPTION:')}
  The agent CLI uses Claude to interpret natural language queries and orchestrate
  multiple tools autonomously. This is slower and more expensive than the direct
  CLI (src/cli.ts), but better for complex multi-step tasks.

${bold('OPTIONS:')}
  --session <id>         Resume previous conversation session
  --model <name>         Claude model to use (default: ${DEFAULT_MODEL})
  --max-turns <n>        Maximum agent turns (default: ${DEFAULT_MAX_TURNS})
  --verbose              Show detailed agent reasoning and tool calls
  --quiet                Minimal output (only final response)
  --stream               Stream responses as they arrive (default: true)
  --no-stream            Disable streaming, wait for complete response
  --save-session         Save session for later resume
  --notify-issue <repo>  Notify GitHub issue when complete
  --help                 Show this help message

${bold('EXAMPLES:')}

  ${gray('# Simple conversational query')}
  ${cyan('bun run agent')} ${yellow('"help me catch up on yesterday\'s work"')}

  ${gray('# Resume previous session')}
  ${cyan('bun run agent')} ${yellow('"continue debugging"')} --session abc-123

  ${gray('# Verbose mode to see agent reasoning')}
  ${cyan('bun run agent')} ${yellow('"analyze recent performance issues"')} --verbose

  ${gray('# Save session for multi-turn conversation')}
  ${cyan('bun run agent')} ${yellow('"start investigating the auth bug"')} --save-session

  ${gray('# Use different model')}
  ${cyan('bun run agent')} ${yellow('"quick question about X"')} --model claude-sonnet-4-5

${bold('COMPARISON WITH DIRECT CLI:')}

  ${bold('Agent CLI')} (this):
    - Natural language orchestration
    - Multi-step autonomous tasks
    - ~3-5s response time
    - ~3-5k tokens per query
    - Use when: Complex tasks, exploratory queries

  ${bold('Direct CLI')} (src/cli.ts):
    - Direct tool invocation
    - Single-step operations
    - ~1s response time
    - <1k tokens per query
    - Use when: Known tool, quick lookups

  ${gray('Example:')}
    ${cyan('bun run agent')} ${yellow('"help me understand recent changes"')}  ${gray('# Agent orchestrates tools')}
    ${cyan('evna boot')} ${yellow('"recent changes"')}                       ${gray('# Direct brain_boot call')}

${bold('SESSION MANAGEMENT:')}

  Sessions are stored in ~/.evna/sessions/ and can be resumed later for
  multi-turn conversations. Use --save-session to enable.

  ${cyan('bun run agent')} ${yellow('"start task X"')} --save-session
  ${gray('# Returns session ID: abc-123')}
  ${cyan('bun run agent')} ${yellow('"continue with Y"')} --session abc-123

${gray('For more information: https://github.com/yourusername/evna')}
`);
}

/**
 * Parse CLI arguments with support for multiple options
 */
function parseArgs(): ParsedArgs {
  const args = process.argv.slice(2);

  if (args.length === 0 || args.includes('--help') || args.includes('-h')) {
    return { help: true };
  }

  const result: ParsedArgs = {
    stream: true, // Default to streaming
  };

  let i = 0;
  while (i < args.length) {
    const arg = args[i];

    if (arg.startsWith('--')) {
      const key = arg.slice(2);

      switch (key) {
        case 'session':
          result.session = args[++i];
          break;
        case 'model':
          result.model = args[++i];
          break;
        case 'max-turns':
          result.maxTurns = parseInt(args[++i]);
          break;
        case 'notify-issue':
          result.notifyIssue = args[++i];
          break;
        case 'verbose':
          result.verbose = true;
          break;
        case 'quiet':
          result.quiet = true;
          break;
        case 'stream':
          result.stream = true;
          break;
        case 'no-stream':
          result.stream = false;
          break;
        case 'save-session':
          result.saveSession = true;
          break;
        case 'help':
        case 'h':
          result.help = true;
          break;
        default:
          console.error(red(`Unknown option: ${arg}`));
          process.exit(1);
      }
    } else {
      // First non-option argument is the query
      if (!result.query) {
        result.query = arg;
      }
    }

    i++;
  }

  return result;
}

/**
 * Session management
 */
const SESSION_DIR = join(homedir(), '.evna', 'sessions');

interface SavedSession {
  id: string;
  messages: any[];
  created: string;
  updated: string;
}

function ensureSessionDir(): void {
  if (!existsSync(SESSION_DIR)) {
    mkdirSync(SESSION_DIR, { recursive: true });
  }
}

function saveSession(sessionId: string, messages: any[]): void {
  ensureSessionDir();
  const sessionFile = join(SESSION_DIR, `${sessionId}.json`);
  const session: SavedSession = {
    id: sessionId,
    messages,
    created: existsSync(sessionFile)
      ? JSON.parse(readFileSync(sessionFile, 'utf-8')).created
      : new Date().toISOString(),
    updated: new Date().toISOString(),
  };
  writeFileSync(sessionFile, JSON.stringify(session, null, 2));
}

function loadSession(sessionId: string): any[] | null {
  ensureSessionDir();
  const sessionFile = join(SESSION_DIR, `${sessionId}.json`);
  if (!existsSync(sessionFile)) {
    return null;
  }
  const session: SavedSession = JSON.parse(readFileSync(sessionFile, 'utf-8'));
  return session.messages;
}

function generateSessionId(): string {
  return `agent-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

/**
 * Main CLI runner
 * Processes queries with Agent SDK orchestration
 */
async function main() {
  const args = parseArgs();

  // Show help if requested or no query
  if (args.help || !args.query) {
    showHelp();
    process.exit(args.help ? 0 : 1);
  }

  const userQuery = args.query!;
  const sessionId = args.session || (args.saveSession ? generateSessionId() : undefined);

  // Header (unless quiet mode)
  if (!args.quiet) {
    console.error(bold('🧠 EVNA Agent CLI'));
    console.error(gray('Conversational interface with LLM orchestration'));
    console.error('');
    console.error(`${gray('Query:')} ${userQuery}`);
    if (sessionId) {
      console.error(`${gray('Session:')} ${cyan(sessionId)}`);
    }
    if (args.model && args.model !== DEFAULT_MODEL) {
      console.error(`${gray('Model:')} ${args.model}`);
    }
    if (args.notifyIssue) {
      console.error(`${gray('Notify Issue:')} ${args.notifyIssue}`);
    }
    console.error('');
  }

  // Load previous session messages if resuming
  let previousMessages: any[] = [];
  if (args.session) {
    const loaded = loadSession(args.session);
    if (loaded) {
      previousMessages = loaded;
      if (!args.quiet) {
        console.error(green(`✓ Loaded ${previousMessages.length} previous messages`));
        console.error('');
      }
    } else {
      console.error(yellow(`⚠ Session ${args.session} not found, starting new conversation`));
      console.error('');
    }
  }

  // Generate messages from CLI input
  async function* generateMessages(): AsyncGenerator<SDKUserMessage> {
    // Yield previous messages if resuming
    for (const msg of previousMessages) {
      yield msg;
    }

    // Yield new user message
    yield {
      type: "user" as const,
      session_id: sessionId || "",
      message: {
        role: "user" as const,
        content: userQuery,
      },
      parent_tool_use_id: null,
    };
  }

  try {
    // Lazy load dependencies (requires env vars)
    await lazyImports();

    // Configure Agent SDK options with Skills and hooks enabled
    const options = createQueryOptions(evnaNextMcpServer) as any;

    // Apply user options
    if (args.model) {
      options.model = args.model;
    }
    if (args.maxTurns) {
      options.maxTurns = args.maxTurns;
    }

    // Enable Skills and filesystem settings
    options.settingSources = ["user", "project"];
    options.allowedTools = [
      ...(options.allowedTools || []),
      "Skill",  // Enable Agent Skills
      "TodoWrite",  // Enable todo tracking
      "SlashCommand"  // Enable slash commands
    ];

    // Set working directory to ~/.evna (enables global skills, slash commands, hooks)
    // Agent SDK will look for skills at ~/.evna/skills/, commands at ~/.evna/commands/, etc.
    options.cwd = join(homedir(), '.evna');

    // Run agent query
    const result = await query({
      prompt: generateMessages(),
      options,
    });

    const allMessages: any[] = [];
    let lastContent = '';

    // Stream responses
    for await (const message of result) {
      allMessages.push(message);

      // Extract text content
      let content = '';
      if (typeof message === 'string') {
        content = message;
      } else if (message.message?.content) {
        if (typeof message.message.content === 'string') {
          content = message.message.content;
        } else if (Array.isArray(message.message.content)) {
          content = message.message.content
            .filter((block: any) => block.type === 'text')
            .map((block: any) => block.text)
            .join('\n');
        }
      }

      // Streaming output
      if (args.stream && content && content !== lastContent) {
        // Only show new content (incremental)
        const newContent = content.slice(lastContent.length);
        if (newContent) {
          process.stdout.write(newContent);
        }
        lastContent = content;
      }

      // Verbose mode: show tool calls
      if (args.verbose && message.message?.content) {
        const toolUses = Array.isArray(message.message.content)
          ? message.message.content.filter((block: any) => block.type === 'tool_use')
          : [];

        for (const toolUse of toolUses) {
          console.error('');
          console.error(magenta(`🔧 Tool: ${toolUse.name}`));
          console.error(gray(JSON.stringify(toolUse.input, null, 2)));
        }
      }
    }

    // Non-streaming: output final response
    if (!args.stream && lastContent) {
      console.log(lastContent);
    }

    // Save session if requested
    if (sessionId && args.saveSession) {
      saveSession(sessionId, allMessages);
      if (!args.quiet) {
        console.error('');
        console.error(green(`✓ Session saved: ${sessionId}`));
        console.error(gray(`  Resume with: ${cyan(`bun run agent "next query" --session ${sessionId}`)}`));
      }
    }

    if (!args.quiet) {
      console.error('');
      console.error(green('✅ Query completed'));
    }
  } catch (error) {
    console.error('');
    console.error(red('❌ Error running agent:'), error);
    process.exit(1);
  }
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export { main };
