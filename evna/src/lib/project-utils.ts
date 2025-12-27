/**
 * Project Utilities
 * Shared utilities for project name handling across evna
 *
 * Philosophy: "LLMs as fuzzy compilers" - match generously, normalize gently
 */

import workspaceContextData from '../config/workspace-context.json';

// Type definitions for workspace context config (minimal - only what's needed)
interface ProjectConfig {
  canonical: string;
  aliases: string[];
  description: string;
  repo: string;
  type: string;
}

interface WorkspaceContext {
  projects: Record<string, ProjectConfig>;
  [key: string]: any; // Allow other fields we don't use here
}

const workspace = workspaceContextData as WorkspaceContext;

/**
 * Expand project name to include all known aliases
 * Used for fuzzy search queries (ILIKE matching)
 *
 * Example: "floatctl" -> ["floatctl-rs", "floatctl", "float/floatctl"]
 */
export function expandProjectAliases(project: string): string[] {
  const lowerProject = project.toLowerCase();

  // Find matching canonical or alias
  for (const [_key, config] of Object.entries(workspace.projects)) {
    const allVariants = [config.canonical, ...config.aliases].map(v => v.toLowerCase());
    if (allVariants.some(v => v.includes(lowerProject) || lowerProject.includes(v))) {
      return [config.canonical, ...config.aliases];
    }
  }

  // No match in config - return original (fuzzy match with ILIKE)
  return [project];
}

/**
 * Normalize project name to canonical form
 * Used when storing/capturing project metadata
 *
 * Example: "floatctl" -> "floatctl-rs"
 */
export function normalizeProjectName(rawProject: string): string {
  const lowerProject = rawProject.toLowerCase().trim();

  // Find matching canonical or alias
  for (const [_key, config] of Object.entries(workspace.projects)) {
    const allVariants = [config.canonical, ...config.aliases].map(v => v.toLowerCase());
    if (allVariants.includes(lowerProject)) {
      return config.canonical;
    }
  }

  // No exact match - return original (user might be adding new project)
  return rawProject;
}

/**
 * Get project config by name (canonical or alias)
 * Returns undefined if not found
 */
export function getProjectConfig(project: string): ProjectConfig | undefined {
  const lowerProject = project.toLowerCase().trim();

  for (const [_key, config] of Object.entries(workspace.projects)) {
    const allVariants = [config.canonical, ...config.aliases].map(v => v.toLowerCase());
    if (allVariants.includes(lowerProject)) {
      return config;
    }
  }

  return undefined;
}

/**
 * Check if a project name is known (canonical or alias)
 */
export function isKnownProject(project: string): boolean {
  return getProjectConfig(project) !== undefined;
}
