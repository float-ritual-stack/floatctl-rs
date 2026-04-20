/**
 * Annotation Parser
 * Extracts data annotations (::) and populates metadata
 */

import workspaceContextData from '../config/workspace-context.json';
import { canonicalizeProject } from './canonicalize.js';

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
 * Heuristic: does this string look like prose that got miscaptured as a project?
 * Real projects are short, no sentence punctuation, dash/slash/word-identifier shape.
 * Triggered by parser misses where free-form text lands in the project column
 * (seen in the 2026-04-20 Supabase audit — 34 such rows in 14 days).
 */
function looksLikeProse(s: string): boolean {
  if (s.length > 80) return true;
  // sentence-shape punctuation that never appears in a project name
  if (/[.!?]\s|[:;]\s/.test(s)) return true;
  if (s.split(/\s+/).length > 6) return true;
  return false;
}

/**
 * Normalize project name to canonical form.
 *
 * Order of operations:
 *   1. Reject prose — return the raw value so downstream can decide to NULL it
 *      (we don't silently drop because loud failure beats silent data loss)
 *   2. Canonicalize (strip trailing " |", collapse " / ", lowercase)
 *   3. Match against workspace canonical+aliases
 *   4. Fall back to the canonicalized value if no alias match
 *      (trailing-pipe drift is healed even without an alias entry)
 */
function normalizeProjectName(rawProject: string): string {
  if (looksLikeProse(rawProject)) {
    // Preserve as-is; captureMessage logic will decide whether to NULL
    return rawProject;
  }

  const canonicalized = canonicalizeProject(rawProject);
  if (!canonicalized) return rawProject;

  // Match against workspace canonical+aliases (case-insensitive, post-canonicalize)
  for (const config of Object.values(workspace.projects)) {
    const allVariants = [config.canonical, ...config.aliases].map(v => v.toLowerCase());
    if (allVariants.includes(canonicalized)) {
      return config.canonical;
    }
  }

  // No alias match — return the canonicalized form (not the raw with drift)
  return canonicalized;
}

export interface ParsedAnnotation {
  type: string;
  value: string;
  fullMatch: string;
}

export interface MessageMetadata {
  ctx?: {
    timestamp?: string;
    date?: string;
    time?: string;
    mode?: string;
    metadata?: string;
  };
  project?: string;
  issue?: string;
  personas?: string[];
  connections?: string[];
  highlights?: string[];
  commands?: string[];
  patterns?: string[];
  temporal?: {
    extracted_timestamp?: string;
    unix_timestamp?: number;
  };
}

export class AnnotationParser {
  /**
   * Parse all annotations from message content
   */
  parseAnnotations(content: string): ParsedAnnotation[] {
    const annotations: ParsedAnnotation[] = [];

    // Match pattern: <word>::<value>
    // The lookahead stops at the NEXT annotation start, end-of-line, or a
    // bare " | " delimiter — the pipe case fixes a drift bug where
    // "project::X | mode::Y" was capturing value="X |" because the
    // lookahead only recognized `\s+\w+::` as a boundary (the pipe +
    // whitespace before `mode::` slipped into the value).
    const annotationRegex = /(\w+)::\s*([^\n]+?)(?=\s+\w+::|\s+\|\s|$)/g;

    let match;
    while ((match = annotationRegex.exec(content)) !== null) {
      // Defense-in-depth: also strip any residual trailing " |" from the
      // captured value (covers edge cases like end-of-line pipes).
      const value = match[2].trim().replace(/\s*\|\s*$/, '');
      annotations.push({
        type: match[1],
        value,
        fullMatch: match[0],
      });
    }

    return annotations;
  }

  /**
   * Extract metadata from annotations
   */
  extractMetadata(content: string): MessageMetadata {
    const annotations = this.parseAnnotations(content);
    const metadata: MessageMetadata = {
      personas: [],
      connections: [],
      highlights: [],
      commands: [],
      patterns: [],
    };

    for (const annotation of annotations) {
      switch (annotation.type.toLowerCase()) {
        case 'ctx':
          metadata.ctx = this.parseCtxAnnotation(annotation.value);
          // Extract project from ctx:: value if present (e.g., ctx::... [project::foo])
          const ctxProjectMatch = annotation.value.match(/\[project::\s*([^\]]+)\]/);
          if (ctxProjectMatch && !metadata.project) {
            metadata.project = normalizeProjectName(ctxProjectMatch[1]);
          }
          // Extract issue from ctx:: value if present (e.g., ctx::... [issue::123])
          const ctxIssueMatch = annotation.value.match(/\[issue::\s*([^\]]+)\]/);
          if (ctxIssueMatch && !metadata.issue) {
            metadata.issue = ctxIssueMatch[1].trim();
          }
          break;

        case 'project':
          // Projects can appear as:
          //   - "project::name"                 (bare)
          //   - "project::name - prose tail"    (direct annotation w/ dash separator)
          //   - "project::a, b, c"              (multi-project — take first)
          //   - "project::name; other meta"     (semicolon separator)
          //
          // Split on common prose delimiters so the tail doesn't bleed
          // into the project identifier. Projects themselves never contain
          // ` - ` (space-dash-space) or commas or semicolons.
          const firstProjectToken = annotation.value
            .split(/[,;]|\s+-\s+/)[0]
            .trim();
          metadata.project = normalizeProjectName(firstProjectToken);
          break;

        case 'issue':
          metadata.issue = annotation.value.trim();
          break;

        case 'karen':
        case 'lf1m':
        case 'sysop':
        case 'evna':
        case 'qtb':
          if (!metadata.personas) metadata.personas = [];
          metadata.personas.push(annotation.type);
          break;

        case 'connectto':
          if (!metadata.connections) metadata.connections = [];
          metadata.connections.push(annotation.value);
          break;

        case 'highlight':
        case 'eureka':
        case 'gotcha':
        case 'insight':
          if (!metadata.highlights) metadata.highlights = [];
          metadata.highlights.push(annotation.value);
          break;

        case 'pattern':
        case 'bridge':
        case 'note':
          if (!metadata.patterns) metadata.patterns = [];
          metadata.patterns.push(`${annotation.type}:${annotation.value}`);
          break;

        default:
          // Capture other annotations as patterns
          if (!metadata.patterns) metadata.patterns = [];
          metadata.patterns.push(`${annotation.type}:${annotation.value}`);
      }
    }

    // Extract float.* commands
    const floatCommands = content.match(/float\.\w+\([^)]*\)/g);
    if (floatCommands) {
      metadata.commands = floatCommands;
    }

    // Extract temporal information
    metadata.temporal = this.extractTemporal(content);

    return metadata;
  }

  /**
   * Parse ctx:: annotation format
   * Examples:
   * - ctx::2025-10-21 @ 08:25:54 AM - [project::float/evna]
   * - ctx:: 2025-07-28 - session complete - [mode:: semantic archival]
   */
  private parseCtxAnnotation(value: string): MessageMetadata['ctx'] {
    const ctx: MessageMetadata['ctx'] = {};

    // Extract date (YYYY-MM-DD)
    const dateMatch = value.match(/(\d{4}-\d{2}-\d{2})/);
    if (dateMatch) {
      ctx.date = dateMatch[1];
      ctx.timestamp = dateMatch[1];
    }

    // Extract time (@ HH:MM:SS AM/PM or @ HH:MM AM/PM)
    const timeMatch = value.match(/@\s*(\d{1,2}:\d{2}(?::\d{2})?\s*(?:AM|PM)?)/i);
    if (timeMatch) {
      ctx.time = timeMatch[1].trim();
      if (ctx.date) {
        ctx.timestamp = `${ctx.date} ${ctx.time}`;
      }
    }

    // Extract mode ([mode:: value])
    const modeMatch = value.match(/\[mode::\s*([^\]]+)\]/);
    if (modeMatch) {
      ctx.mode = modeMatch[1].trim();
    }

    // Extract remaining metadata
    const metadataMatch = value.match(/-\s*\[([^\]]+)\]/);
    if (metadataMatch) {
      ctx.metadata = metadataMatch[1].trim();
    }

    return ctx;
  }

  /**
   * Extract temporal information from content
   */
  private extractTemporal(content: string): MessageMetadata['temporal'] {
    const temporal: MessageMetadata['temporal'] = {};

    // Look for ISO timestamps
    const isoMatch = content.match(/(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?)/);
    if (isoMatch) {
      temporal.extracted_timestamp = isoMatch[1];
      try {
        temporal.unix_timestamp = new Date(isoMatch[1]).getTime();
      } catch (e) {
        console.error('[annotation-parser] Failed to parse timestamp:', {
          timestamp: isoMatch[1],
          error: e instanceof Error ? e.message : String(e),
        });
      }
    }

    return temporal;
  }

  /**
   * Check if content has annotations
   */
  hasAnnotations(content: string): boolean {
    return /\w+::\s*.+/.test(content);
  }

  /**
   * Extract project from various formats:
   * - project::name
   * - [project::name]
   * - ctx:: ... [project::name]
   */
  extractProject(content: string): string | undefined {
    // Direct project:: annotation
    const directMatch = content.match(/project::\s*([^\s\]]+)/);
    if (directMatch) {
      // Handle comma-separated values - return first project
      return directMatch[1].split(',')[0].trim();
    }

    // Project in ctx:: metadata
    const ctxProjectMatch = content.match(/\[project::\s*([^\]]+)\]/);
    if (ctxProjectMatch) {
      // Handle comma-separated values - return first project
      return ctxProjectMatch[1].split(',')[0].trim();
    }

    return undefined;
  }

  /**
   * Extract all persona invocations from content
   */
  extractPersonas(content: string): string[] {
    const personas = new Set<string>();
    const personaRegex = /(karen|lf1m|sysop|evna|qtb)::/gi;

    let match;
    while ((match = personaRegex.exec(content)) !== null) {
      personas.add(match[1].toLowerCase());
    }

    return Array.from(personas);
  }
}
