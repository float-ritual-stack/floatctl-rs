/**
 * File-based logger for EVNA
 * Writes to ~/.evna/logs/ to avoid polluting stdout/stderr (MCP uses these for JSON-RPC)
 */

import { appendFile, mkdir } from "fs/promises";
import { join } from "path";
import { homedir } from "os";

class Logger {
  private logDir: string;
  private enabled: boolean;

  constructor() {
    this.logDir = join(homedir(), ".floatctl", "logs");
    this.enabled = process.env.EVNA_DEBUG === "true";
    
    // Ensure log directory exists
    mkdir(this.logDir, { recursive: true }).catch(() => {});
  }

  private async write(level: string, component: string, message: string, data?: any) {
    if (!this.enabled) return;

    const timestamp = new Date().toISOString();
    const logEntry = {
      timestamp,
      level,
      component,
      message,
      ...(data ? { data } : {}),
    };

    const logLine = JSON.stringify(logEntry) + "\n";
    const logFile = join(this.logDir, "evna-mcp.jsonl");

    try {
      await appendFile(logFile, logLine, "utf-8");
    } catch {
      // Silently fail - don't break on logging errors
    }
  }

  log(component: string, message: string, data?: any) {
    this.write("info", component, message, data);
  }

  error(component: string, message: string, data?: any) {
    this.write("error", component, message, data);
    // Also log to stderr for immediate visibility in development
    if (this.enabled) {
      console.error(`[${component}] ${message}`, data || "");
    }
  }

  debug(component: string, message: string, data?: any) {
    this.write("debug", component, message, data);
  }
}

// Singleton logger instance
export const logger = new Logger();

// Convenience functions
export const log = (component: string, message: string, data?: any) => 
  logger.log(component, message, data);

export const error = (component: string, message: string, data?: any) => 
  logger.error(component, message, data);

export const debug = (component: string, message: string, data?: any) => 
  logger.debug(component, message, data);
