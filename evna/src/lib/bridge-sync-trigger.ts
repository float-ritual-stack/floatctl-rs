/**
 * Bridge Sync Trigger
 * Auto-triggers R2 sync when bridges are written (makes AutoRAG near real-time)
 */

import { watch } from "fs";
import { join } from "path";
import { homedir } from "os";
import { execFile } from "child_process";
import { promisify } from "util";

const execFileAsync = promisify(execFile);

export interface BridgeSyncTriggerOptions {
  enabled?: boolean;          // Enable auto-sync (default: true)
  debounce_ms?: number;       // Debounce writes (default: 5000 - 5 seconds)
  daemon_type?: "dispatch";   // Which daemon to trigger
}

export class BridgeSyncTrigger {
  private dispatchDir: string;
  private debounceTimer: NodeJS.Timeout | null = null;
  private pendingWrites: Set<string> = new Set();
  private enabled: boolean;
  private debounceMs: number;
  private daemonType: string;

  constructor(options: BridgeSyncTriggerOptions = {}) {
    this.dispatchDir = join(homedir(), "float-hub", "float.dispatch");
    this.enabled = options.enabled ?? true;
    this.debounceMs = options.debounce_ms ?? 5000;
    this.daemonType = options.daemon_type ?? "dispatch";
  }

  /**
   * Start watching float.dispatch directory for changes
   */
  start(): void {
    if (!this.enabled) {
      console.error("[bridge-sync-trigger] Disabled, not starting watcher");
      return;
    }

    console.error(`[bridge-sync-trigger] Watching ${this.dispatchDir} for changes`);
    console.error(`[bridge-sync-trigger] Debounce: ${this.debounceMs}ms`);

    // Watch entire float.dispatch directory recursively
    const watcher = watch(
      this.dispatchDir,
      { recursive: true },
      (eventType, filename) => {
        if (!filename || !filename.endsWith(".md")) return;

        console.error(`[bridge-sync-trigger] File changed: ${filename}`);
        this.pendingWrites.add(filename);
        this.scheduleSync();
      }
    );

    watcher.on("error", (error) => {
      console.error("[bridge-sync-trigger] Watcher error:", error);
    });
  }

  /**
   * Schedule sync with debouncing (batch rapid writes)
   */
  private scheduleSync(): void {
    // Clear existing timer
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }

    // Schedule new sync after debounce period
    this.debounceTimer = setTimeout(() => {
      this.triggerSync();
    }, this.debounceMs);
  }

  /**
   * Trigger R2 sync via floatctl
   */
  private async triggerSync(): Promise<void> {
    // Capture and clear pending set BEFORE sync to avoid race condition
    // (new writes during sync should schedule a new sync, not be lost)
    const filesToSync = new Set(this.pendingWrites);
    this.pendingWrites.clear();

    const fileCount = filesToSync.size;
    console.error(`[bridge-sync-trigger] Triggering sync for ${fileCount} changed file(s)`);

    try {
      const floatctlBin = process.env.FLOATCTL_BIN || "floatctl";
      const { stdout } = await execFileAsync(floatctlBin, [
        "sync", "trigger", "--daemon", this.daemonType
      ], { timeout: 30000 });

      console.error(`[bridge-sync-trigger] Sync triggered successfully`);
      if (stdout) console.error(`[bridge-sync-trigger] ${stdout.trim()}`);
    } catch (error) {
      console.error("[bridge-sync-trigger] Sync trigger failed:", error);
      // Re-add files on failure so they'll be retried
      for (const file of filesToSync) {
        this.pendingWrites.add(file);
      }
    }
  }
}

/**
 * Global singleton (start once on MCP server startup)
 */
let globalTrigger: BridgeSyncTrigger | null = null;

export function startBridgeSyncTrigger(options?: BridgeSyncTriggerOptions): void {
  if (globalTrigger) {
    console.error("[bridge-sync-trigger] Already running");
    return;
  }

  globalTrigger = new BridgeSyncTrigger(options);
  globalTrigger.start();
}

export function getBridgeSyncTrigger(): BridgeSyncTrigger | null {
  return globalTrigger;
}
