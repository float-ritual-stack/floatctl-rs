#!/usr/bin/env tsx
/**
 * Safe migration runner with preview and confirmation
 *
 * Usage:
 *   tsx scripts/run-migration.ts preview   # Preview changes
 *   tsx scripts/run-migration.ts execute   # Run the migration
 */

import 'dotenv/config';
import { createClient } from '@supabase/supabase-js';
import * as readline from 'readline';

const SUPABASE_URL = process.env.SUPABASE_URL!;
const SUPABASE_ANON_KEY = process.env.SUPABASE_ANON_KEY!;

if (!SUPABASE_URL || !SUPABASE_ANON_KEY) {
  console.error('❌ Missing SUPABASE_URL or SUPABASE_ANON_KEY');
  process.exit(1);
}

const supabase = createClient(SUPABASE_URL, SUPABASE_ANON_KEY);

async function preview() {
  console.log('🔍 PREVIEW: Records that will be updated\n');

  const { data, error } = await supabase.rpc('exec_sql', {
    sql: `
      SELECT
        message_id,
        substring(content from 1 for 80) as content_preview,
        metadata->'ctx'->>'metadata' as ctx_metadata_string,
        regexp_replace(
          (metadata->'ctx'->>'metadata'),
          '.*project::([^\\]]+).*',
          '\\1'
        ) as extracted_project
      FROM active_context_stream
      WHERE
        metadata->'ctx'->>'metadata' LIKE '%project::%'
        AND (metadata->'project' IS NULL OR metadata->'project'::text = 'null')
      ORDER BY timestamp DESC
      LIMIT 20
    `
  });

  if (error) {
    console.error('❌ Preview query failed:', error);
    // Try direct query instead - fetch all and filter in JS
    const { data: results, error: err2 } = await supabase
      .from('active_context_stream')
      .select('message_id,content,metadata')
      .order('timestamp', { ascending: false })
      .limit(100); // Get more to ensure we find some matches

    if (err2) {
      console.error('❌ Alternative query also failed:', err2);
      return;
    }

    // Filter in JavaScript
    const needsUpdate = results?.filter(record => {
      const ctxMetadata = record.metadata?.ctx?.metadata;
      const hasProjectInCtx = ctxMetadata && ctxMetadata.includes('project::');
      const missingTopLevelProject = !record.metadata?.project;
      return hasProjectInCtx && missingTopLevelProject;
    }) || [];

    console.log(`Found ${needsUpdate.length} records to update (scanned ${results?.length || 0} total):\n`);

    needsUpdate.slice(0, 20).forEach((record, idx) => {
      const ctxMetadata = record.metadata?.ctx?.metadata;
      const match = ctxMetadata?.match(/project::([^\]]+)/);
      const extractedProject = match ? match[1] : 'N/A';

      console.log(`${idx + 1}. ${record.message_id}`);
      console.log(`   Content: ${record.content.substring(0, 80)}...`);
      console.log(`   ctx.metadata: ${ctxMetadata || 'none'}`);
      console.log(`   → Will set project: ${extractedProject}`);
      console.log('');
    });

    return;
  }

  console.log(`Found ${data?.length || 0} records:\n`);
  data?.forEach((row: any, idx: number) => {
    console.log(`${idx + 1}. ${row.message_id}`);
    console.log(`   Content: ${row.content_preview}...`);
    console.log(`   ctx.metadata: ${row.ctx_metadata_string}`);
    console.log(`   → Will set project: ${row.extracted_project}`);
    console.log('');
  });
}

async function getCount() {
  // Fetch all records and count in JS (PostgREST filtering too complex)
  const { data, error } = await supabase
    .from('active_context_stream')
    .select('metadata')
    .order('timestamp', { ascending: false })
    .limit(1000); // Reasonable limit

  if (error) {
    console.error('❌ Count query failed:', error);
    return 0;
  }

  const needsUpdate = data?.filter(record => {
    const ctxMetadata = record.metadata?.ctx?.metadata;
    const hasProjectInCtx = ctxMetadata && ctxMetadata.includes('project::');
    const missingTopLevelProject = !record.metadata?.project;
    return hasProjectInCtx && missingTopLevelProject;
  }) || [];

  return needsUpdate.length;
}

async function execute() {
  console.log('📊 Counting affected records...\n');

  const count = await getCount();
  console.log(`Will update ${count} records\n`);

  if (count === 0) {
    console.log('✅ No records to update!');
    return;
  }

  // Show preview first
  await preview();

  console.log('\n⚠️  WARNING: This will update the database!');
  console.log('Type "yes" to continue, anything else to cancel:\n');

  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
  });

  const answer = await new Promise<string>(resolve => {
    rl.question('> ', resolve);
  });
  rl.close();

  if (answer.toLowerCase() !== 'yes') {
    console.log('❌ Migration cancelled');
    return;
  }

  console.log('\n🚀 Running migration...\n');

  // Fetch records and update manually since we can't use RPC
  const { data: allRecords, error: fetchError } = await supabase
    .from('active_context_stream')
    .select('message_id,metadata')
    .order('timestamp', { ascending: false })
    .limit(1000); // Process up to 1000 records

  if (fetchError) {
    console.error('❌ Fetch failed:', fetchError);
    return;
  }

  // Filter to only records that need updating
  const records = allRecords?.filter(record => {
    const ctxMetadata = record.metadata?.ctx?.metadata;
    const hasProjectInCtx = ctxMetadata && ctxMetadata.includes('project::');
    const missingTopLevelProject = !record.metadata?.project;
    return hasProjectInCtx && missingTopLevelProject;
  }) || [];

  console.log(`Found ${records.length} records to update\n`);

  let updated = 0;
  for (const record of records) {
    const ctxMetadata = record.metadata?.ctx?.metadata;
    const match = ctxMetadata?.match(/project::([^\]]+)/);

    if (match) {
      const projectValue = match[1].trim();
      const updatedMetadata = {
        ...record.metadata,
        project: projectValue
      };

      const { error: updateError } = await supabase
        .from('active_context_stream')
        .update({ metadata: updatedMetadata })
        .eq('message_id', record.message_id);

      if (updateError) {
        console.error(`❌ Failed to update ${record.message_id}:`, updateError);
      } else {
        updated++;
        if (updated % 10 === 0) {
          console.log(`   Updated ${updated}/${records.length}...`);
        }
      }
    }
  }

  console.log(`\n✅ Migration complete! Updated ${updated} records\n`);

  // Verify
  console.log('🔍 Verifying updates...\n');
  const { data: verified, error: verifyError } = await supabase
    .from('active_context_stream')
    .select('message_id,metadata')
    .not('metadata->project', 'is', null)
    .order('timestamp', { ascending: false })
    .limit(5);

  if (verifyError) {
    console.error('❌ Verification failed:', verifyError);
    return;
  }

  verified?.forEach((record, idx) => {
    console.log(`${idx + 1}. ${record.message_id}`);
    console.log(`   project: ${record.metadata.project}`);
    console.log('');
  });
}

// Main
const command = process.argv[2];

(async () => {
  switch (command) {
    case 'preview':
      await preview();
      break;
    case 'execute':
      await execute();
      break;
    default:
      console.log('Usage:');
      console.log('  tsx scripts/run-migration.ts preview   # Preview changes');
      console.log('  tsx scripts/run-migration.ts execute   # Run migration');
      process.exit(1);
  }
})();
