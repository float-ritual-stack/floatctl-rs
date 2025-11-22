/**
 * Bridge Health Tool
 * 
 * Analyzes bridge documents for maintenance needs:
 * - Detect large bridges that need splitting
 * - Find duplicates/similar content
 * - Identify stale bridges
 * - Suggest bridges ready for imprint promotion
 */

import { readdir, readFile, stat } from "fs/promises";
import { join } from "path";
import { homedir } from "os";
import { ollama, OLLAMA_MODELS } from "../lib/ollama-client.js";

export interface BridgeHealthOptions {
  report_type?: "duplicates" | "large" | "stale" | "ready_for_imprint" | "all";
  max_age_days?: number;      // For stale detection (default: 90)
  large_threshold_kb?: number; // For large bridge detection (default: 10)
  use_ollama?: boolean;        // Enable Ollama analysis (default: true if running)
}

export interface BridgeInfo {
  path: string;
  filename: string;
  size_kb: number;
  mtime: Date;
  age_days: number;
  frontmatter?: Record<string, any>;
  content_preview?: string;
}

export interface BridgeHealthReport {
  large_bridges: BridgeInfo[];
  stale_bridges: BridgeInfo[];
  duplicate_candidates: Array<{ bridges: BridgeInfo[]; similarity: number }>;
  ready_for_imprint: BridgeInfo[];
  summary: string;
}

export class BridgeHealthTool {
  private bridgesDir: string;

  constructor(bridgesDir?: string) {
    this.bridgesDir = bridgesDir || join(homedir(), "float-hub", "float.dispatch", "bridges");
  }

  /**
   * Analyze bridge health and return report
   */
  async analyze(options: BridgeHealthOptions = {}): Promise<BridgeHealthReport> {
    const {
      report_type = "all",
      max_age_days = 90,
      large_threshold_kb = 10,
      use_ollama = true,
    } = options;

    // Select best available Ollama model
    const selectedModel = use_ollama ? await ollama.selectModel(OLLAMA_MODELS.balanced) : null;
    if (use_ollama && !selectedModel) {
      console.error("[bridge_health] Ollama not available, falling back to basic analysis");
    }

    // Scan all bridges
    const bridges = await this.scanBridges();

    // Run analysis based on report type
    const large_bridges = (report_type === "large" || report_type === "all")
      ? this.detectLargeBridges(bridges, large_threshold_kb)
      : [];

    const stale_bridges = (report_type === "stale" || report_type === "all")
      ? this.detectStaleBridges(bridges, max_age_days)
      : [];

    const duplicate_candidates = (report_type === "duplicates" || report_type === "all")
      ? await this.detectDuplicates(bridges, selectedModel)
      : [];

    const ready_for_imprint = (report_type === "ready_for_imprint" || report_type === "all")
      ? await this.detectReadyForImprint(bridges, selectedModel)
      : [];

    // Generate summary
    const summary = this.generateSummary({
      total: bridges.length,
      large: large_bridges.length,
      stale: stale_bridges.length,
      duplicates: duplicate_candidates.length,
      ready: ready_for_imprint.length,
    });

    return {
      large_bridges,
      stale_bridges,
      duplicate_candidates,
      ready_for_imprint,
      summary,
    };
  }

  /**
   * Scan bridges directory and collect metadata
   */
  private async scanBridges(): Promise<BridgeInfo[]> {
    const bridges: BridgeInfo[] = [];

    try {
      const files = await readdir(this.bridgesDir);
      const mdFiles = files.filter(f => f.endsWith(".md") && !f.startsWith("."));

      for (const filename of mdFiles) {
        const path = join(this.bridgesDir, filename);
        const stats = await stat(path);
        const content = await readFile(path, "utf-8");

        const age_days = Math.floor((Date.now() - stats.mtime.getTime()) / (1000 * 60 * 60 * 24));
        const size_kb = Math.round(stats.size / 1024);

        // Extract frontmatter (simple parse)
        const frontmatter = this.parseFrontmatter(content);
        const content_preview = content.substring(0, 500);

        bridges.push({
          path,
          filename,
          size_kb,
          mtime: stats.mtime,
          age_days,
          frontmatter,
          content_preview,
        });
      }
    } catch (error) {
      console.error("[bridge_health] Error scanning bridges:", error);
    }

    return bridges;
  }

  /**
   * Detect bridges that are too large (need splitting)
   */
  private detectLargeBridges(bridges: BridgeInfo[], threshold_kb: number): BridgeInfo[] {
    return bridges
      .filter(b => b.size_kb > threshold_kb)
      .sort((a, b) => b.size_kb - a.size_kb);
  }

  /**
   * Detect bridges that haven't been modified recently
   */
  private detectStaleBridges(bridges: BridgeInfo[], max_age_days: number): BridgeInfo[] {
    return bridges
      .filter(b => b.age_days > max_age_days)
      .sort((a, b) => b.age_days - a.age_days);
  }

  /**
   * Detect duplicate or very similar bridges using Ollama embeddings
   */
  private async detectDuplicates(
    bridges: BridgeInfo[],
    model: string | null
  ): Promise<Array<{ bridges: BridgeInfo[]; similarity: number }>> {
    if (!model || bridges.length < 2) {
      return [];
    }

    try {
      // Select embedding model from fallback chain
      const embeddingModel = await ollama.selectModel(OLLAMA_MODELS.embeddings);
      if (!embeddingModel) {
        return [];
      }

      // Generate embeddings for all bridges (use first 1000 chars)
      const embeddings = await Promise.all(
        bridges.map(async (bridge) => {
          const text = bridge.content_preview || "";
          const embedding = await ollama.embeddings({
            model: embeddingModel,
            prompt: text,
          });
          return { bridge, embedding };
        })
      );

      // Find pairs with high cosine similarity (>0.85)
      const duplicatePairs: Array<{ bridges: BridgeInfo[]; similarity: number }> = [];

      for (let i = 0; i < embeddings.length; i++) {
        for (let j = i + 1; j < embeddings.length; j++) {
          const similarity = this.cosineSimilarity(
            embeddings[i].embedding,
            embeddings[j].embedding
          );

          if (similarity > 0.85) {
            duplicatePairs.push({
              bridges: [embeddings[i].bridge, embeddings[j].bridge],
              similarity,
            });
          }
        }
      }

      return duplicatePairs.sort((a, b) => b.similarity - a.similarity);
    } catch (error) {
      console.error("[bridge_health] Error detecting duplicates:", error);
      return [];
    }
  }

  /**
   * Detect bridges ready for imprint promotion using Ollama scoring
   */
  private async detectReadyForImprint(
    bridges: BridgeInfo[],
    model: string | null
  ): Promise<BridgeInfo[]> {
    if (!model) {
      // Fallback: simple heuristics
      return bridges.filter(b =>
        b.size_kb > 5 &&        // Substantial content
        b.age_days > 7 &&       // Aged at least a week
        b.age_days < 90         // Not too stale
      ).slice(0, 5);
    }

    try {
      // Score each bridge for maturity (0-100)
      const scored = await Promise.all(
        bridges.slice(0, 20).map(async (bridge) => { // Limit to 20 most recent
          const score = await this.scoreBridgeMaturity(bridge, model);
          return { bridge, score };
        })
      );

      // Return bridges with score >70
      return scored
        .filter(s => s.score > 70)
        .sort((a, b) => b.score - a.score)
        .map(s => s.bridge);
    } catch (error) {
      console.error("[bridge_health] Error scoring bridges:", error);
      return [];
    }
  }

  /**
   * Score bridge maturity using Ollama (0-100)
   */
  private async scoreBridgeMaturity(bridge: BridgeInfo, model: string): Promise<number> {
    const prompt = `
Analyze this knowledge bridge and rate its readiness for publication (0-100).

Consider:
- Content completeness (has introduction, body, conclusions?)
- Quality of writing (clear, coherent, well-structured?)
- Actionable insights (practical value?)
- Self-contained (understandable on its own?)

Bridge: ${bridge.filename}
Size: ${bridge.size_kb}KB
Age: ${bridge.age_days} days

Content preview:
${bridge.content_preview}

Rate 0-100 (just the number):
`.trim();

    try {
      const response = await ollama.generate({
        model, // use passed model from fallback chain
        prompt,
        temperature: 0.3, // Low temperature for consistent scoring
      });

      // Extract number from response
      const match = response.match(/\b(\d{1,3})\b/);
      return match ? Math.min(100, parseInt(match[1], 10)) : 50;
    } catch (error) {
      console.error(`[bridge_health] Error scoring ${bridge.filename}:`, error);
      return 0;
    }
  }

  /**
   * Calculate cosine similarity between two vectors
   */
  private cosineSimilarity(a: number[], b: number[]): number {
    if (a.length !== b.length) return 0;

    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }

    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
  }

  /**
   * Parse YAML frontmatter from markdown
   */
  private parseFrontmatter(content: string): Record<string, any> | undefined {
    const match = content.match(/^---\n([\s\S]*?)\n---/);
    if (!match) return undefined;

    const frontmatterText = match[1];
    const parsed: Record<string, any> = {};

    // Simple line-by-line parsing (good enough for basic fields)
    frontmatterText.split("\n").forEach(line => {
      const colonIndex = line.indexOf(":");
      if (colonIndex > 0) {
        const key = line.substring(0, colonIndex).trim();
        const value = line.substring(colonIndex + 1).trim();
        parsed[key] = value;
      }
    });

    return parsed;
  }

  /**
   * Generate markdown summary of health report
   */
  private generateSummary(stats: {
    total: number;
    large: number;
    stale: number;
    duplicates: number;
    ready: number;
  }): string {
    return `
## Bridge Health Summary

**Total bridges analyzed**: ${stats.total}

### Maintenance Recommendations

${stats.large > 0 ? `🔴 **${stats.large} large bridges** need splitting (>10KB)` : "✅ No oversized bridges"}
${stats.stale > 0 ? `⚠️  **${stats.stale} stale bridges** (no updates >90 days)` : "✅ No stale bridges"}
${stats.duplicates > 0 ? `🔄 **${stats.duplicates} duplicate pairs** detected` : "✅ No duplicates detected"}
${stats.ready > 0 ? `🎯 **${stats.ready} bridges** ready for imprint promotion` : "📝 No bridges ready for promotion yet"}

### Next Steps
${stats.large > 0 ? "- Review large bridges and consider splitting into atomic notes\n" : ""}${stats.stale > 0 ? "- Archive or refresh stale bridges\n" : ""}${stats.duplicates > 0 ? "- Merge duplicate content\n" : ""}${stats.ready > 0 ? "- Promote mature bridges to imprints\n" : ""}
`.trim();
  }

  /**
   * Format full health report as markdown
   */
  formatReport(report: BridgeHealthReport): string {
    let output = report.summary + "\n\n---\n\n";

    if (report.large_bridges.length > 0) {
      output += "## Large Bridges (>10KB)\n\n";
      report.large_bridges.forEach(b => {
        output += `- **${b.filename}** (${b.size_kb}KB) - Last modified ${b.age_days} days ago\n`;
      });
      output += "\n";
    }

    if (report.stale_bridges.length > 0) {
      output += "## Stale Bridges (>90 days)\n\n";
      report.stale_bridges.forEach(b => {
        output += `- **${b.filename}** (${b.age_days} days old) - ${b.size_kb}KB\n`;
      });
      output += "\n";
    }

    if (report.duplicate_candidates.length > 0) {
      output += "## Duplicate/Similar Bridges\n\n";
      report.duplicate_candidates.forEach(({ bridges, similarity }) => {
        output += `- **${Math.round(similarity * 100)}% similar**:\n`;
        output += `  - ${bridges[0].filename}\n`;
        output += `  - ${bridges[1].filename}\n\n`;
      });
    }

    if (report.ready_for_imprint.length > 0) {
      output += "## Ready for Imprint Promotion\n\n";
      report.ready_for_imprint.forEach(b => {
        output += `- **${b.filename}** (${b.size_kb}KB, ${b.age_days} days old)\n`;
      });
      output += "\n";
    }

    return output;
  }
}
