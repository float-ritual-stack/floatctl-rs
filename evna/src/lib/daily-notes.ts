/**
 * Daily notes reader for Evans Notes
 * Reads and parses markdown daily notes from ~/.evans-notes/daily
 */

import { readFile, readdir } from 'fs/promises';
import { join } from 'path';
import { homedir } from 'os';

export interface DailyNote {
  date: string;
  path: string;
  content: string;
  frontmatter?: Record<string, any>;
  sections: Array<{
    heading: string;
    content: string;
    level: number;
  }>;
}

export class DailyNotesReader {
  private notesDir: string;

  constructor(notesDir?: string) {
    this.notesDir = notesDir || join(homedir(), '.evans-notes', 'daily');
  }

  /**
   * Get recent daily notes (files matching YYYY-MM-DD.md pattern)
   */
  async getRecentNotes(days: number = 3): Promise<DailyNote[]> {
    try {
      const files = await readdir(this.notesDir);

      // Filter for daily note files (YYYY-MM-DD.md)
      const dailyNotePattern = /^(\d{4}-\d{2}-\d{2})\.md$/;
      const dailyNotes = files
        .filter(f => dailyNotePattern.test(f))
        .map(f => ({
          file: f,
          date: f.replace('.md', ''),
        }))
        .sort((a, b) => b.date.localeCompare(a.date)) // Most recent first
        .slice(0, days);

      const notes: DailyNote[] = [];
      for (const note of dailyNotes) {
        const content = await readFile(join(this.notesDir, note.file), 'utf-8');
        notes.push(this.parseNote(note.date, join(this.notesDir, note.file), content));
      }

      return notes;
    } catch (error) {
      // Note: No console.error here - MCP uses stderr for JSON-RPC
      return [];
    }
  }

  /**
   * Parse a daily note markdown file
   */
  private parseNote(date: string, path: string, content: string): DailyNote {
    const lines = content.split('\n');
    let frontmatter: Record<string, any> | undefined;
    let contentStart = 0;

    // Parse frontmatter if present
    if (lines[0] === '---') {
      const endIndex = lines.slice(1).findIndex(l => l === '---');
      if (endIndex !== -1) {
        const frontmatterLines = lines.slice(1, endIndex + 1);
        frontmatter = this.parseFrontmatter(frontmatterLines);
        contentStart = endIndex + 2;
      }
    }

    // Extract content after frontmatter
    const noteContent = lines.slice(contentStart).join('\n');

    // Parse sections by headings
    const sections = this.parseSections(noteContent);

    return {
      date,
      path,
      content: noteContent,
      frontmatter,
      sections,
    };
  }

  /**
   * Parse YAML frontmatter
   */
  private parseFrontmatter(lines: string[]): Record<string, any> {
    const frontmatter: Record<string, any> = {};
    for (const line of lines) {
      const match = line.match(/^([^:]+):\s*(.+)$/);
      if (match) {
        frontmatter[match[1].trim()] = match[2].trim();
      }
    }
    return frontmatter;
  }

  /**
   * Parse markdown sections by headings
   */
  private parseSections(content: string): Array<{ heading: string; content: string; level: number }> {
    const lines = content.split('\n');
    const sections: Array<{ heading: string; content: string; level: number }> = [];
    let currentSection: { heading: string; content: string; level: number } | null = null;

    for (const line of lines) {
      const headingMatch = line.match(/^(#{1,6})\s+(.+)$/);

      if (headingMatch) {
        // Save previous section
        if (currentSection) {
          sections.push(currentSection);
        }

        // Start new section
        currentSection = {
          heading: headingMatch[2].trim(),
          content: '',
          level: headingMatch[1].length,
        };
      } else if (currentSection) {
        currentSection.content += line + '\n';
      }
    }

    // Save last section
    if (currentSection) {
      sections.push(currentSection);
    }

    return sections;
  }

  /**
   * Format recent notes as markdown summary
   */
  formatRecentNotes(notes: DailyNote[]): string {
    if (notes.length === 0) {
      return '**No recent daily notes found**';
    }

    const lines: string[] = ['## 📝 Recent Daily Notes\n'];

    notes.forEach(note => {
      lines.push(`### ${note.date}\n`);

      // Extract key sections (standup, pending PRs, focus)
      const standupSection = note.sections.find(s =>
        s.heading.toLowerCase().includes('standup') ||
        s.heading.toLowerCase().includes('evans updates')
      );

      const pendingPRsSection = note.sections.find(s =>
        s.heading.toLowerCase().includes('pending') ||
        s.heading.toLowerCase().includes('prs')
      );

      const focusSection = note.sections.find(s =>
        s.heading.toLowerCase().includes('focus') ||
        s.heading.toLowerCase().includes('today')
      );

      if (standupSection) {
        const preview = this.extractPreview(standupSection.content, 300);
        lines.push(`**${standupSection.heading}**:`);
        lines.push(preview);
        lines.push('');
      }

      if (pendingPRsSection) {
        const preview = this.extractPreview(pendingPRsSection.content, 200);
        lines.push(`**${pendingPRsSection.heading}**:`);
        lines.push(preview);
        lines.push('');
      }

      if (focusSection) {
        const preview = this.extractPreview(focusSection.content, 200);
        lines.push(`**${focusSection.heading}**:`);
        lines.push(preview);
        lines.push('');
      }

      lines.push('---\n');
    });

    return lines.join('\n');
  }

  /**
   * Extract a preview of content (first N chars, preserving task lists)
   */
  private extractPreview(content: string, maxLength: number): string {
    const lines = content.trim().split('\n').filter(l => l.trim());

    // Prioritize task lists and important markers
    const importantLines = lines.filter(l =>
      l.includes('- [ ]') ||
      l.includes('- [x]') ||
      l.includes('**') ||
      l.startsWith('-')
    );

    const preview = importantLines.length > 0
      ? importantLines.slice(0, 5).join('\n')
      : lines.slice(0, 3).join('\n');

    return preview.length > maxLength
      ? preview.substring(0, maxLength) + '...'
      : preview;
  }
}
