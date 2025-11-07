/**
 * Tool for EVNA to update her own system prompt
 * Allows self-modification and experimentation with identity/behavior
 */

import { writeFileSync, readFileSync, mkdirSync } from "fs";
import { dirname, join } from "path";
import { homedir } from "os";
import { getSystemPromptPath } from "../core/config.js";

export interface UpdateSystemPromptArgs {
  content: string;
  backup?: boolean;
}

/**
 * Update EVNA's system prompt
 * Creates backup by default before overwriting
 */
export async function updateSystemPrompt(args: UpdateSystemPromptArgs): Promise<string> {
  const { content, backup = true } = args;

  // Get current system prompt path
  const promptPath = getSystemPromptPath();
  
  // Ensure ~/.evna directory exists
  const evnaDir = join(homedir(), ".evna");
  mkdirSync(evnaDir, { recursive: true });
  
  // If updating project file, migrate to user location first
  const userPromptPath = join(evnaDir, "system-prompt.md");
  const finalPath = promptPath.includes(".evna") ? promptPath : userPromptPath;

  // Create backup if requested
  if (backup) {
    try {
      const currentContent = readFileSync(finalPath, "utf-8");
      const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
      const backupPath = join(evnaDir, `system-prompt.backup.${timestamp}.md`);
      writeFileSync(backupPath, currentContent, "utf-8");
    } catch (error) {
      // If file doesn't exist yet, no backup needed
      if ((error as any).code !== "ENOENT") {
        throw error;
      }
    }
  }

  // Write new content
  writeFileSync(finalPath, content, "utf-8");

  return `System prompt updated successfully at: ${finalPath}\n\n` +
         `Note: Changes will take effect on next session (restart CLI/TUI or reload MCP server).\n` +
         (backup ? `Backup created in ${evnaDir}\n` : "");
}

/**
 * Read current system prompt
 */
export async function readSystemPrompt(): Promise<string> {
  const promptPath = getSystemPromptPath();
  return readFileSync(promptPath, "utf-8");
}

/**
 * Append to current system prompt (for incremental updates)
 */
export async function appendSystemPrompt(args: { content: string }): Promise<string> {
  const currentContent = await readSystemPrompt();
  const newContent = currentContent + "\n\n" + args.content;
  return updateSystemPrompt({ content: newContent, backup: true });
}
