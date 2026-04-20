#!/usr/bin/env node
/**
 * Minimal floatty-explorer MCP server for evna
 * Data tools only — no render-ui, no HTML build required.
 * Queries floatty-server via HTTP (ngrok tunnel from float-box).
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

// ── Config ─────────────────────────────────────────────────────────

const FLOATTY_URL = process.env.FLOATTY_URL;
const FLOATTY_API_KEY = process.env.FLOATTY_API_KEY;

if (!FLOATTY_URL || !FLOATTY_API_KEY) {
  console.error("Missing FLOATTY_URL or FLOATTY_API_KEY");
  process.exit(1);
}

// ── HTTP client ────────────────────────────────────────────────────

async function floattyFetch<T>(path: string): Promise<T> {
  const res = await fetch(`${FLOATTY_URL}${path}`, {
    headers: {
      Authorization: `Bearer ${FLOATTY_API_KEY}`,
      "Content-Type": "application/json",
    },
  });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`Floatty ${res.status}: ${body}`);
  }
  return res.json() as Promise<T>;
}

function textResult(data: unknown) {
  return { content: [{ type: "text" as const, text: JSON.stringify(data, null, 2) }] };
}

function errorResult(msg: string) {
  return { content: [{ type: "text" as const, text: `Error: ${msg}` }], isError: true };
}

// ── Tools ──────────────────────────────────────────────────────────

async function main() {
  const server = new McpServer({ name: "floatty-explorer", version: "1.0.0" });

  // expand_page — fetch a page's subtree by title
  server.tool(
    "expand_page",
    "Fetch a page's subtree by title from the floatty outline.",
    { title: z.string().describe("Page title to look up") },
    async ({ title }) => {
      try {
        const params = new URLSearchParams({ prefix: title, limit: "5" });
        const pagesRes = await floattyFetch<{ pages: { name: string; blockId: string | null }[] }>(
          `/api/v1/pages/search?${params}`
        );
        const pages = pagesRes.pages ?? [];
        if (!pages.length) return textResult({ error: `Page "${title}" not found` });

        const exact = pages.find((p) => p.name.toLowerCase() === title.toLowerCase());
        const match = exact ?? pages[0];
        if (!match.blockId) return textResult({ error: `Page "${match.name}" is a stub` });

        const block = await floattyFetch<{ tree?: { depth: number; content: string }[] }>(
          `/api/v1/blocks/${match.blockId}?include=tree`
        );
        const lines = (block.tree ?? []).slice(0, 200).map(n => `${"  ".repeat(n.depth)}${n.content}`);
        return textResult({ page: match.name, blockId: match.blockId, blockCount: block.tree?.length ?? 0, tree: lines.join("\n") });
      } catch (e) { return errorResult(String(e)); }
    }
  );

  // get_block — fetch a specific block by UUID
  server.tool(
    "get_block",
    "Fetch a block by UUID with its subtree and ancestors.",
    {
      blockId: z.string().describe("Block UUID"),
      includeTree: z.boolean().optional().describe("Include subtree (default true)"),
    },
    async ({ blockId, includeTree = true }) => {
      try {
        const includes = ["ancestors"];
        if (includeTree) includes.push("tree");
        const block = await floattyFetch<{
          id: string; content: string; blockType: string; outputType?: string | null;
          childIds?: string[]; metadata?: { outlinks?: string[]; renderedMarkdown?: string | null } | null;
          ancestors?: { id: string; content: string }[];
          tree?: { depth: number; content: string }[];
        }>(`/api/v1/blocks/${blockId}?include=${includes.join(",")}`);

        const lines = (block.tree ?? []).slice(0, 200).map(n => `${"  ".repeat(n.depth)}${n.content}`);
        return textResult({
          blockId: block.id, content: block.content, blockType: block.blockType,
          breadcrumb: block.ancestors?.map(a => a.content) ?? [],
          outlinks: block.metadata?.outlinks ?? [],
          childCount: block.childIds?.length ?? 0,
          tree: lines.join("\n"), treeBlockCount: block.tree?.length ?? 0,
          ...(block.outputType === "door" && block.metadata?.renderedMarkdown
            ? { renderedMarkdown: block.metadata.renderedMarkdown } : {}),
        });
      } catch (e) { return errorResult(String(e)); }
    }
  );

  // search_blocks — full-text search
  server.tool(
    "search_blocks",
    "Search the floatty outline. Returns blocks with breadcrumb context and metadata.",
    {
      query: z.string().describe("Search query"),
      limit: z.number().optional().describe("Max results (default 15)"),
    },
    async ({ query, limit = 15 }) => {
      try {
        const params = new URLSearchParams({ q: query, limit: String(limit), include_breadcrumb: "true", include_metadata: "true" });
        const results = await floattyFetch<{
          total: number;
          hits: { content: string; snippet: string | null; breadcrumb?: string[]; metadata?: { outlinks?: string[] } | null }[];
        }>(`/api/v1/search?${params}`);
        return textResult({
          total: results.total,
          hits: results.hits.map(h => ({ content: h.content, snippet: h.snippet, breadcrumb: h.breadcrumb, outlinks: h.metadata?.outlinks })),
        });
      } catch (e) { return errorResult(String(e)); }
    }
  );

  // get_inbound — backlinks via [[wikilinks]]
  server.tool(
    "get_inbound",
    "Find blocks that link TO a target via [[wikilinks]].",
    { target: z.string().describe("Page or link name") },
    async ({ target }) => {
      try {
        const params = new URLSearchParams({ outlink: target, limit: "15", include_breadcrumb: "true", include_metadata: "true" });
        const results = await floattyFetch<{ total: number; hits: { content: string; breadcrumb?: string[] }[] }>(`/api/v1/search?${params}`);
        return textResult({ total: results.total, refs: results.hits.map(h => ({ content: h.content, breadcrumb: h.breadcrumb })) });
      } catch (e) { return errorResult(String(e)); }
    }
  );

  // presence — where is the human right now?
  server.tool(
    "presence",
    "Check where the human is focused in the floatty outline right now.",
    {},
    async () => {
      try {
        const res = await fetch(`${FLOATTY_URL}/api/v1/presence`, {
          headers: { Authorization: `Bearer ${FLOATTY_API_KEY}` },
        });
        if (res.status === 204) return textResult({ focused: false });
        const data = await res.json();
        return textResult(data);
      } catch (e) { return errorResult(String(e)); }
    }
  );

  // Connect
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error(`floatty-explorer MCP (evna) connected — ${FLOATTY_URL}`);
}

main().catch((err) => { console.error("Fatal:", err); process.exit(1); });
