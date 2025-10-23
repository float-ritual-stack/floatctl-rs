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
 */

import 'dotenv/config';
import { DatabaseClient } from '../src/lib/db.js';

const SUPABASE_URL = process.env.SUPABASE_URL;
const SUPABASE_ANON_KEY = process.env.SUPABASE_ANON_KEY;

if (!SUPABASE_URL || !SUPABASE_ANON_KEY) {
  console.error('❌ Missing SUPABASE_URL or SUPABASE_ANON_KEY environment variables');
  console.error('Make sure .env file exists with these values');
  process.exit(1);
}

const db = new DatabaseClient(SUPABASE_URL, SUPABASE_ANON_KEY);

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

    default:
      console.log('Usage:');
      console.log('  tsx scripts/test-db-methods.ts query-active-context [--project <name>] [--limit <n>] [--since <ISO date>]');
      console.log('  tsx scripts/test-db-methods.ts recent-messages [--project <name>] [--limit <n>] [--since <ISO date>]');
      console.log('');
      console.log('Examples:');
      console.log('  tsx scripts/test-db-methods.ts query-active-context --project "rangle/pharmacy" --limit 5');
      console.log('  tsx scripts/test-db-methods.ts recent-messages --limit 10');
      process.exit(1);
  }
})();
