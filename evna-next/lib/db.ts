import { createClient, SupabaseClient } from "@supabase/supabase-js";
import { Pool } from "pg";

let supabaseClient: SupabaseClient | null = null;
let pgPool: Pool | null = null;

function getSupabaseClient() {
  if (!supabaseClient) {
    supabaseClient = createClient(
      process.env.SUPABASE_URL || "",
      process.env.SUPABASE_SERVICE_KEY || ""
    );
  }
  return supabaseClient;
}

function getPool() {
  if (!pgPool) {
    pgPool = new Pool({
      connectionString: process.env.DATABASE_URL,
    });
  }
  return pgPool;
}

// Export functions to get clients (lazy initialization)
export function getSupabase() {
  return getSupabaseClient();
}

export function getDBPool() {
  return getPool();
}

// Semantic search function
export interface SemanticSearchResult {
  conversation_id: string;
  message_id: string;
  content: string;
  similarity: number;
  timestamp: string;
  project?: string;
  meeting?: string;
  mode?: string;
  source: "embeddings" | "active_context";
}

export async function semanticSearch({
  query,
  embedding,
  limit = 10,
  threshold = 0.5,
  project,
  since,
}: {
  query: string;
  embedding: number[];
  limit?: number;
  threshold?: number;
  project?: string;
  since?: string;
}): Promise<SemanticSearchResult[]> {
  const client = await getPool().connect();
  try {
    let sql = `
      SELECT 
        e.conversation_id,
        e.message_id,
        m.content,
        1 - (e.embedding <=> $1::vector) as similarity,
        m.timestamp,
        m.project,
        m.meeting,
        m.mode,
        'embeddings' as source
      FROM embeddings e
      JOIN messages m ON e.message_id = m.id
      WHERE 1 - (e.embedding <=> $1::vector) > $2
    `;
    
    const params: any[] = [JSON.stringify(embedding), threshold];
    let paramIndex = 3;

    if (project) {
      sql += ` AND m.project = $${paramIndex}`;
      params.push(project);
      paramIndex++;
    }

    if (since) {
      sql += ` AND m.timestamp > $${paramIndex}`;
      params.push(since);
      paramIndex++;
    }

    sql += ` ORDER BY similarity DESC LIMIT $${paramIndex}`;
    params.push(limit);

    const result = await client.query(sql, params);
    return result.rows;
  } finally {
    client.release();
  }
}

// Active context search
export async function getActiveContext({
  query,
  limit = 10,
  project,
}: {
  query?: string;
  limit?: number;
  project?: string;
}): Promise<any[]> {
  let sql = `
    SELECT 
      conversation_id,
      content,
      timestamp,
      project,
      meeting,
      mode,
      client_type,
      'active_context' as source
    FROM active_context_stream
    WHERE 1=1
  `;

  const params: any[] = [];
  let paramIndex = 1;

  if (project) {
    sql += ` AND project = $${paramIndex}`;
    params.push(project);
    paramIndex++;
  }

  if (query) {
    sql += ` AND content ILIKE $${paramIndex}`;
    params.push(`%${query}%`);
    paramIndex++;
  }

  sql += ` ORDER BY timestamp DESC LIMIT $${paramIndex}`;
  params.push(limit);

  const client = await getPool().connect();
  try {
    const result = await client.query(sql, params);
    return result.rows;
  } finally {
    client.release();
  }
}
