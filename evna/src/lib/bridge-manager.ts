/**
 * Bridge Manager
 * Manages knowledge bridges - check, build, extend, and connect bridge documents
 *
 * Bridges are grep-able markdown files with YAML frontmatter stored in:
 * ~/float-hub/float.dispatch/bridges/
 *
 * Philosophy: Self-organizing knowledge graph via [[wiki-links]] and daily roots
 */

import { readFile, writeFile, mkdir, readdir, access } from "fs/promises";
import { join } from "path";
import { homedir } from "os";

export interface BridgeSearchHistory {
  timestamp: string;
  query: string;
  tools_used: string[];
  result_quality: "excellent" | "good" | "medium" | "low";
  token_cost?: number;
}

export interface BridgeOptions {
  topic: string;
  query: string;
  findings: string;
  tools_used: string[];
  result_quality?: "excellent" | "good" | "medium" | "low";
  connections?: string[];
}

export interface ExtendOptions {
  query: string;
  findings: string;
  tools_used: string[];
}

export class BridgeManager {
  private bridgesDir: string;

  constructor() {
    this.bridgesDir = join(homedir(), "float-hub", "float.dispatch", "bridges");
  }

  /**
   * Ensure bridges directory exists
   */
  private async ensureBridgesDir(): Promise<void> {
    try {
      await mkdir(this.bridgesDir, { recursive: true });
    } catch (error) {
      // Directory likely exists, ignore
    }
  }

  /**
   * Convert topic to filename slug
   * "Grep Patterns Discovery" → "grep-patterns-discovery"
   */
  private slugify(topic: string): string {
    return topic
      .toLowerCase()
      .trim()
      .replace(/[^\w\s-]/g, "") // Remove special chars
      .replace(/[\s_]+/g, "-")   // Replace spaces/underscores with dash
      .replace(/^-+|-+$/g, "");  // Trim dashes
  }

  /**
   * Get bridge file path
   */
  private getBridgePath(topic: string): string {
    const slug = this.slugify(topic);
    return join(this.bridgesDir, `${slug}.bridge.md`);
  }

  /**
   * Get daily root file path
   */
  private getDailyRootPath(date?: string): string {
    const dateStr = date || new Date().toISOString().split("T")[0]; // YYYY-MM-DD
    return join(this.bridgesDir, `${dateStr}.bridge.md`);
  }

  /**
   * Check if bridge exists for topic
   * Returns bridge content if found, null otherwise
   */
  async checkBridge(topic: string): Promise<string | null> {
    await this.ensureBridgesDir();

    const bridgePath = this.getBridgePath(topic);

    try {
      const content = await readFile(bridgePath, "utf-8");
      console.error(`[bridge-manager] Found existing bridge: ${topic}`);
      return content;
    } catch (error) {
      // Bridge doesn't exist
      return null;
    }
  }

  /**
   * Build new bridge document
   */
  async buildBridge(options: BridgeOptions): Promise<string> {
    await this.ensureBridgesDir();

    const { topic, query, findings, tools_used, result_quality = "good", connections = [] } = options;
    const slug = this.slugify(topic);
    const bridgePath = this.getBridgePath(topic);

    // Check if bridge already exists
    const existing = await this.checkBridge(topic);
    if (existing) {
      return `Bridge "${topic}" already exists. Use extend action to add to it.`;
    }

    // Get current date and time
    const now = new Date();
    const date = now.toISOString().split("T")[0]; // YYYY-MM-DD
    const time = now.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: true
    }); // HH:MM AM/PM

    // Build frontmatter
    const frontmatter = {
      type: "bridge_document",
      created: `${date} @ ${time}`,
      topic: slug,
      daily_root: `[[${date}]]`,
      related_queries: [query],
      connected_bridges: connections,
      search_history: [
        {
          timestamp: `${date} @ ${time}`,
          query,
          tools_used,
          result_quality,
        },
      ],
    };

    // Build content
    const content = `---
${Object.entries(frontmatter)
  .map(([key, value]) => `${key}: ${JSON.stringify(value)}`)
  .join("\n")}
---

# ${topic}

## What This Is
${findings}

## Search History
- **${date} @ ${time}**: ${query}
  - Tools: ${tools_used.join(", ")}
  - Quality: ${result_quality}

## Connected Bridges
${connections.length > 0 ? connections.map(c => `- [[${c}]]`).join("\n") : "*(No connections yet)*"}

## Daily Root
Part of: [[${date}]]
`;

    // Write bridge file
    await writeFile(bridgePath, content, "utf-8");

    // Update daily root
    await this.updateDailyRoot(date, slug, "created");

    console.error(`[bridge-manager] Created bridge: ${topic}`);
    return `Bridge "${topic}" created successfully.\n\nPath: ~/float-hub/float.dispatch/bridges/${slug}.bridge.md\nDaily root: [[${date}]]`;
  }

  /**
   * Extend existing bridge with new findings
   */
  async extendBridge(topic: string, options: ExtendOptions): Promise<string> {
    await this.ensureBridgesDir();

    const bridgePath = this.getBridgePath(topic);
    const { query, findings, tools_used } = options;

    // Check if bridge exists
    const existing = await this.checkBridge(topic);
    if (!existing) {
      return `Bridge "${topic}" not found. Use build action to create it first.`;
    }

    // Get current date and time
    const now = new Date();
    const date = now.toISOString().split("T")[0];
    const time = now.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: true
    });

    // Append to search history section
    const extension = `\n## Additional Search (${date} @ ${time})
- **Query**: ${query}
- **Tools**: ${tools_used.join(", ")}
- **Findings**: ${findings}
`;

    await writeFile(bridgePath, existing + extension, "utf-8");

    // Update daily root
    await this.updateDailyRoot(date, this.slugify(topic), "extended");

    console.error(`[bridge-manager] Extended bridge: ${topic}`);
    return `Bridge "${topic}" extended with new findings.`;
  }

  /**
   * Merge multiple bridges into one
   * Consolidates content, search history, and connections from source bridges into target
   */
  async mergeBridges(targetTopic: string, sourceTopics: string[]): Promise<string> {
    await this.ensureBridgesDir();

    const targetPath = this.getBridgePath(targetTopic);

    // Check if target exists
    const targetBridge = await this.checkBridge(targetTopic);
    if (!targetBridge) {
      return `Target bridge "${targetTopic}" not found. Use build action to create it first.`;
    }

    // Get current date and time
    const now = new Date();
    const date = now.toISOString().split("T")[0];
    const time = now.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: true
    });

    let mergedContent = targetBridge;
    const mergedBridges: string[] = [];

    // Process each source bridge
    for (const sourceTopic of sourceTopics) {
      const sourceBridge = await this.checkBridge(sourceTopic);
      if (!sourceBridge) {
        console.error(`[bridge-manager] Warning: Source bridge "${sourceTopic}" not found, skipping`);
        continue;
      }

      // Extract content sections (skip frontmatter and title)
      const sourceLines = sourceBridge.split("\n");
      let inFrontmatter = false;
      let pastTitle = false;
      const contentToMerge: string[] = [];

      for (const line of sourceLines) {
        if (line === "---") {
          if (!inFrontmatter) {
            inFrontmatter = true;
            continue;
          } else {
            inFrontmatter = false;
            continue;
          }
        }
        if (inFrontmatter) continue;
        if (!pastTitle && line.startsWith("# ")) {
          pastTitle = true;
          continue;
        }
        if (pastTitle) {
          contentToMerge.push(line);
        }
      }

      // Append merged content
      mergedContent += `\n## Merged from [[${this.slugify(sourceTopic)}]] (${date} @ ${time})\n`;
      mergedContent += contentToMerge.join("\n");

      mergedBridges.push(sourceTopic);

      // Delete source bridge file
      const sourcePath = this.getBridgePath(sourceTopic);
      try {
        await writeFile(sourcePath, "", "utf-8"); // Clear content
        console.error(`[bridge-manager] Merged and archived: ${sourceTopic}`);
      } catch (error) {
        console.error(`[bridge-manager] Warning: Could not archive source bridge "${sourceTopic}": ${error}`);
      }
    }

    if (mergedBridges.length === 0) {
      return `No valid source bridges found to merge into "${targetTopic}".`;
    }

    // Write updated target
    await writeFile(targetPath, mergedContent, "utf-8");

    // Update daily root
    await this.updateDailyRoot(date, this.slugify(targetTopic), "extended");

    console.error(`[bridge-manager] Merged ${mergedBridges.length} bridge(s) into: ${targetTopic}`);
    return `Successfully merged ${mergedBridges.length} bridge(s) into "${targetTopic}": ${mergedBridges.join(", ")}`;
  }

  /**
   * Connect bridges together
   */
  async connectBridges(fromTopic: string, toTopics: string[]): Promise<string> {
    await this.ensureBridgesDir();

    const bridgePath = this.getBridgePath(fromTopic);

    // Check if bridge exists
    const existing = await this.checkBridge(fromTopic);
    if (!existing) {
      return `Bridge "${fromTopic}" not found. Use build action to create it first.`;
    }

    // Find the "Connected Bridges" section and add new connections
    const lines = existing.split("\n");
    const connectedIndex = lines.findIndex(line => line.startsWith("## Connected Bridges"));

    if (connectedIndex === -1) {
      return `Could not find "Connected Bridges" section in bridge "${fromTopic}".`;
    }

    // Build connection links
    const newConnections = toTopics.map(topic => `- [[${this.slugify(topic)}]]`).join("\n");

    // Replace the placeholder or append
    const hasPlaceholder = lines[connectedIndex + 1]?.includes("*(No connections yet)*");
    if (hasPlaceholder) {
      lines[connectedIndex + 1] = newConnections;
    } else {
      // Append after the section header
      lines.splice(connectedIndex + 1, 0, newConnections);
    }

    await writeFile(bridgePath, lines.join("\n"), "utf-8");

    console.error(`[bridge-manager] Connected bridge ${fromTopic} to: ${toTopics.join(", ")}`);
    return `Bridge "${fromTopic}" connected to: ${toTopics.join(", ")}`;
  }

  /**
   * Update daily root bridge
   */
  private async updateDailyRoot(date: string, bridgeSlug: string, action: "created" | "extended"): Promise<void> {
    const rootPath = this.getDailyRootPath(date);

    try {
      // Try to read existing daily root
      const existing = await readFile(rootPath, "utf-8");

      // Add bridge reference if not already there
      if (!existing.includes(`[[${bridgeSlug}]]`)) {
        const lines = existing.split("\n");
        const bridgesIndex = lines.findIndex(line => line.startsWith("## Bridges"));

        if (bridgesIndex !== -1) {
          // Find the end of the bridges list
          let insertIndex = bridgesIndex + 1;
          while (insertIndex < lines.length && lines[insertIndex].startsWith("- ")) {
            insertIndex++;
          }
          lines.splice(insertIndex, 0, `- [[${bridgeSlug}]] (${action})`);
          await writeFile(rootPath, lines.join("\n"), "utf-8");
        }
      }
    } catch (error) {
      // Daily root doesn't exist, create it
      const content = `---
type: bridge_root
date: ${date}
---

# ${date} Bridge Root

## Bridges Created Today
- [[${bridgeSlug}]] (${action})

## Search Patterns
*(Patterns emerge organically)*
`;
      await writeFile(rootPath, content, "utf-8");
      console.error(`[bridge-manager] Created daily root: ${date}`);
    }
  }

  /**
   * List all bridges
   */
  async listBridges(): Promise<string[]> {
    await this.ensureBridgesDir();

    try {
      const files = await readdir(this.bridgesDir);
      return files
        .filter(f => f.endsWith(".bridge.md"))
        .filter(f => !f.match(/^\d{4}-\d{2}-\d{2}\.bridge\.md$/)) // Exclude daily roots
        .map(f => f.replace(".bridge.md", ""));
    } catch (error) {
      return [];
    }
  }

  /**
   * Find daily root
   */
  async findDailyRoot(date?: string): Promise<string | null> {
    await this.ensureBridgesDir();

    const rootPath = this.getDailyRootPath(date);

    try {
      const content = await readFile(rootPath, "utf-8");
      return content;
    } catch (error) {
      return null;
    }
  }
}
