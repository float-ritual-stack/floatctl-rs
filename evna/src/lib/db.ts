/**
 * PostgreSQL/pgvector database client + AutoRAG integration
 * Provides semantic search over conversation history via Cloudflare AutoRAG
 */

import { createClient } from '@supabase/supabase-js';
import type { SupabaseClient } from '@supabase/supabase-js';
import type Anthropic from '@anthropic-ai/sdk';
import { AutoRAGClient, type AutoRAGResult } from './autorag-client.js';
import { expandProjectAliases } from './project-utils.js';
import { logger } from './logger.js';

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
   * Retry wrapper for transient AutoRAG failures
   * Handles Workers AI internal server errors with exponential backoff
   */
  private async retryAutoRAG<T>(
    operation: () => Promise<T>,
    maxRetries = 3
  ): Promise<T> {
    let lastError: Error | undefined;

    for (let attempt = 0; attempt < maxRetries; attempt++) {
      try {
        return await operation();
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);

        // Detect transient errors that are worth retrying
        const isTransient =
          errorMessage.includes('Internal server error') ||
          errorMessage.includes('code":7019') ||
          errorMessage.includes('503') ||
          errorMessage.includes('500');

        if (!isTransient || attempt === maxRetries - 1) {
          // Non-transient error or final attempt - rethrow
          throw error;
        }

        // Log retry attempt
        const backoffMs = Math.pow(2, attempt) * 1000; // 1s, 2s, 4s
        logger.error('db', `AutoRAG transient error (attempt ${attempt + 1}/${maxRetries}), retrying in ${backoffMs}ms...`, {
          error: errorMessage,
          attempt: attempt + 1,
          maxRetries,
          backoffMs,
        });

        // Wait before retry with exponential backoff
        await new Promise(resolve => setTimeout(resolve, backoffMs));
        lastError = error instanceof Error ? error : new Error(errorMessage);
      }
    }

    // Should never reach here due to throw in loop, but TypeScript needs this
    throw lastError || new Error('AutoRAG operation failed after retries');
  }

  /**
   * Semantic search via Cloudflare AutoRAG with structural filtering
   *
   * Filter behavior (Nov 22, 2025 - CORRECTED):
   * - When project specified: search dispatch/ only (exclude personal daily notes)
   * - When no project: search all folders (dispatch/ + daily/)
   * - Trust AutoRAG semantic matching for project relevance (query rewriting + BGE reranker)
   *
   * Why structural filtering (not project-based folder filtering):
   * - Project is YAML frontmatter metadata (`project: floatctl-rs`), NOT a folder path
   * - R2 structure: dispatch/ subfolder hierarchy (bridges/, imprints/, docs/, operations/)
   *   with YAML frontmatter project metadata. Broad filter preferred - project names drift
   *   over time (rangle/pharmacy vs pharmacy, floatctl vs floatctl-rs). Trust AutoRAG
   *   semantic matching (query rewriting + BGE reranker) for project relevance.
   * - daily/ contains personal time-indexed notes (excluded when project filter specified)
   * - See: sysops-log/2025-11-22-rotfield-recursion-evna-autorag-structural-filtering.md
   *
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
      const autorag = this.ensureAutoRAG();

      // Structural filtering: dispatch/ (work content) vs daily/ (personal notes)
      // When project specified, exclude personal daily notes
      // AutoRAG will find project-relevant content via semantic matching
      const folder_filter = project ? 'dispatch/' : undefined;

      const results = await this.retryAutoRAG(() =>
        autorag.search({
          query: queryText,
          max_results: limit,
          score_threshold: threshold,
          folder_filter,  // dispatch/ when project specified, undefined otherwise
        })
      );

      return this.transformAutoRAGResults(results);
    } catch (error) {
      logger.error('db', 'AutoRAG search failed', {
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
   * Transform AutoRAG search results to SearchResult format
   * Shared by semanticSearch and semanticSearchNotes
   */
  private transformAutoRAGResults(results: AutoRAGResult[]): SearchResult[] {
    return results.map((result) => ({
      message: {
        id: result.file_id,
        conversation_id: result.file_id,
        idx: 0,
        role: 'assistant', // AutoRAG results are curated content
        timestamp: result.attributes.modified_date
          ? new Date(result.attributes.modified_date * 1000).toISOString()
          : new Date().toISOString(),
        content: result.content.map((c) => c.text).join('\n\n'),
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
  }

  /**
   * Semantic search via Cloudflare AutoRAG - curated notes and bridges
   *
   * Filter behavior: Uses folder-based filtering for note type organization
   * - noteType parameter maps to specific folders (e.g., "bridges/", "imprints/")
   * - Defaults to "bridges/" for curated knowledge
   * - Different from semanticSearch: this targets organizational structure,
   *   not project-based semantic search with naming drift tolerance
   *
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
      // Call AutoRAG search with retry logic for transient failures
      const autorag = this.ensureAutoRAG();
      const results = await this.retryAutoRAG(() =>
        autorag.search({
          query: queryText,
          max_results: limit,
          score_threshold: threshold,
          folder_filter: noteType ? `${noteType}/` : 'bridges/', // Default to bridges folder
        })
      );

      // Transform AutoRAG results to SearchResult format
      return this.transformAutoRAGResults(results);
    } catch (error) {
      logger.error('db', 'AutoRAG note search failed', {
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
      logger.error('db', 'Failed to fetch recent messages', {
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
      logger.error('db', 'Failed to store active context', {
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
        logger.error('db', 'Failed to update double-write linkage', {
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
      logger.error('db', 'Failed to double-write to permanent storage', {
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
      // Split comma-separated projects and expand each
      const projects = project.split(',').map(p => p.trim()).filter(p => p.length > 0);
      const allVariants = projects.flatMap(p => expandProjectAliases(p));

      // Build OR condition for fuzzy matching
      const orConditions = allVariants.map(v => `metadata->>project.ilike.%${v}%`).join(',');
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
      logger.error('db', 'Failed to query active context', {
        options,
        error: error.message,
        code: error.code,
        details: error.details,
        hint: error.hint,
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
      logger.error('db', 'Failed to fetch conversation', {
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
      logger.error('db', 'Failed to fetch conversation messages', {
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
      logger.error('db', 'Failed to create conversation', {
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
      logger.error('db', 'Failed to create message', {
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
      logger.error('db', 'Failed to save ask_evna session', {
        session_id: sessionId,
        error: error.message
      });
      throw new Error(`Failed to save ask_evna session: ${error.message}`);
    }
  }
}
