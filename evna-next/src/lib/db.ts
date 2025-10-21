/**
 * PostgreSQL/pgvector database client
 * Provides semantic search over conversation history
 */

import { createClient } from '@supabase/supabase-js';
import type { SupabaseClient } from '@supabase/supabase-js';

export interface Message {
  id: string;
  conversation_id: string;
  idx: number;
  role: string;
  timestamp: string;
  content: string;
  project?: string | null;
  meeting?: string | null;
  markers: string[];
}

export interface Conversation {
  id: string;
  conv_id: string;
  title?: string | null;
  created_at: string;
  markers: string[];
}

export interface SearchResult {
  message: Message;
  conversation?: Conversation;
  similarity: number;
}

export class DatabaseClient {
  private supabase: SupabaseClient;

  constructor(supabaseUrl: string, supabaseKey: string) {
    this.supabase = createClient(supabaseUrl, supabaseKey);
  }

  /**
   * Semantic search via Rust CLI with JSON output
   * Delegates to floatctl-cli which has correct filter implementation
   */
  async semanticSearch(
    queryText: string,
    options: {
      limit?: number;
      project?: string;
      since?: string;
      threshold?: number;
    } = {}
  ): Promise<SearchResult[]> {
    const { limit = 10, project, since, threshold } = options;
    const { exec } = await import('child_process');
    const { promisify } = await import('util');
    const execAsync = promisify(exec);

    // Calculate days from since timestamp
    let days: number | undefined;
    if (since) {
      const sinceDate = new Date(since);
      const now = new Date();
      days = Math.ceil((now.getTime() - sinceDate.getTime()) / (1000 * 60 * 60 * 24));
    }

    // Build Rust CLI command
    let cmd = `cargo run --release -p floatctl-cli -- query "${queryText.replace(/"/g, '\\"')}" --json --limit ${limit}`;
    if (project) {
      cmd += ` --project "${project.replace(/"/g, '\\"')}"`;
    }
    if (days !== undefined) {
      cmd += ` --days ${days}`;
    }
    if (threshold !== undefined) {
      cmd += ` --threshold ${threshold}`;
    }

    try {
      // Note: No console.log here - MCP uses stdout for JSON-RPC
      const { stdout, stderr } = await execAsync(cmd, {
        cwd: '../', // Run from floatctl-rs root
        maxBuffer: 10 * 1024 * 1024, // 10MB buffer
        env: {
          ...process.env,
          RUST_LOG: 'off', // Disable logging to prevent pollution of JSON output
        },
      });

      // Parse JSON output from Rust CLI
      const rows = JSON.parse(stdout) as Array<{
        content: string;
        role: string;
        project?: string;
        meeting?: string;
        timestamp: string;
        markers: string[];
        conversation_title?: string;
        conv_id: string;
        similarity: number;
      }>;

      // Transform to SearchResult format
      return rows.map((row) => ({
        message: {
          id: '', // Not provided by Rust CLI
          conversation_id: row.conv_id,
          idx: 0, // Not provided by Rust CLI
          role: row.role,
          timestamp: row.timestamp,
          content: row.content,
          project: row.project || null,
          meeting: row.meeting || null,
          markers: row.markers,
        },
        conversation: {
          id: row.conv_id,
          conv_id: row.conv_id,
          title: row.conversation_title || null,
          created_at: row.timestamp, // Approximation
          markers: row.markers,
        },
        similarity: row.similarity,
      }));
    } catch (error) {
      // Note: No console.error here - MCP uses stderr for logs
      throw new Error(`Rust CLI search failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  /**
   * Get recent messages for a project
   */
  async getRecentMessages(
    options: {
      limit?: number;
      project?: string;
      since?: string;
    } = {}
  ): Promise<Message[]> {
    const { limit = 20, project, since } = options;

    let query = this.supabase
      .from('messages')
      .select('*')
      .order('timestamp', { ascending: false })
      .limit(limit);

    if (project) {
      query = query.eq('project', project);
    }
    if (since) {
      query = query.gte('timestamp', since);
    }

    const { data, error } = await query;

    if (error) {
      throw new Error(`Failed to fetch recent messages: ${error.message}`);
    }

    return data || [];
  }

  /**
   * Store message to active_context_stream table
   */
  async storeActiveContext(message: {
    message_id: string;
    conversation_id: string;
    role: string;
    content: string;
    timestamp: Date;
    client_type?: 'desktop' | 'claude_code';
    metadata: Record<string, any>;
  }): Promise<void> {
    const { error } = await this.supabase
      .from('active_context_stream')
      .insert({
        message_id: message.message_id,
        conversation_id: message.conversation_id,
        role: message.role,
        content: message.content,
        timestamp: message.timestamp.toISOString(),
        client_type: message.client_type,
        metadata: message.metadata,
      });

    if (error) {
      throw new Error(`Failed to store active context: ${error.message}`);
    }
  }

  /**
   * Query active_context_stream table
   */
  async queryActiveContext(options: {
    limit?: number;
    project?: string;
    since?: Date;
    client_type?: 'desktop' | 'claude_code';
    mode?: string;
  } = {}): Promise<Array<{
    message_id: string;
    conversation_id: string;
    role: string;
    content: string;
    timestamp: string;
    client_type?: string;
    metadata: Record<string, any>;
  }>> {
    const { limit = 10, project, since, client_type, mode } = options;

    let query = this.supabase
      .from('active_context_stream')
      .select('*')
      .order('timestamp', { ascending: false })
      .limit(limit);

    if (project) {
      query = query.eq('metadata->>project', project);
    }
    if (since) {
      query = query.gte('timestamp', since.toISOString());
    }
    if (client_type) {
      query = query.eq('client_type', client_type);
    }
    if (mode) {
      query = query.eq('metadata->ctx->>mode', mode);
    }

    const { data, error } = await query;

    if (error) {
      throw new Error(`Failed to query active context: ${error.message}`);
    }

    return data || [];
  }

  /**
   * Get conversation by ID
   */
  async getConversation(convId: string): Promise<Conversation | null> {
    const { data, error } = await this.supabase
      .from('conversations')
      .select('*')
      .eq('conv_id', convId)
      .single();

    if (error) {
      if (error.code === 'PGRST116') return null; // Not found
      throw new Error(`Failed to fetch conversation: ${error.message}`);
    }

    return data;
  }

  /**
   * Get messages for a conversation
   */
  async getConversationMessages(conversationId: string): Promise<Message[]> {
    const { data, error } = await this.supabase
      .from('messages')
      .select('*')
      .eq('conversation_id', conversationId)
      .order('idx', { ascending: true });

    if (error) {
      throw new Error(`Failed to fetch conversation messages: ${error.message}`);
    }

    return data || [];
  }
}
