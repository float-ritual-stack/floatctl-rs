/**
 * EVNA-Next MCP Server — native streamable-HTTP entry
 *
 * Replaces the supergateway stdio→HTTP bridge with the SDK's own
 * StreamableHTTPServerTransport (protocol 2025-11-25 native, proper session
 * negotiation — no proxy header-stripping required).
 *
 * Auth: evna is an OAuth RESOURCE SERVER only. WorkOS AuthKit is the
 * authorization server (DCR + GitHub login + token issuance). This process
 * validates Bearer JWTs against AuthKit's public JWKS and pins the token
 * subject to a single allowed user. claude.ai discovers the AS via the
 * /.well-known/oauth-protected-resource metadata this file serves.
 *
 * Env (via env-loader fallback chain: ./.env → ~/.floatctl/.env):
 *   EVNA_HTTP_PORT            listen port            (default 3106)
 *   EVNA_AUTH_MODE            "oauth" | "off"        (default oauth; "off" for tailnet-local testing ONLY)
 *   WORKOS_AUTHKIT_DOMAIN     e.g. https://serene-countryside-63-staging.authkit.app
 *   EVNA_RESOURCE_URL         e.g. https://evna.floatbbs.net/mcp (must match the
 *                             OAuth resource registered in WorkOS)
 *   EVNA_ALLOWED_SUBJECTS     comma list of allowed `sub` (user_…) and/or email
 *                             claims. UNSET = capture mode: every request is
 *                             rejected 403 and the presented sub is logged so it
 *                             can be pinned. Secure by default — never fails open.
 */

// Load .env with fallback chain: ./.env → ~/.floatctl/.env → existing env vars
import { loadEnvWithFallback } from "./lib/env-loader.js";
loadEnvWithFallback();
import { randomUUID } from "node:crypto";
import express from "express";
import type { Request, Response } from "express";
import { createRemoteJWKSet, jwtVerify, type JWTPayload } from "jose";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { isInitializeRequest } from "@modelcontextprotocol/sdk/types.js";
import { requireBearerAuth } from "@modelcontextprotocol/sdk/server/auth/middleware/bearerAuth.js";
import { InvalidTokenError } from "@modelcontextprotocol/sdk/server/auth/errors.js";
import type { OAuthTokenVerifier } from "@modelcontextprotocol/sdk/server/auth/provider.js";
import type { AuthInfo } from "@modelcontextprotocol/sdk/server/auth/types.js";
import { createEvnaServer } from "./mcp/server-factory.js";
import { startBridgeSyncTrigger } from "./lib/bridge-sync-trigger.js";

const PORT = Number(process.env.EVNA_HTTP_PORT ?? 3106);
const AUTH_MODE = process.env.EVNA_AUTH_MODE ?? "oauth";
const AUTHKIT_DOMAIN = (process.env.WORKOS_AUTHKIT_DOMAIN ?? "").replace(/\/$/, "");
const RESOURCE_URL = (process.env.EVNA_RESOURCE_URL ?? "").replace(/\/$/, "");
const ALLOWED_SUBJECTS = (process.env.EVNA_ALLOWED_SUBJECTS ?? "")
  .split(",")
  .map((s) => s.trim().toLowerCase())
  .filter(Boolean);

if (AUTH_MODE === "oauth" && (!AUTHKIT_DOMAIN || !RESOURCE_URL)) {
  console.error(
    "[evna-http] FATAL: EVNA_AUTH_MODE=oauth requires WORKOS_AUTHKIT_DOMAIN and EVNA_RESOURCE_URL"
  );
  process.exit(1);
}
if (AUTH_MODE === "off") {
  console.error(
    "[evna-http] WARNING: EVNA_AUTH_MODE=off — no authentication. Tailnet/local use only; never expose publicly."
  );
}

// ── WorkOS AuthKit token verification (resource-server side) ────────────────
const jwks = AUTHKIT_DOMAIN
  ? createRemoteJWKSet(new URL(`${AUTHKIT_DOMAIN}/oauth2/jwks`))
  : null;

const workosVerifier: OAuthTokenVerifier = {
  async verifyAccessToken(token: string): Promise<AuthInfo> {
    if (!jwks) throw new InvalidTokenError("JWKS not configured");
    // All verification failures MUST surface as InvalidTokenError — the SDK
    // middleware maps it to 401 + WWW-Authenticate, which is the signal
    // clients (Raycast, claude.ai) use to refresh an expired token. A plain
    // Error becomes a 500 and clients treat the server as broken
    // (discovered live: Raycast died at the 5-min token expiry, 2026-07-21).
    let payload: JWTPayload;
    try {
      ({ payload } = await jwtVerify(token, jwks, {
        issuer: AUTHKIT_DOMAIN,
        clockTolerance: 60,
      }));
    } catch (e) {
      const reason = e instanceof Error ? e.message : "Token verification failed";
      console.error(`[evna-http] token verify failed: ${reason}`);
      throw new InvalidTokenError(reason);
    }

    // Audience: when the token carries aud (resource-bound flows), it must
    // match the registered resource. Tokens without aud are rejected — the
    // WorkOS resource registration guarantees claude.ai's tokens carry it.
    const audiences = payload.aud
      ? Array.isArray(payload.aud) ? payload.aud : [payload.aud]
      : [];
    const audOk = audiences.some((a) => a.replace(/\/$/, "") === RESOURCE_URL);
    if (!audOk) {
      console.error(
        `[evna-http] token rejected: aud=${JSON.stringify(payload.aud)} does not match resource ${RESOURCE_URL}`
      );
      throw new InvalidTokenError("Token audience does not match this resource");
    }

    // Single-user pinning. Capture mode (no pins configured) rejects but logs
    // the identity so it can be copied into EVNA_ALLOWED_SUBJECTS.
    const sub = (payload.sub ?? "").toLowerCase();
    const email = String((payload as Record<string, unknown>).email ?? "").toLowerCase();
    if (ALLOWED_SUBJECTS.length === 0) {
      console.error(
        `[evna-http] CAPTURE MODE — rejecting valid token. Pin this identity: sub=${payload.sub} email=${email || "(none)"} → set EVNA_ALLOWED_SUBJECTS`
      );
      throw new InvalidTokenError("Server is in identity-capture mode; subject not yet pinned");
    }
    if (!ALLOWED_SUBJECTS.includes(sub) && !(email && ALLOWED_SUBJECTS.includes(email))) {
      console.error(`[evna-http] token rejected: sub=${payload.sub} email=${email || "(none)"} not in allowlist`);
      throw new InvalidTokenError("Subject not authorized for this resource");
    }

    return {
      token,
      clientId: String((payload as Record<string, unknown>).client_id ?? "unknown"),
      scopes: typeof payload.scope === "string" ? payload.scope.split(" ") : [],
      expiresAt: payload.exp,
      extra: { sub: payload.sub, email },
    };
  },
};

// ── HTTP app ────────────────────────────────────────────────────────────────
const app = express();
app.use(express.json({ limit: "4mb" }));

// Protected-resource metadata: how claude.ai discovers the authorization
// server. Served WITHOUT auth (clients fetch it in response to a 401).
// Both the bare and path-suffixed forms — clients probe both.
const resourceMetadata = (_req: Request, res: Response) => {
  res.json({
    resource: RESOURCE_URL,
    authorization_servers: [AUTHKIT_DOMAIN],
    bearer_methods_supported: ["header"],
  });
};
app.get("/.well-known/oauth-protected-resource", resourceMetadata);
app.get("/.well-known/oauth-protected-resource/mcp", resourceMetadata);

app.get("/healthz", (_req, res) => {
  res.json({ ok: true, name: "evna-next", transport: "streamable-http", auth: AUTH_MODE });
});

// Bearer auth on /mcp (skipped entirely in AUTH_MODE=off)
const resourceMetadataUrl = RESOURCE_URL
  ? `${new URL(RESOURCE_URL).origin}/.well-known/oauth-protected-resource`
  : undefined;
const authMiddleware =
  AUTH_MODE === "oauth"
    ? requireBearerAuth({ verifier: workosVerifier, resourceMetadataUrl })
    : (_req: Request, _res: Response, next: () => void) => next();

// ── Streamable-HTTP session plumbing (SDK canonical pattern) ────────────────
const transports = new Map<string, StreamableHTTPServerTransport>();

app.post("/mcp", authMiddleware, async (req: Request, res: Response) => {
  try {
    const sessionId = req.headers["mcp-session-id"] as string | undefined;
    // Request tracing — cheap, journal-only; the client UIs (Raycast) hide
    // which tool they called, so the server narrates.
    for (const msg of Array.isArray(req.body) ? req.body : [req.body]) {
      if (msg?.method) {
        const tool = msg.params?.name ? ` → ${msg.params.name}` : "";
        console.error(`[evna-http] ${msg.method}${tool} (session ${sessionId?.slice(0, 8) ?? "new"})`);
      }
    }

    if (sessionId && transports.has(sessionId)) {
      await transports.get(sessionId)!.handleRequest(req, res, req.body);
      return;
    }

    if (!sessionId && isInitializeRequest(req.body)) {
      const transport = new StreamableHTTPServerTransport({
        sessionIdGenerator: () => randomUUID(),
        onsessioninitialized: (sid) => {
          transports.set(sid, transport);
        },
      });
      transport.onclose = () => {
        if (transport.sessionId) transports.delete(transport.sessionId);
      };
      const server = createEvnaServer();
      await server.connect(transport);
      await transport.handleRequest(req, res, req.body);
      return;
    }

    // 404 (not 400): per spec a client receiving 404 for a session starts a
    // fresh one — this is what lets sessions survive server restarts.
    res.status(404).json({
      jsonrpc: "2.0",
      error: { code: -32001, message: "Session not found" },
      id: null,
    });
  } catch (error) {
    console.error("[evna-http] POST /mcp error:", error);
    if (!res.headersSent) {
      res.status(500).json({
        jsonrpc: "2.0",
        error: { code: -32603, message: "Internal server error" },
        id: null,
      });
    }
  }
});

// GET = server-initiated SSE stream; DELETE = session termination
const handleSessionRequest = async (req: Request, res: Response) => {
  try {
    const sessionId = req.headers["mcp-session-id"] as string | undefined;
    if (!sessionId || !transports.has(sessionId)) {
      res.status(404).send("Session not found");
      return;
    }
    await transports.get(sessionId)!.handleRequest(req, res);
  } catch (error) {
    console.error("[evna-http] session request error:", error);
    if (!res.headersSent) res.status(500).send("Internal server error");
  }
};
app.get("/mcp", authMiddleware, handleSessionRequest);
app.delete("/mcp", authMiddleware, handleSessionRequest);

app.listen(PORT, () => {
  console.error(
    `🧠 EVNA-Next MCP (streamable-http) on :${PORT} — auth=${AUTH_MODE}` +
      (AUTH_MODE === "oauth" ? ` as=${AUTHKIT_DOMAIN} resource=${RESOURCE_URL}` : "") +
      (ALLOWED_SUBJECTS.length === 0 && AUTH_MODE === "oauth" ? " [IDENTITY-CAPTURE MODE]" : "")
  );
  // Process-level side effect: file watcher → R2 sync (once per process, not per session)
  startBridgeSyncTrigger({
    enabled: process.env.EVNA_AUTO_SYNC !== "false",
    debounce_ms: 5000,
  });
});
