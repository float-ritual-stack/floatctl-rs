#!/usr/bin/env tsx
/**
 * Test harness for database methods
 *
 * Usage:
 *   # Test queryActiveContext with project filter
 *   tsx scripts/test-db-methods.ts query-active-context --project "rangle/pharmacy" --limit 5
 *
 *   # Test queryActiveContext without filter
 *   tsx scripts/test-db-methods.ts query-active-context --limit 10
 *
 *   # Test getRecentMessages
 *   tsx scripts/test-db-methods.ts recent-messages --project "pharmacy" --limit 5
 *
 *   # Test semantic search (dual-source: active_context + embeddings)
 *   tsx scripts/test-db-methods.ts semantic-search --query "pharmacy project filter" --limit 5 --threshold 0.5
 */

import 'dotenv/config';
import { DatabaseClient } from '../src/lib/db.js';
import { EmbeddingsClient } from '../src/lib/embeddings.js';
import { PgVectorSearchTool } from '../src/tools/pgvector-search.js';

const SUPABASE_URL = process.env.SUPABASE_URL;
const SUPABASE_ANON_KEY = process.env.SUPABASE_ANON_KEY;
const OPENAI_API_KEY = process.env.OPENAI_API_KEY;

if (!SUPABASE_URL || !SUPABASE_ANON_KEY) {
  console.error('❌ Missing SUPABASE_URL or SUPABASE_ANON_KEY environment variables');
  console.error('Make sure .env file exists with these values');
  process.exit(1);
}

const db = new DatabaseClient(SUPABASE_URL, SUPABASE_ANON_KEY);
const embeddings = OPENAI_API_KEY ? new EmbeddingsClient(OPENAI_API_KEY) : null;

async function testQueryActiveContext(args: {
  project?: string;
  limit?: number;
  since?: string;
  clientType?: 'desktop' | 'claude_code';
}) {
  console.log('🧪 Testing queryActiveContext');
  console.log('📋 Parameters:', JSON.stringify(args, null, 2));
  console.log('');

  try {
    const results = await db.queryActiveContext({
      limit: args.limit || 10,
      project: args.project,
      since: args.since ? new Date(args.since) : undefined,
      client_type: args.clientType,
    });

    console.log(`✅ Success: ${results.length} results`);
    console.log('');

    results.forEach((result, idx) => {
      console.log(`--- Result ${idx + 1} ---`);
      console.log(`Message ID: ${result.message_id}`);
      console.log(`Timestamp: ${result.timestamp}`);
      console.log(`Role: ${result.role}`);
      console.log(`Client: ${result.client_type || 'unknown'}`);
      console.log(`Project: ${result.metadata?.project || 'none'}`);
      console.log(`Metadata: ${JSON.stringify(result.metadata, null, 2)}`);
      console.log(`Content preview: ${result.content.substring(0, 100)}...`);
      console.log('');
    });
  } catch (error) {
    console.error('❌ Error:', error instanceof Error ? error.message : String(error));
    if (error instanceof Error && error.stack) {
      console.error(error.stack);
    }
    process.exit(1);
  }
}

async function testGetRecentMessages(args: {
  project?: string;
  limit?: number;
  since?: string;
}) {
  console.log('🧪 Testing getRecentMessages');
  console.log('📋 Parameters:', JSON.stringify(args, null, 2));
  console.log('');

  try {
    const results = await db.getRecentMessages({
      limit: args.limit || 20,
      project: args.project,
      since: args.since,
    });

    console.log(`✅ Success: ${results.length} results`);
    console.log('');

    results.forEach((result, idx) => {
      console.log(`--- Result ${idx + 1} ---`);
      console.log(`Message ID: ${result.message_id}`);
      console.log(`Timestamp: ${result.timestamp}`);
      console.log(`Project: ${result.project || 'none'}`);
      console.log(`Content preview: ${result.content.substring(0, 100)}...`);
      console.log('');
    });
  } catch (error) {
    console.error('❌ Error:', error instanceof Error ? error.message : String(error));
    if (error instanceof Error && error.stack) {
      console.error(error.stack);
    }
    process.exit(1);
  }
}

async function testSemanticSearch(args: {
  query: string;
  project?: string;
  limit?: number;
  threshold?: number;
  since?: string;
}) {
  console.log('🧪 Testing Semantic Search (Dual-Source)');
  console.log('📋 Parameters:', JSON.stringify(args, null, 2));
  console.log('');

  if (!embeddings) {
    console.error('❌ OPENAI_API_KEY not set - cannot initialize EmbeddingsClient');
    process.exit(1);
  }

  try {
    const searchTool = new PgVectorSearchTool(db, embeddings);

    const results = await searchTool.search({
      query: args.query,
      limit: args.limit || 10,
      project: args.project,
      threshold: args.threshold || 0.5,
      since: args.since,
    });

    console.log(`✅ Success: ${results.length} results`);
    console.log('');

    // Group by source for clarity
    const activeResults = results.filter(r => r.similarity === 1.0);
    const embeddingResults = results.filter(r => r.similarity < 1.0);

    if (activeResults.length > 0) {
      console.log('🔴 ACTIVE CONTEXT RESULTS (Recent, last 7 days):');
      activeResults.forEach((result, idx) => {
        console.log(`--- Active Result ${idx + 1} ---`);
        console.log(`Timestamp: ${result.message.timestamp}`);
        console.log(`Project: ${result.message.project || 'none'}`);
        console.log(`Similarity: ${result.similarity.toFixed(2)} (priority)`);
        console.log(`Content preview: ${result.message.content.substring(0, 150)}...`);
        console.log('');
      });
    }

    if (embeddingResults.length > 0) {
      console.log('🗄️  EMBEDDINGS RESULTS (Historical, archived):');
      embeddingResults.forEach((result, idx) => {
        console.log(`--- Embedding Result ${idx + 1} ---`);
        console.log(`Timestamp: ${result.message.timestamp}`);
        console.log(`Conversation: ${result.conversation?.title || result.conversation?.conv_id || 'Unknown'}`);
        console.log(`Project: ${result.message.project || 'none'}`);
        console.log(`Similarity: ${result.similarity.toFixed(2)}`);
        console.log(`Content preview: ${result.message.content.substring(0, 150)}...`);
        console.log('');
      });
    }

    if (results.length === 0) {
      console.log('⚠️  No results found in either active_context_stream or embeddings');
    }

  } catch (error) {
    console.error('❌ Error:', error instanceof Error ? error.message : String(error));
    if (error instanceof Error && error.stack) {
      console.error(error.stack);
    }
    process.exit(1);
  }
}

// Parse CLI args
const command = process.argv[2];
const args: Record<string, string> = {};

for (let i = 3; i < process.argv.length; i++) {
  const arg = process.argv[i];
  if (arg.startsWith('--')) {
    const key = arg.slice(2);
    const value = process.argv[i + 1];
    if (value && !value.startsWith('--')) {
      args[key] = value;
      i++; // Skip the value in next iteration
    }
  }
}

// Run the appropriate test
(async () => {
  switch (command) {
    case 'query-active-context':
      await testQueryActiveContext({
        project: args.project,
        limit: args.limit ? parseInt(args.limit) : undefined,
        since: args.since,
        clientType: args.clientType as 'desktop' | 'claude_code' | undefined,
      });
      break;

    case 'recent-messages':
      await testGetRecentMessages({
        project: args.project,
        limit: args.limit ? parseInt(args.limit) : undefined,
        since: args.since,
      });
      break;

    case 'semantic-search':
      if (!args.query) {
        console.error('❌ --query parameter is required for semantic-search');
        process.exit(1);
      }
      await testSemanticSearch({
        query: args.query,
        project: args.project,
        limit: args.limit ? parseInt(args.limit) : undefined,
        threshold: args.threshold ? parseFloat(args.threshold) : undefined,
        since: args.since,
      });
      break;

    default:
      console.log('Usage:');
      console.log('  tsx scripts/test-db-methods.ts query-active-context [--project <name>] [--limit <n>] [--since <ISO date>]');
      console.log('  tsx scripts/test-db-methods.ts recent-messages [--project <name>] [--limit <n>] [--since <ISO date>]');
      console.log('  tsx scripts/test-db-methods.ts semantic-search --query <text> [--project <name>] [--limit <n>] [--threshold <0-1>] [--since <ISO date>]');
      console.log('');
      console.log('Examples:');
      console.log('  tsx scripts/test-db-methods.ts query-active-context --project "rangle/pharmacy" --limit 5');
      console.log('  tsx scripts/test-db-methods.ts recent-messages --limit 10');
      console.log('  tsx scripts/test-db-methods.ts semantic-search --query "pharmacy project filter" --limit 5 --threshold 0.5');
      process.exit(1);
  }
})();
