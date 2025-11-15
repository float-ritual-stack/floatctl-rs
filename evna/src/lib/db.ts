/**
 * PostgreSQL/pgvector database client + AutoRAG integration
 * Provides semantic search over conversation history via Cloudflare AutoRAG
 */

import { createClient } from '@supabase/supabase-js';
import type { SupabaseClient } from '@supabase/supabase-js';
import type Anthropic from '@anthropic-ai/sdk';
import { AutoRAGClient } from './autorag-client.js';
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
  private autorag: AutoRAGClient | null = null;

  constructor(supabaseUrl: string, supabaseKey: string) {
    this.supabase = createClient(supabaseUrl, supabaseKey);

    // Initialize AutoRAG client lazily (only needed for semantic search)
    const accountId = process.env.CLOUDFLARE_ACCOUNT_ID;
    const apiToken = process.env.AUTORAG_API_TOKEN;

    if (accountId && apiToken) {
      this.autorag = new AutoRAGClient(accountId, apiToken);
    }
  }

  private ensureAutoRAG(): AutoRAGClient {
    if (!this.autorag) {
      throw new Error(
        'AutoRAG not initialized. Set CLOUDFLARE_ACCOUNT_ID and AUTORAG_API_TOKEN environment variables.'
      );
    }
    return this.autorag;
  }

  /**
   * Semantic search via Cloudflare AutoRAG
   * Replaced pgvector embeddings (vestigial, Nov 15 2025)
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
    const { limit = 10, project, since, threshold = 0.5 } = options;

    try {
      // Call AutoRAG search (historical knowledge from R2-synced content)
      const autorag = this.ensureAutoRAG();
      const results = await autorag.search({
        query: queryText,
        max_results: limit,
        score_threshold: threshold,
        folder_filter: project ? `${project}/` : undefined,
      });

      // Transform AutoRAG results to SearchResult format
      return results.map((result) => ({
        message: {
          id: result.file_id,
          conversation_id: result.file_id,
          idx: 0,
          role: 'assistant', // AutoRAG results are curated content
          timestamp: result.attributes.modified_date
            ? new Date(result.attributes.modified_date * 1000).toISOString()
            : new Date().toISOString(),
          content: result.content.map(c => c.text).join('\n\n'),
          project: result.attributes.folder || null,
          meeting: null,
          markers: [],
        },
        conversation: {
          id: result.file_id,
          conv_id: result.file_id,
          title: result.filename,
          created_at: result.attributes.modified_date
            ? new Date(result.attributes.modified_date * 1000).toISOString()
            : new Date().toISOString(),
          markers: [],
        },
        similarity: result.score,
        source: 'embeddings', // Mark as historical for brain_boot compatibility
      }));
    } catch (error) {
      console.error('[db] AutoRAG search failed:', {
        queryText,
        limit,
        project,
        since,
        threshold,
        error: error instanceof Error ? error.message : String(error),
      });
      throw new Error(`AutoRAG search failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  /**
   * Semantic search via Cloudflare AutoRAG - curated notes and bridges
   * Replaced note_embeddings table (vestigial, Nov 15 2025)
   */
  async semanticSearchNotes(
    queryText: string,
    options: {
      limit?: number;
      noteType?: string;
      threshold?: number;
    } = {}
  ): Promise<SearchResult[]> {
    const { limit = 10, noteType, threshold = 0.5 } = options;

    try {
      // Call AutoRAG search for curated notes (bridges, daily notes, etc)
      const autorag = this.ensureAutoRAG();
      const results = await autorag.search({
        query: queryText,
        max_results: limit,
        score_threshold: threshold,
        folder_filter: noteType ? `${noteType}/` : 'bridges/', // Default to bridges folder
      });

      // Transform AutoRAG results to SearchResult format
      return results.map((result) => ({
        message: {
          id: result.file_id,
          conversation_id: result.file_id,
          idx: 0,
          role: 'assistant', // Curated note content
          timestamp: result.attributes.modified_date
            ? new Date(result.attributes.modified_date * 1000).toISOString()
            : new Date().toISOString(),
          content: result.content.map(c => c.text).join('\n\n'),
          project: result.attributes.folder || null,
          meeting: null,
          markers: [],
        },
        conversation: {
          id: result.file_id,
          conv_id: result.file_id,
          title: result.filename,
          created_at: result.attributes.modified_date
            ? new Date(result.attributes.modified_date * 1000).toISOString()
            : new Date().toISOString(),
          markers: [],
        },
        similarity: result.score,
        source: 'embeddings', // Mark as historical for brain_boot compatibility
      }));
    } catch (error) {
      console.error('[db] AutoRAG note search failed:', {
        queryText,
        limit,
        noteType,
        threshold,
        error: error instanceof Error ? error.message : String(error),
      });
      throw new Error(`AutoRAG note search failed: ${error instanceof Error ? error.message : String(error)}`);
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

      // TODO: Step 5 should call floatctl-cli to embed the message
      // This maintains separation: floatctl handles embedding, evna orchestrates
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

  /**
   * Get ask_evna session by ID
   * Returns messages array for conversation resumption
   */
  async getAskEvnaSession(sessionId: string): Promise<{ messages: Anthropic.MessageParam[] } | null> {
    const { data, error } = await this.supabase
      .from('ask_evna_sessions')
      .select('messages')
      .eq('session_id', sessionId)
      .single();

    if (error || !data) {
      return null;
    }

    return { messages: data.messages };
  }

  /**
   * Save/update ask_evna session
   * Uses upsert to handle both create and update
   */
  async saveAskEvnaSession(sessionId: string, messages: Anthropic.MessageParam[]): Promise<void> {
    const { error } = await this.supabase
      .from('ask_evna_sessions')
      .upsert({
        session_id: sessionId,
        messages,
        last_used: new Date().toISOString()
      });

    if (error) {
      console.error('[db] Failed to save ask_evna session:', {
        session_id: sessionId,
        error: error.message
      });
      throw new Error(`Failed to save ask_evna session: ${error.message}`);
    }
  }
}
