/**
 * FloatConfig - Centralized configuration loader
 *
 * Reads from ~/.floatctl/config.toml (single source of truth)
 * Replaces hardcoded paths from workspace-context.json
 */

import { readFileSync } from 'fs';
import * as toml from 'toml';
import * as path from 'path';
import * as os from 'os';

export interface FloatConfigPaths {
  float_home: string;
  daily_notes_home: string;
  daily_notes: string;
  bridges: string;
  operations: string;
  inbox: string;
  dispatches: string;
  archives?: string;
}

export interface FloatConfigMachine {
  name: string;
  environment: string;
  region?: string;
}

export interface FloatConfigEvna {
  database_url: string;
  system_prompt?: string;
  mcp_server_port?: number;
  active_context_ttl?: string;
  sessions_dir?: string;
}

export interface FloatConfig {
  machine: FloatConfigMachine;
  paths: FloatConfigPaths;
  evna?: FloatConfigEvna;
  floatctl?: any;
  r2?: any;
  integrations?: any;
}

/**
 * Load centralized floatctl config from ~/.floatctl/config.toml
 *
 * Fails hard with actionable error if config doesn't exist
 * Expands ${var} references using environment variables
 */
export function loadFloatConfig(): FloatConfig {
  const configPath = path.join(os.homedir(), '.floatctl', 'config.toml');

  try {
    const content = readFileSync(configPath, 'utf-8');
    let config = toml.parse(content) as any;

    // Apply machine-specific overrides
    // IMPORTANT: Only override base paths (float_home, daily_notes_home), not derived paths
    // Derived paths will be recalculated via variable expansion
    // This matches Rust behavior in floatctl-core/src/config.rs:120-154
    const machine = process.env.FLOATCTL_MACHINE || config.machine.name;

    // Check for paths override (only float_home, daily_notes_home)
    const pathsKey = `paths.${machine}`;
    if (config[pathsKey]) {
      const overrides = config[pathsKey];
      if (overrides.float_home) config.paths.float_home = overrides.float_home;
      if (overrides.daily_notes_home) config.paths.daily_notes_home = overrides.daily_notes_home;
      // Derived paths (daily_notes, bridges, etc.) NOT overridden - recalculated via variable expansion
    }

    // Check for evna override (only database_url, mcp_server_port)
    const evnaKey = `evna.${machine}`;
    if (config[evnaKey] && config.evna) {
      const overrides = config[evnaKey];
      if (overrides.database_url) config.evna.database_url = overrides.database_url;
      if (overrides.mcp_server_port !== undefined) config.evna.mcp_server_port = overrides.mcp_server_port;
      // Other evna fields NOT overridden
    }

    // Expand variables
    config = expandVariables(config);

    return config as FloatConfig;
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      throw new Error(
        `Config not found at ${configPath}\n\n` +
        `Run: floatctl config init --detect`
      );
    }
    throw new Error(`Failed to load config: ${error.message}`);
  }
}

/**
 * Expand ${var} references in config
 *
 * Uses two-pass expansion to handle nested variables correctly:
 * 1. First pass: Expand base paths (float_home, daily_notes_home) using only env vars
 * 2. Second pass: Add expanded base paths to vars, then expand dependent paths
 *
 * Supports:
 * - ${HOME} - User home directory
 * - ${float_home} - Float home path (from config)
 * - ${daily_notes_home} - Daily notes path (from config)
 * - ${DATABASE_URL} - From environment variable
 * - Other env vars: ${VARNAME}
 */
function expandVariables(config: any): any {
  // First pass: Environment variables only
  const envVars: Record<string, string> = {
    HOME: os.homedir(),
    DATABASE_URL: process.env.DATABASE_URL || '',
    R2_ACCOUNT_ID: process.env.R2_ACCOUNT_ID || '',
    R2_API_TOKEN: process.env.R2_API_TOKEN || '',
    COHERE_API_KEY: process.env.COHERE_API_KEY || '',
    OPENAI_API_KEY: process.env.OPENAI_API_KEY || '',
    ANTHROPIC_API_KEY: process.env.ANTHROPIC_API_KEY || '',
  };

  // Expand base paths first using only env vars
  if (config.paths) {
    config.paths.float_home = expandString(config.paths.float_home, envVars);
    config.paths.daily_notes_home = expandString(config.paths.daily_notes_home, envVars);
  }

  // Second pass: Add expanded base paths to vars for dependent path expansion
  const vars = { ...envVars };
  if (config.paths) {
    vars.float_home = config.paths.float_home;
    vars.daily_notes_home = config.paths.daily_notes_home;
  }

  // Recursively expand all string values
  return expandObject(config, vars);
}

function expandObject(obj: any, vars: Record<string, string>): any {
  if (typeof obj === 'string') {
    return expandString(obj, vars);
  }

  if (Array.isArray(obj)) {
    return obj.map(item => expandObject(item, vars));
  }

  if (obj !== null && typeof obj === 'object') {
    const result: any = {};
    for (const [key, value] of Object.entries(obj)) {
      result[key] = expandObject(value, vars);
    }
    return result;
  }

  return obj;
}

function expandString(s: string, vars: Record<string, string>): string {
  let result = s;

  for (const [key, value] of Object.entries(vars)) {
    const pattern = `\${${key}}`;
    result = result.replace(new RegExp(pattern.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'), value);
  }

  return result;
}

/**
 * Get config value by dot-notation key
 * Example: get(config, 'paths.float_home')
 */
export function getConfigValue(config: FloatConfig, key: string): any {
  const parts = key.split('.');
  let value: any = config;

  for (const part of parts) {
    if (value === undefined || value === null) {
      return undefined;
    }
    value = value[part];
  }

  return value;
}

/**
 * Validate all paths exist and are accessible
 * Throws error with helpful message if validation fails
 */
export function validatePaths(config: FloatConfig): void {
  const fs = require('fs');
  const pathsToCheck = [
    ['float_home', config.paths.float_home],
    ['daily_notes_home', config.paths.daily_notes_home],
    ['daily_notes', config.paths.daily_notes],
    ['bridges', config.paths.bridges],
    ['operations', config.paths.operations],
    ['inbox', config.paths.inbox],
    ['dispatches', config.paths.dispatches],
  ] as [string, string][];

  const errors: string[] = [];

  for (const [name, pathValue] of pathsToCheck) {
    try {
      const stats = fs.statSync(pathValue);
      if (!stats.isDirectory()) {
        errors.push(`  ✗ ${name}: ${pathValue} (not a directory)`);
      }
    } catch (error: any) {
      errors.push(`  ✗ ${name}: ${pathValue} (does not exist)`);
    }
  }

  if (errors.length > 0) {
    throw new Error(`Path validation failed:\n${errors.join('\n')}`);
  }
}
