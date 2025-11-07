/**
 * Claude Projects Context Hook
 * 
 * Agent SDK hook that injects recent conversation snippets from ~/.claude/projects
 * into ask_evna system prompt, giving EVNA "peripheral vision" into recent work
 */

import { getAskEvnaContextInjection, getAllProjectsContextInjection } from "../lib/claude-projects-context.js";

export interface ClaudeProjectsContextHookOptions {
  enabled?: boolean;        // Enable context injection (default: true)
  allProjects?: boolean;    // Include all projects vs just evna (default: false)
  maxProjects?: number;     // Override default max projects
  maxFiles?: number;        // Override default max files per project
  headLines?: number;       // Override default head lines
  tailLines?: number;       // Override default tail lines
  maxAge?: number;          // Override default max age in hours
}

/**
 * Hook to inject Claude projects context into system prompt
 * 
 * Usage in Agent SDK:
 * - Add to hooks array when creating query options
 * - Triggered before each turn
 * - Injects recent conversation context into system prompt
 */
export async function claudeProjectsContextHook(
  eventName: string,
  options: ClaudeProjectsContextHookOptions = {}
): Promise<{ systemPromptAppend?: string } | void> {
  const {
    enabled = true,
    allProjects = false,
    maxProjects,
    maxFiles,
    headLines,
    tailLines,
    maxAge,
  } = options;

  // Only inject on specific events (avoid redundant injections)
  if (eventName !== "UserPromptSubmit" && eventName !== "BeforeTurn") {
    return;
  }

  if (!enabled) {
    return;
  }

  try {
    console.error(`[claude-projects-context-hook] Injecting context (allProjects: ${allProjects})`);
    
    const contextInjection = allProjects
      ? await getAllProjectsContextInjection({
          maxProjects,
          maxFiles,
          headLines,
          tailLines,
          maxAge,
        })
      : await getAskEvnaContextInjection({
          maxProjects,
          maxFiles,
          headLines,
          tailLines,
          maxAge,
        });

    if (contextInjection) {
      return {
        systemPromptAppend: contextInjection,
      };
    }
  } catch (error) {
    console.error("[claude-projects-context-hook] Error:", error);
    // Graceful degradation - don't block on context injection failures
  }
}

/**
 * Create a configured hook function for Agent SDK
 */
export function createClaudeProjectsContextHook(
  options: ClaudeProjectsContextHookOptions = {}
) {
  return async (eventName: string) => {
    return claudeProjectsContextHook(eventName, options);
  };
}
