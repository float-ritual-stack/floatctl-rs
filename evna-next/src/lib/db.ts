/**
 * PostgreSQL/pgvector database client
 * Provides semantic search over conversation history
 */

import { createClient } from '@supabase/supabase-js';
import type { SupabaseClient } from '@supabase/supabase-js';
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
 * Expand project name to include all known aliases
 * Philosophy: "LLMs as fuzzy compilers" - match generously
 */
function expandProjectAliases(project: string): string[] {
  const lowerProject = project.toLowerCase();

  // Find matching canonical or alias
  for (const [key, config] of Object.entries(workspace.projects)) {
    const allVariants = [config.canonical, ...config.aliases].map(v => v.toLowerCase());
    if (allVariants.some(v => v.includes(lowerProject) || lowerProject.includes(v))) {
      return [config.canonical, ...config.aliases];
    }
  }

  // No match in config - return original (fuzzy match with ILIKE)
  return [project];
}

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
  source?: 'active_context' | 'embeddings'; // Source of the result (for brain_boot detection)
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
        source: 'embeddings', // Mark as embeddings for brain_boot
      }));
    } catch (error) {
      console.error('[db] Rust CLI search failed:', {
        queryText,
        limit,
        project,
        since,
        threshold,
        error: error instanceof Error ? error.message : String(error),
      });
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
      console.error('[db] Failed to fetch recent messages:', {
        limit,
        project,
        since,
        error: error.message,
      });
      throw new Error(`Failed to fetch recent messages: ${error.message}`);
    }

    return data || [];
  }

  /**
   * Store message to active_context_stream table WITH double-write to permanent storage
   *
   * Double-write pattern:
   * 1. Write to hot cache (active_context_stream) with 36hr TTL
   * 2. Write to permanent storage (conversations + messages)
   * 3. Link them together for organic corpus growth
   *
   * Result: Discussions about gaps fill those gaps in searchable history
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
    // Step 1: Write to hot cache (active_context_stream)
    const { error: insertError } = await this.supabase
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

    if (insertError) {
      console.error('[db] Failed to store active context:', {
        message_id: message.message_id,
        conversation_id: message.conversation_id,
        error: insertError.message,
      });
      throw new Error(`Failed to store active context: ${insertError.message}`);
    }

    try {
      // Step 2: Get or create conversation in permanent storage
      const conversation = await this.getOrCreateConversation(
        message.conversation_id,
        {
          title: message.metadata.conversation_title,
          markers: message.metadata.markers,
        }
      );

      // Step 3: Create message in permanent storage
      const persistedMessage = await this.createMessage({
        conversation_id: conversation.id,
        role: message.role,
        content: message.content,
        timestamp: message.timestamp,
        project: message.metadata.project,
        meeting: message.metadata.meeting,
        markers: message.metadata.markers || [],
      });

      // Step 4: Link hot cache record to permanent storage
      const { error: updateError } = await this.supabase
        .from('active_context_stream')
        .update({
          persisted_to_long_term: true,
          persisted_message_id: persistedMessage.id,
        })
        .eq('message_id', message.message_id);

      if (updateError) {
        console.error('[db] Failed to update double-write linkage:', {
          message_id: message.message_id,
          persisted_message_id: persistedMessage.id,
          error: updateError.message,
        });
        // Don't throw - hot cache write succeeded, permanent write succeeded
        // Just log the linkage failure
      }
    } catch (permanentStorageError) {
      // Log but don't fail - hot cache write already succeeded
      console.error('[db] Failed to double-write to permanent storage:', {
        message_id: message.message_id,
        conversation_id: message.conversation_id,
        error: permanentStorageError instanceof Error
          ? permanentStorageError.message
          : String(permanentStorageError),
      });
      // Continue - hot cache is primary, permanent storage is best-effort
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
      // Fuzzy match: expand to all known aliases
      const variants = expandProjectAliases(project);

      // Build OR condition for fuzzy matching
      const orConditions = variants.map(v => `metadata->>project.ilike.%${v}%`).join(',');
      query = query.or(orConditions);
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
      console.error('[db] Failed to query active context:', {
        options,
        error: error.message,
      });
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
      console.error('[db] Failed to fetch conversation:', {
        convId,
        error: error.message,
        code: error.code,
      });
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
      console.error('[db] Failed to fetch conversation messages:', {
        conversationId,
        error: error.message,
      });
      throw new Error(`Failed to fetch conversation messages: ${error.message}`);
    }

    return data || [];
  }

  /**
   * Get or create conversation by conv_id
   * Part of double-write pattern: ensures conversation exists before creating message
   */
  async getOrCreateConversation(convId: string, metadata?: {
    title?: string;
    markers?: string[];
  }): Promise<Conversation> {
    // Try to get existing conversation
    const existing = await this.getConversation(convId);
    if (existing) {
      return existing;
    }

    // Create new conversation
    const { data, error } = await this.supabase
      .from('conversations')
      .insert({
        conv_id: convId,
        title: metadata?.title || null,
        markers: metadata?.markers || [],
        created_at: new Date().toISOString(),
      })
      .select()
      .single();

    if (error) {
      console.error('[db] Failed to create conversation:', {
        convId,
        error: error.message,
      });
      throw new Error(`Failed to create conversation: ${error.message}`);
    }

    return data;
  }

  /**
   * Create message in permanent storage
   * Part of double-write pattern: persists active_context to long-term storage
   */
  async createMessage(message: {
    conversation_id: string; // UUID from conversations.id
    role: string;
    content: string;
    timestamp: Date;
    project?: string;
    meeting?: string;
    markers?: string[];
    idx?: number;
  }): Promise<Message> {
    // Generate UUID for message (no default in schema)
    const { randomUUID } = await import('crypto');
    const messageId = randomUUID();

    const { data, error } = await this.supabase
      .from('messages')
      .insert({
        id: messageId,
        conversation_id: message.conversation_id,
        idx: message.idx ?? 0,
        role: message.role,
        timestamp: message.timestamp.toISOString(),
        content: message.content,
        project: message.project || null,
        meeting: message.meeting || null,
        markers: message.markers || [],
      })
      .select()
      .single();

    if (error) {
      console.error('[db] Failed to create message:', {
        conversation_id: message.conversation_id,
        error: error.message,
      });
      throw new Error(`Failed to create message: ${error.message}`);
    }

    return data;
  }
}
