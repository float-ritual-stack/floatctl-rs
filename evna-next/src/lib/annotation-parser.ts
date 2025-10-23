/**
 * Annotation Parser
 * Extracts data annotations (::) and populates metadata
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
 * Normalize project name to canonical form
 * Philosophy: "LLMs as fuzzy compilers" - gentle normalization on capture
 */
function normalizeProjectName(rawProject: string): string {
  const lowerProject = rawProject.toLowerCase().trim();

  // Find matching canonical or alias
  for (const [key, config] of Object.entries(workspace.projects)) {
    const allVariants = [config.canonical, ...config.aliases].map(v => v.toLowerCase());
    if (allVariants.includes(lowerProject)) {
      return config.canonical;
    }
  }

  // No exact match - return original (user might be adding new project)
  return rawProject;
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
    // Handles multi-line and complex values
    const annotationRegex = /(\w+)::\s*([^\n]+?)(?=\s+\w+::|$)/g;

    let match;
    while ((match = annotationRegex.exec(content)) !== null) {
      annotations.push({
        type: match[1],
        value: match[2].trim(),
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
          break;

        case 'project':
          metadata.project = normalizeProjectName(annotation.value);
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
        // Invalid timestamp, skip
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
    if (directMatch) return directMatch[1];

    // Project in ctx:: metadata
    const ctxProjectMatch = content.match(/\[project::\s*([^\]]+)\]/);
    if (ctxProjectMatch) return ctxProjectMatch[1];

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
