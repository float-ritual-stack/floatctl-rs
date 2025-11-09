/**
 * Peripheral Context Collector
 * Grab ambient awareness snippets for Ollama synthesis
 */

import { promises as fs } from 'fs';
import { join } from 'path';
import { homedir } from 'os';

export interface PeripheralContext {
  dailyNote?: string;
  askEvnaSessions?: string[];
  otherProjects?: string[];
}

/**
 * Get full daily note for today
 */
export async function getDailyNoteContext(): Promise<string | undefined> {
  try {
    const today = new Date().toISOString().split('T')[0]; // YYYY-MM-DD
    const dailyPath = join(homedir(), '.evans-notes', 'daily', `${today}.md`);
    const content = await fs.readFile(dailyPath, 'utf-8');
    
    return content.trim();
  } catch (error) {
    // File doesn't exist or other error - graceful fallback
    return undefined;
  }
}

/**
 * Get first/last lines from recent ask_evna sessions
 * Sessions stored in ~/.evna/projects/Users-evan--evna/history.jsonl
 */
export async function getAskEvnaSessionContext(sessionCount: number = 3, linesPerSession: number = 3): Promise<string[]> {
  try {
    const historyPath = join(homedir(), '.evna', 'projects', 'Users-evan--evna', 'history.jsonl');
    const content = await fs.readFile(historyPath, 'utf-8');
    const lines = content.trim().split('\n').filter(l => l.trim());
    
    // Get last N sessions
    const recentSessions = lines.slice(-sessionCount);
    
    const snippets: string[] = [];
    for (const line of recentSessions) {
      try {
        const session = JSON.parse(line);
        // Extract first/last messages
        const messages = session.messages || [];
        const firstMessages = messages.slice(0, linesPerSession).map((m: any) => 
          `[${m.role}] ${truncate(m.content, 200)}`
        );
        const lastMessages = messages.slice(-linesPerSession).map((m: any) => 
          `[${m.role}] ${truncate(m.content, 200)}`
        );
        
        const timestamp = new Date(session.created_at || session.last_used).toLocaleString();
        snippets.push(`Session ${timestamp}:\nFirst: ${firstMessages.join(' → ')}\nLast: ${lastMessages.join(' → ')}`);
      } catch {
        // Skip malformed JSON
      }
    }
    
    return snippets;
  } catch (error) {
    return [];
  }
}

/**
 * Get last few lines from other recently active projects
 * Projects in ~/.evna/projects/{project-name}/history.jsonl
 */
export async function getOtherProjectsContext(projectCount: number = 2, linesPerProject: number = 3): Promise<string[]> {
  try {
    const projectsDir = join(homedir(), '.evna', 'projects');
    const entries = await fs.readdir(projectsDir, { withFileTypes: true });
    
    const projectDirs = entries
      .filter(e => e.isDirectory() && e.name !== 'Users-evan--evna') // Exclude ask_evna itself
      .map(e => join(projectsDir, e.name));
    
    // Get modification times to find recent projects
    const projectsWithMtime = await Promise.all(
      projectDirs.map(async (dir) => {
        try {
          const historyPath = join(dir, 'history.jsonl');
          const stats = await fs.stat(historyPath);
          return { dir, mtime: stats.mtime, name: dir.split('/').pop()! };
        } catch {
          return null;
        }
      })
    );
    
    const recentProjects = projectsWithMtime
      .filter(p => p !== null)
      .sort((a, b) => b!.mtime.getTime() - a!.mtime.getTime())
      .slice(0, projectCount);
    
    const snippets: string[] = [];
    for (const project of recentProjects) {
      if (!project) continue;
      
      try {
        const historyPath = join(project.dir, 'history.jsonl');
        const content = await fs.readFile(historyPath, 'utf-8');
        const lines = content.trim().split('\n').filter(l => l.trim());
        const lastSession = lines[lines.length - 1];
        
        if (lastSession) {
          const session = JSON.parse(lastSession);
          const messages = session.messages || [];
          const lastMessages = messages.slice(-linesPerProject).map((m: any) => 
            `[${m.role}] ${truncate(m.content, 150)}`
          );
          
          const timestamp = new Date(session.last_used || session.created_at).toLocaleString();
          snippets.push(`${project.name} (${timestamp}):\n${lastMessages.join('\n')}`);
        }
      } catch {
        // Skip errors
      }
    }
    
    return snippets;
  } catch (error) {
    return [];
  }
}

/**
 * Collect all peripheral context
 */
export async function collectPeripheralContext(): Promise<PeripheralContext> {
  const [dailyNote, askEvnaSessions, otherProjects] = await Promise.all([
    getDailyNoteContext(),
    getAskEvnaSessionContext(),
    getOtherProjectsContext(),
  ]);
  
  return {
    dailyNote,
    askEvnaSessions: askEvnaSessions.length > 0 ? askEvnaSessions : undefined,
    otherProjects: otherProjects.length > 0 ? otherProjects : undefined,
  };
}

/**
 * Format peripheral context for Ollama prompt
 */
export function formatPeripheralContext(context: PeripheralContext): string {
  const sections: string[] = [];
  
  if (context.dailyNote) {
    sections.push(`### Today's Daily Note\n${context.dailyNote}`);
  }
  
  if (context.askEvnaSessions) {
    sections.push(`### Recent ask_evna Sessions\n${context.askEvnaSessions.join('\n\n---\n\n')}`);
  }
  
  if (context.otherProjects) {
    sections.push(`### Other Active Projects\n${context.otherProjects.join('\n\n---\n\n')}`);
  }
  
  if (sections.length === 0) {
    return '';
  }
  
  return `\n\n## Peripheral Context (for ambient awareness)\n\n${sections.join('\n\n---\n\n')}`;
}

/**
 * Truncate text to max length with ellipsis
 */
function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.substring(0, maxLength - 3) + '...';
}
