/**
 * Multi-location .env loader with priority cascade
 *
 * Priority (highest to lowest):
 * 1. Current directory .env (local overrides)
 * 2. ~/.floatctl/.env (global defaults)
 * 3. Environment variables already set
 *
 * Philosophy: Allow per-project overrides while maintaining global defaults
 */

import { config as dotenvConfig } from "dotenv";
import { existsSync } from "fs";
import { join } from "path";
import { homedir } from "os";

/**
 * Load environment variables from multiple locations with priority
 *
 * Unlike standard dotenv which only checks current directory,
 * this implements a fallback chain for zero-config operation.
 */
export function loadEnvWithFallback(): void {
  const cwd = process.cwd();
  const localEnv = join(cwd, ".env");
  const globalEnv = join(homedir(), ".floatctl", ".env");

  // Only show debug output if EVNA_DEBUG is set
  const debug = !!process.env.EVNA_DEBUG;
  const quiet = !debug; // Quiet by default, verbose only with EVNA_DEBUG

  // Track what we loaded for debugging
  const loaded: string[] = [];

  // Priority 1: Current directory .env (overrides)
  if (existsSync(localEnv)) {
    dotenvConfig({ path: localEnv, override: false, quiet }); // Don't override existing vars
    loaded.push(`local (${localEnv})`);
  }

  // Priority 2: Global ~/.floatctl/.env (defaults)
  if (existsSync(globalEnv)) {
    dotenvConfig({ path: globalEnv, override: false, quiet }); // Don't override local or existing
    loaded.push(`global (${globalEnv})`);
  }

  // Priority 3: Environment variables already set (lowest priority, no action needed)
  // These are already in process.env

  // Debug logging (only if EVNA_DEBUG env var is set)
  if (process.env.EVNA_DEBUG) {
    console.error(`[env-loader] Loaded from: ${loaded.join(", ") || "none (using existing env vars only)"}`);
  }
}
