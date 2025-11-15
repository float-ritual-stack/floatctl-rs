#!/usr/bin/env bun
/**
 * Enhanced CLI for EVNA
 * Direct access to tools without Agent SDK overhead
 *
 * Usage:
 *   evna boot "what was I working on yesterday?"
 *   evna search "performance optimization" --project floatctl
 *   evna active "capture this note"
 *   evna ask "help me debug" --session abc-123
 *   evna sync status
 *   evna sessions list
 */

import "dotenv/config";

// Lazy-load tools to avoid requiring env vars for simple commands like help
let toolsLoaded = false;
let brainBoot: any;
let search: any;
let activeContext: any;
let askEvna: any;
let r2Sync: any;
let floatctlClaude: any;

async function loadTools() {
  if (!toolsLoaded) {
    const tools = await import("./tools/index.js");
    brainBoot = tools.brainBoot;
    search = tools.search;
    activeContext = tools.activeContext;
    askEvna = tools.askEvna;
    r2Sync = tools.r2Sync;
    floatctlClaude = tools.floatctlClaude;
    toolsLoaded = true;
  }
}

// ANSI color codes for better output
const colors = {
  reset: '\x1b[0m',
  bright: '\x1b[1m',
  dim: '\x1b[2m',
  cyan: '\x1b[36m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  gray: '\x1b[90m',
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

/**
 * Parse command line arguments into command and options
 */
function parseArgs(): { command: string; args: string[]; options: Record<string, any> } {
  const argv = process.argv.slice(2);

  if (argv.length === 0) {
    return { command: 'help', args: [], options: {} };
  }

  const command = argv[0];
  const rest = argv.slice(1);
  const args: string[] = [];
  const options: Record<string, any> = {};

  for (let i = 0; i < rest.length; i++) {
    const arg = rest[i];

    if (arg.startsWith('--')) {
      const key = arg.slice(2);
      const nextArg = rest[i + 1];

      // Check if next arg is a value or another flag
      if (nextArg && !nextArg.startsWith('--')) {
        options[key] = nextArg;
        i++; // Skip next arg since we consumed it
      } else {
        options[key] = true; // Boolean flag
      }
    } else {
      args.push(arg);
    }
  }

  return { command, args, options };
}

/**
 * Show help message
 */
function showHelp(): void {
  console.log(`
${bold('EVNA CLI')} - Direct access to context synthesis and search tools

${bold('USAGE:')}
  ${cyan('evna')} ${yellow('<command>')} ${gray('[arguments] [options]')}

${bold('COMMANDS:')}

  ${bold('Context & Search:')}
    ${cyan('boot')} ${yellow('<query>')}              Morning brain boot - semantic + active context + GitHub
                                  ${gray('Options: --project, --days, --limit, --github')}

    ${cyan('search')} ${yellow('<query>')}            Deep semantic search across history
                                  ${gray('Options: --project, --limit, --threshold, --since')}

    ${cyan('active')} ${yellow('[query]')}            Query recent activity stream
                                  ${gray('Options: --capture, --project, --limit')}

    ${cyan('ask')} ${yellow('<query>')}               Orchestrated multi-tool search with LLM
                                  ${gray('Options: --session, --fork, --timeout')}

  ${bold('Sessions & History:')}
    ${cyan('sessions list')}               List recent Claude Code sessions
                                  ${gray('Options: --n, --project')}

    ${cyan('sessions read')} ${yellow('<id>')}        Read session context
                                  ${gray('Options: --first, --last, --truncate')}

  ${bold('Sync & Operations:')}
    ${cyan('sync status')}                 Check R2 sync daemon status
                                  ${gray('Options: --daemon (daily|dispatch)')}

    ${cyan('sync trigger')}                Trigger immediate sync
                                  ${gray('Options: --daemon, --wait')}

    ${cyan('sync start|stop')}             Start/stop sync daemon
                                  ${gray('Options: --daemon')}

    ${cyan('sync logs')}                   View sync daemon logs
                                  ${gray('Options: --daemon, --lines')}

  ${bold('Utilities:')}
    ${cyan('help')}                        Show this help message
    ${cyan('version')}                     Show version information

${bold('EXAMPLES:')}

  ${gray('# Morning brain boot')}
  ${cyan('evna boot')} ${yellow('"what was I working on yesterday?"')}

  ${gray('# Search with project filter')}
  ${cyan('evna search')} ${yellow('"performance optimization"')} --project floatctl

  ${gray('# Capture note to active context')}
  ${cyan('evna active')} ${yellow('"finished PR review"')} --capture

  ${gray('# Ask orchestrator with session resume')}
  ${cyan('evna ask')} ${yellow('"continue debugging"')} --session abc-123

  ${gray('# List recent sessions')}
  ${cyan('evna sessions list')} --n 5 --project floatctl

  ${gray('# Check sync status')}
  ${cyan('evna sync status')}

${bold('OPTIONS:')}
  --project     Filter by project name (fuzzy match)
  --days        Lookback days for brain_boot (default: 7)
  --limit       Max results (default: 10)
  --threshold   Similarity threshold 0-1 (default: 0.5)
  --since       Filter by ISO timestamp
  --github      GitHub username for PR/issue status
  --capture     Capture message to active context
  --session     Resume ask_evna session by ID
  --fork        Fork existing session
  --timeout     Timeout in milliseconds
  --daemon      Daemon type: daily or dispatch
  --wait        Wait for sync to complete
  --lines       Number of log lines (default: 50)
  --n           Number of results (default: 10)
  --first       First N messages from session
  --last        Last N messages from session
  --truncate    Truncate long messages (chars)
  --json        Output as JSON
  --quiet       Minimal output

${gray('For more information: https://github.com/yourusername/evna')}
`);
}

/**
 * Handle brain_boot command
 */
async function handleBoot(args: string[], options: Record<string, any>): Promise<void> {
  await loadTools();

  const query = args[0];

  if (!query) {
    console.error(red('Error: boot requires a query argument'));
    console.error(`Usage: ${cyan('evna boot')} ${yellow('<query>')} ${gray('[options]')}`);
    process.exit(1);
  }

  const params = {
    query,
    project: options.project,
    lookbackDays: options.days ? parseInt(options.days) : 7,
    maxResults: options.limit ? parseInt(options.limit) : 10,
    githubUsername: options.github,
  };

  if (!options.quiet) {
    console.error(gray(`🧠 Brain boot: ${query}`));
    if (params.project) console.error(gray(`   Project: ${params.project}`));
    if (params.githubUsername) console.error(gray(`   GitHub: ${params.githubUsername}`));
    console.error('');
  }

  try {
    const result = await brainBoot.boot(params);

    if (options.json) {
      console.log(JSON.stringify(result, null, 2));
    } else {
      console.log(result.summary);
    }
  } catch (error) {
    console.error(red('Error during brain boot:'), error);
    process.exit(1);
  }
}

/**
 * Handle search command
 */
async function handleSearch(args: string[], options: Record<string, any>): Promise<void> {
  await loadTools();

  const query = args[0];

  if (!query) {
    console.error(red('Error: search requires a query argument'));
    console.error(`Usage: ${cyan('evna search')} ${yellow('<query>')} ${gray('[options]')}`);
    process.exit(1);
  }

  const params = {
    query,
    limit: options.limit ? parseInt(options.limit) : 10,
    project: options.project,
    since: options.since,
    threshold: options.threshold ? parseFloat(options.threshold) : 0.5,
  };

  if (!options.quiet) {
    console.error(gray(`🔍 Semantic search: ${query}`));
    if (params.project) console.error(gray(`   Project: ${params.project}`));
    if (params.threshold !== 0.5) console.error(gray(`   Threshold: ${params.threshold}`));
    console.error('');
  }

  try {
    const results = await search.search(params);

    if (options.json) {
      console.log(JSON.stringify(results, null, 2));
    } else {
      const formatted = search.formatResults(results);
      console.log(formatted);
    }
  } catch (error) {
    console.error(red('Error during search:'), error);
    process.exit(1);
  }
}

/**
 * Handle active context command
 */
async function handleActive(args: string[], options: Record<string, any>): Promise<void> {
  await loadTools();

  const query = args[0];

  if (!query && !options.capture) {
    console.error(red('Error: active requires either a query argument or --capture option'));
    console.error(`Usage: ${cyan('evna active')} ${yellow('[query]')} ${gray('[options]')}`);
    console.error(`       ${cyan('evna active')} ${yellow('"message"')} --capture`);
    process.exit(1);
  }

  const params = {
    query: options.capture ? undefined : query,
    capture: options.capture === true ? query : options.capture,
    limit: options.limit ? parseInt(options.limit) : 10,
    project: options.project,
    client_type: options.client,
    include_cross_client: !options['no-cross-client'],
    synthesize: !options['no-synthesize'],
  };

  if (!options.quiet) {
    if (params.capture) {
      console.error(gray(`📝 Capturing: ${params.capture}`));
    } else {
      console.error(gray(`📋 Active context: ${query}`));
      if (params.project) console.error(gray(`   Project: ${params.project}`));
    }
    console.error('');
  }

  try {
    const result = await activeContext.query(params);

    if (options.json) {
      console.log(JSON.stringify(result, null, 2));
    } else {
      console.log(result);
    }
  } catch (error) {
    console.error(red('Error querying active context:'), error);
    process.exit(1);
  }
}

/**
 * Handle ask_evna orchestrator command
 */
async function handleAsk(args: string[], options: Record<string, any>): Promise<void> {
  await loadTools();

  const query = args[0];

  if (!query) {
    console.error(red('Error: ask requires a query argument'));
    console.error(`Usage: ${cyan('evna ask')} ${yellow('<query>')} ${gray('[options]')}`);
    process.exit(1);
  }

  const params = {
    query,
    session_id: options.session,
    fork_session: options.fork || false,
    timeout_ms: options.timeout ? parseInt(options.timeout) : undefined,
    include_projects_context: !options['no-projects'],
    all_projects: options['all-projects'] || false,
  };

  if (!options.quiet) {
    console.error(gray(`🤔 Ask EVNA: ${query}`));
    if (params.session_id) console.error(gray(`   Session: ${params.session_id}`));
    if (params.fork_session) console.error(gray(`   Mode: fork`));
    console.error('');
  }

  try {
    const result = await askEvna.ask(params);

    if (options.json) {
      console.log(JSON.stringify(result, null, 2));
    } else {
      // Format the response nicely
      console.log(result.response);
      if (result.session_id && !options.quiet) {
        console.error('');
        console.error(gray(`💾 Session: ${result.session_id}`));
        console.error(gray(`   Resume with: ${cyan('evna ask')} ${yellow('"follow up question"')} --session ${result.session_id}`));
      }
    }
  } catch (error) {
    console.error(red('Error during ask_evna:'), error);
    process.exit(1);
  }
}

/**
 * Handle sessions commands
 */
async function handleSessions(args: string[], options: Record<string, any>): Promise<void> {
  await loadTools();

  const subcommand = args[0];

  if (!subcommand || subcommand === 'list') {
    // List recent sessions
    const params = {
      n: options.n ? parseInt(options.n) : 10,
      project: options.project,
    };

    if (!options.quiet) {
      console.error(gray(`📜 Recent Claude Code sessions`));
      if (params.project) console.error(gray(`   Project filter: ${params.project}`));
      console.error('');
    }

    try {
      const result = await floatctlClaude.listRecentSessions(params);
      console.log(result);
    } catch (error) {
      console.error(red('Error listing sessions:'), error);
      process.exit(1);
    }
  } else if (subcommand === 'read') {
    // Read session context
    const sessionId = args[1];

    if (!sessionId) {
      console.error(red('Error: sessions read requires a session ID'));
      console.error(`Usage: ${cyan('evna sessions read')} ${yellow('<session-id>')} ${gray('[options]')}`);
      process.exit(1);
    }

    const params = {
      sessions: [sessionId],
      first: options.first ? parseInt(options.first) : undefined,
      last: options.last ? parseInt(options.last) : undefined,
      truncate: options.truncate ? parseInt(options.truncate) : undefined,
      project: options.project,
    };

    if (!options.quiet) {
      console.error(gray(`📖 Reading session: ${sessionId}`));
      console.error('');
    }

    try {
      const result = await floatctlClaude.readRecentContext(params);
      console.log(result);
    } catch (error) {
      console.error(red('Error reading session:'), error);
      process.exit(1);
    }
  } else {
    console.error(red(`Unknown sessions subcommand: ${subcommand}`));
    console.error(`Usage: ${cyan('evna sessions')} ${yellow('list|read')} ${gray('[options]')}`);
    process.exit(1);
  }
}

/**
 * Handle sync commands
 */
async function handleSync(args: string[], options: Record<string, any>): Promise<void> {
  await loadTools();

  const subcommand = args[0];

  if (!subcommand || subcommand === 'status') {
    const daemonType = (options.daemon || 'daily') as 'daily' | 'dispatch';

    if (!options.quiet) {
      console.error(gray(`📊 R2 Sync Status (${daemonType})`));
      console.error('');
    }

    try {
      const result = await r2Sync.status({ daemon_type: daemonType });
      console.log(result);
    } catch (error) {
      console.error(red('Error checking sync status:'), error);
      process.exit(1);
    }
  } else if (subcommand === 'trigger') {
    const daemonType = (options.daemon || 'daily') as 'daily' | 'dispatch';
    const wait = options.wait || false;

    if (!options.quiet) {
      console.error(gray(`🔄 Triggering sync (${daemonType})...`));
      console.error('');
    }

    try {
      const result = await r2Sync.trigger({ daemon_type: daemonType, wait });
      console.log(result);
    } catch (error) {
      console.error(red('Error triggering sync:'), error);
      process.exit(1);
    }
  } else if (subcommand === 'start') {
    const daemonType = (options.daemon || 'daily') as 'daily' | 'dispatch';

    if (!options.quiet) {
      console.error(gray(`▶️  Starting daemon (${daemonType})...`));
      console.error('');
    }

    try {
      const result = await r2Sync.start({ daemon_type: daemonType });
      console.log(result);
    } catch (error) {
      console.error(red('Error starting daemon:'), error);
      process.exit(1);
    }
  } else if (subcommand === 'stop') {
    const daemonType = (options.daemon || 'daily') as 'daily' | 'dispatch';

    if (!options.quiet) {
      console.error(gray(`⏹️  Stopping daemon (${daemonType})...`));
      console.error('');
    }

    try {
      const result = await r2Sync.stop({ daemon_type: daemonType });
      console.log(result);
    } catch (error) {
      console.error(red('Error stopping daemon:'), error);
      process.exit(1);
    }
  } else if (subcommand === 'logs') {
    const daemonType = (options.daemon || 'daily') as 'daily' | 'dispatch';
    const lines = options.lines ? parseInt(options.lines) : 50;

    if (!options.quiet) {
      console.error(gray(`📋 Sync logs (${daemonType}, last ${lines} lines)`));
      console.error('');
    }

    try {
      const result = await r2Sync.logs({ daemon_type: daemonType, lines });
      console.log(result);
    } catch (error) {
      console.error(red('Error reading logs:'), error);
      process.exit(1);
    }
  } else {
    console.error(red(`Unknown sync subcommand: ${subcommand}`));
    console.error(`Usage: ${cyan('evna sync')} ${yellow('status|trigger|start|stop|logs')} ${gray('[options]')}`);
    process.exit(1);
  }
}

/**
 * Show version information
 */
function showVersion(): void {
  const packageJson = require('../package.json');
  console.log(`EVNA v${packageJson.version}`);
  console.log(gray('Context synthesis and brain boot for cognitive workflows'));
}

/**
 * Main CLI entry point
 */
async function main(): Promise<void> {
  const { command, args, options } = parseArgs();

  try {
    switch (command) {
      case 'boot':
        await handleBoot(args, options);
        break;

      case 'search':
        await handleSearch(args, options);
        break;

      case 'active':
        await handleActive(args, options);
        break;

      case 'ask':
        await handleAsk(args, options);
        break;

      case 'sessions':
        await handleSessions(args, options);
        break;

      case 'sync':
        await handleSync(args, options);
        break;

      case 'version':
        showVersion();
        break;

      case 'help':
      default:
        showHelp();
        break;
    }
  } catch (error) {
    console.error(red('Fatal error:'), error);
    process.exit(1);
  }
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error(red('Unhandled error:'), error);
    process.exit(1);
  });
}

export { main };
