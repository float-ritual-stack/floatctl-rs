import { execFile } from 'child_process';
import { promisify } from 'util';

const execFileAsync = promisify(execFile);

export interface DaemonStatus {
  name: string;
  running: boolean;
  pid?: number;
  last_sync?: string;
  status_message: string;
}

export interface SyncResult {
  daemon: string;
  success: boolean;
  files_transferred?: number;
  bytes_transferred?: number;
  message: string;
}

export interface R2SyncStatusOptions {
  daemon_type?: 'daily' | 'dispatch' | 'all';
}

export interface R2SyncTriggerOptions {
  daemon_type?: 'daily' | 'dispatch' | 'all';
  wait?: boolean;
}

export interface R2SyncStartOptions {
  daemon_type?: 'daily' | 'dispatch' | 'all';
}

export interface R2SyncStopOptions {
  daemon_type?: 'daily' | 'dispatch' | 'all';
}

export interface R2SyncLogsOptions {
  daemon_type: 'daily' | 'dispatch';
  lines?: number;
}

export class R2SyncTool {
  private floatctlBin: string;

  constructor() {
    // Default to floatctl in PATH, but allow override via env
    this.floatctlBin = process.env.FLOATCTL_BIN ?? 'floatctl';
  }

  async status(options: R2SyncStatusOptions = {}): Promise<string> {
    try {
      const args = ['sync', 'status', '--format', 'json'];
      if (options.daemon_type && options.daemon_type !== 'all') {
        args.push('--daemon', options.daemon_type);
      }

      const { stdout } = await execFileAsync(this.floatctlBin, args, {
        maxBuffer: 10 * 1024 * 1024, // 10MB
        timeout: 30_000, // 30 seconds
        env: { ...process.env, RUST_LOG: 'off' },
      });

      const statuses: DaemonStatus[] = JSON.parse(stdout);
      return this.formatStatusMarkdown(statuses);
    } catch (error: any) {
      return `❌ Error getting sync status: ${error.message}`;
    }
  }

  async trigger(options: R2SyncTriggerOptions = {}): Promise<string> {
    try {
      const args = ['sync', 'trigger'];
      if (options.daemon_type && options.daemon_type !== 'all') {
        args.push('--daemon', options.daemon_type);
      }
      if (options.wait) {
        args.push('--wait');
      }

      const { stdout } = await execFileAsync(this.floatctlBin, args, {
        maxBuffer: 10 * 1024 * 1024,
        timeout: 120_000, // 2 minutes for sync operations
        env: { ...process.env, RUST_LOG: 'off' },
      });

      return `✅ Sync triggered successfully\n\n${stdout.trim()}`;
    } catch (error: any) {
      return `❌ Error triggering sync: ${error.message}`;
    }
  }

  async start(options: R2SyncStartOptions = {}): Promise<string> {
    try {
      const args = ['sync', 'start'];
      if (options.daemon_type && options.daemon_type !== 'all') {
        args.push('--daemon', options.daemon_type);
      }

      const { stdout } = await execFileAsync(this.floatctlBin, args, {
        maxBuffer: 10 * 1024 * 1024,
        timeout: 30_000,
        env: { ...process.env, RUST_LOG: 'off' },
      });

      return `✅ Daemon start command executed\n\n${stdout.trim()}`;
    } catch (error: any) {
      return `❌ Error starting daemon: ${error.message}`;
    }
  }

  async stop(options: R2SyncStopOptions = {}): Promise<string> {
    try {
      const args = ['sync', 'stop'];
      if (options.daemon_type && options.daemon_type !== 'all') {
        args.push('--daemon', options.daemon_type);
      }

      const { stdout } = await execFileAsync(this.floatctlBin, args, {
        maxBuffer: 10 * 1024 * 1024,
        timeout: 30_000,
        env: { ...process.env, RUST_LOG: 'off' },
      });

      return `✅ Daemon stop command executed\n\n${stdout.trim()}`;
    } catch (error: any) {
      return `❌ Error stopping daemon: ${error.message}`;
    }
  }

  async logs(options: R2SyncLogsOptions): Promise<string> {
    try {
      const args = ['sync', 'logs', options.daemon_type];
      if (options.lines) {
        args.push('--lines', options.lines.toString());
      }

      const { stdout } = await execFileAsync(this.floatctlBin, args, {
        maxBuffer: 10 * 1024 * 1024,
        timeout: 10_000,
        env: { ...process.env, RUST_LOG: 'off' },
      });

      return stdout.trim();
    } catch (error: any) {
      return `❌ Error reading logs: ${error.message}`;
    }
  }

  private formatStatusMarkdown(statuses: DaemonStatus[]): string {
    let output = '## R2 Sync Status\n\n';

    for (const status of statuses) {
      const emoji = status.running ? '✅' : '❌';
      output += `${emoji} **${status.name}**: ${status.status_message}\n`;
      if (status.pid) {
        output += `   - PID: ${status.pid}\n`;
      }
      if (status.last_sync) {
        output += `   - Last sync: ${status.last_sync}\n`;
      }
      output += '\n';
    }

    return output;
  }
}
