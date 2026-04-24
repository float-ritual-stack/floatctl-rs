#!/usr/bin/env node
/**
 * Minimal floatty-explorer MCP server for evna
 * Data tools only — no render-ui, no HTML build required.
 * Queries floatty-server via HTTP (ngrok tunnel from float-box).
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { createServer as createHttpServer, type IncomingMessage, type ServerResponse } from "node:http";
import { randomUUID } from "node:crypto";
import { z } from "zod";

// ── Config ─────────────────────────────────────────────────────────

const FLOATTY_URL = process.env.FLOATTY_URL;
const FLOATTY_API_KEY = process.env.FLOATTY_API_KEY;
const QMD_URL = process.env.QMD_URL ?? "http://localhost:5050/mcp";

if (!FLOATTY_URL || !FLOATTY_API_KEY) {
  console.error("Missing FLOATTY_URL or FLOATTY_API_KEY");
  process.exit(1);
}

// ── QMD proxy client ───────────────────────────────────────────────
// qmd's HTTP MCP returns content as `resource` blobs; we unpack to
// plain text so cowork live artifacts can consume them.

let qmdSessionId: string | null = null;

async function qmdRpc(body: unknown): Promise<unknown> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Accept: "application/json, text/event-stream",
    "MCP-Protocol-Version": "2025-06-18",
  };
  if (qmdSessionId) headers["Mcp-Session-Id"] = qmdSessionId;

  const res = await fetch(QMD_URL, { method: "POST", headers, body: JSON.stringify(body) });
  const sid = res.headers.get("mcp-session-id");
  if (sid) qmdSessionId = sid;

  const ct = res.headers.get("content-type") ?? "";
  if (ct.includes("text/event-stream")) {
    const text = await res.text();
    const last = text.split("\n").filter(l => l.startsWith("data: ")).pop();
    return last ? JSON.parse(last.slice(6)) : null;
  }
  if (res.status === 202) return null;
  return res.json();
}

async function qmdInit(): Promise<void> {
  if (qmdSessionId) return;
  await qmdRpc({
    jsonrpc: "2.0", id: 1, method: "initialize",
    params: {
      protocolVersion: "2025-06-18",
      capabilities: {},
      clientInfo: { name: "floatty-explorer-qmd-proxy", version: "1.0.0" },
    },
  });
  await qmdRpc({ jsonrpc: "2.0", method: "notifications/initialized" });
}

type McpContentBlock = {
  type: string;
  text?: string;
  resource?: { text?: string; uri?: string };
};
type McpCallResult = {
  content?: McpContentBlock[];
  structuredContent?: unknown;
  isError?: boolean;
};

/**
 * Proxy a tool call to qmd and return a normalized MCP result.
 * Unpacks `resource` content blocks to plain text so cowork live artifacts
 * (which only accept `text` content) don't reject the response.
 * Preserves `structuredContent` when qmd provides it — cowork's `call()`
 * prefers structuredContent over parsing content[].text.
 */
async function qmdCall(toolName: string, args: unknown): Promise<McpCallResult> {
  await qmdInit();
  const resp = await qmdRpc({
    jsonrpc: "2.0", id: Date.now(), method: "tools/call",
    params: { name: toolName, arguments: args },
  }) as { result?: McpCallResult; error?: { message?: string } };

  if (resp?.error) {
    return { content: [{ type: "text", text: `Error: ${resp.error.message ?? "qmd rpc error"}` }], isError: true };
  }
  const result = resp?.result;
  if (!result) return { content: [{ type: "text", text: "Error: empty qmd response" }], isError: true };

  // Normalize content: unpack resource → text, drop non-text blobs
  const normalizedContent: McpContentBlock[] = [];
  for (const b of result.content ?? []) {
    if (b.type === "text" && typeof b.text === "string") {
      normalizedContent.push({ type: "text", text: b.text });
    } else if (b.type === "resource" && b.resource?.text) {
      normalizedContent.push({ type: "text", text: b.resource.text });
    } else if (b.type === "resource" && b.resource?.uri) {
      normalizedContent.push({ type: "text", text: b.resource.uri });
    }
  }
  return {
    content: normalizedContent.length ? normalizedContent : [{ type: "text", text: "" }],
    ...(result.structuredContent !== undefined ? { structuredContent: result.structuredContent } : {}),
    ...(result.isError ? { isError: true } : {}),
  };
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

function createMcpServer(): McpServer {
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

  // qmd_search — short form: lex keyword + vec semantic in one call
  server.tool(
    "qmd_search",
    "Search qmd knowledge base with both lex and vec queries (hybrid). Unpacks qmd resource blobs to plain text.",
    {
      query: z.string().describe("Search string"),
      collection: z.string().optional().describe("Scope to a collection"),
      limit: z.number().optional().describe("Max results (default 10)"),
    },
    async ({ query, collection, limit = 10 }) => {
      try {
        const searches = [{ type: "lex", query }, { type: "vec", query }];
        const args: Record<string, unknown> = { searches, limit };
        if (collection) args.collections = [collection];
        return await qmdCall("query", args);
      } catch (e) { return errorResult(String(e)); }
    }
  );

  // qmd_query — full structured query passthrough
  server.tool(
    "qmd_query",
    "Run a structured qmd query (lex/vec/hyde sub-queries). Unpacks qmd resource blobs to plain text; preserves structuredContent.",
    {
      searches: z.array(z.object({
        type: z.enum(["lex", "vec", "hyde"]),
        query: z.string(),
      })).describe("Sub-queries — first gets 2x weight"),
      collections: z.array(z.string()).optional().describe("Filter to collections"),
      limit: z.number().optional().describe("Max results (default 10)"),
      intent: z.string().optional().describe("Disambiguation hint for reranker"),
      minScore: z.number().optional().describe("Min relevance 0-1"),
    },
    async (args) => {
      try { return await qmdCall("query", args); }
      catch (e) { return errorResult(String(e)); }
    }
  );

  // qmd_get — fetch a single document
  server.tool(
    "qmd_get",
    "Fetch a single qmd document by path or docid. Unpacks qmd resource blobs to plain text.",
    {
      file: z.string().describe("Path or docid (e.g. 'collection/file.md' or '#abc123'). Supports line offset via 'file.md:100'"),
      maxLines: z.number().optional().describe("Line cap"),
    },
    async ({ file, maxLines }) => {
      try {
        const args: Record<string, unknown> = { file };
        if (maxLines !== undefined) args.maxLines = maxLines;
        return await qmdCall("get", args);
      } catch (e) { return errorResult(String(e)); }
    }
  );

  // qmd_status — index health / collection list
  server.tool(
    "qmd_status",
    "Get qmd index health and collection list. Unpacks qmd resource blobs to plain text; preserves structuredContent when present.",
    {},
    async () => {
      try { return await qmdCall("status", {}); }
      catch (e) { return errorResult(String(e)); }
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

  return server;
}

// ── CORS + HTTP helpers ────────────────────────────────────────────

function setCorsHeaders(res: ServerResponse) {
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "POST, GET, DELETE, OPTIONS");
  res.setHeader(
    "Access-Control-Allow-Headers",
    "Content-Type, Accept, Authorization, MCP-Protocol-Version, Mcp-Session-Id, Last-Event-ID",
  );
  res.setHeader("Access-Control-Expose-Headers", "Mcp-Session-Id");
  // PNA (Chrome Private Network Access) — allow cross-origin → localhost fetches
  res.setHeader("Access-Control-Allow-Private-Network", "true");
}

async function readJsonBody(req: IncomingMessage): Promise<unknown> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) chunks.push(chunk as Buffer);
  if (!chunks.length) return undefined;
  const text = Buffer.concat(chunks).toString("utf-8");
  try { return JSON.parse(text); } catch { return undefined; }
}

// ── Entrypoints ────────────────────────────────────────────────────

async function runStdio() {
  const server = createMcpServer();
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error(`floatty-explorer MCP (stdio) connected — ${FLOATTY_URL}`);
}

async function runHttp(port: number) {
  const transports: Record<string, StreamableHTTPServerTransport> = {};

  const httpServer = createHttpServer(async (req, res) => {
    setCorsHeaders(res);
    console.error(`[http] ${req.method} ${req.url} sid=${req.headers["mcp-session-id"] ?? "-"}`);

    if (req.method === "OPTIONS") { res.writeHead(204).end(); return; }

    const url = req.url ?? "/";
    if (!url.startsWith("/mcp")) {
      res.writeHead(404, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ error: "Not found — MCP endpoint is at /mcp" }));
      return;
    }

    try {
      const sessionId = req.headers["mcp-session-id"] as string | undefined;
      let transport = sessionId ? transports[sessionId] : undefined;

      if (!transport && req.method === "POST") {
        // New session — read body to check for initialize request
        const body = await readJsonBody(req);

        transport = new StreamableHTTPServerTransport({
          sessionIdGenerator: () => randomUUID(),
          onsessioninitialized: (id) => { transports[id] = transport!; },
        });
        transport.onclose = () => {
          const sid = transport?.sessionId;
          if (sid) delete transports[sid];
        };

        const server = createMcpServer();
        await server.connect(transport);
        await transport.handleRequest(req, res, body);
        return;
      }

      if (!transport) {
        res.writeHead(400, { "Content-Type": "application/json" });
        res.end(JSON.stringify({
          jsonrpc: "2.0",
          error: { code: -32000, message: "Session not found; send an initialize request first" },
          id: null,
        }));
        return;
      }

      // Existing session — GET (SSE stream) / POST (message) / DELETE (close)
      if (req.method === "POST") {
        const body = await readJsonBody(req);
        await transport.handleRequest(req, res, body);
      } else {
        await transport.handleRequest(req, res);
      }
    } catch (err) {
      console.error("HTTP handler error:", err);
      if (!res.headersSent) {
        res.writeHead(500, { "Content-Type": "application/json" });
        res.end(JSON.stringify({
          jsonrpc: "2.0",
          error: { code: -32603, message: "Internal error" },
          id: null,
        }));
      }
    }
  });

  httpServer.listen(port, () => {
    console.error(`floatty-explorer MCP (http) listening on http://localhost:${port}/mcp — ${FLOATTY_URL}`);
  });
}

async function main() {
  const args = process.argv.slice(2);
  const httpFlag = args.includes("--http");
  if (httpFlag) {
    const portIdx = args.indexOf("--port");
    const port = portIdx >= 0 && args[portIdx + 1]
      ? Number(args[portIdx + 1])
      : Number(process.env.PORT ?? 5051);
    await runHttp(port);
  } else {
    await runStdio();
  }
}

main().catch((err) => { console.error("Fatal:", err); process.exit(1); });
