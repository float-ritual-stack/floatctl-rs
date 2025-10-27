#!/usr/bin/env bun
/**
 * Test script for double-write pattern
 * Verifies active_context writes to both hot cache AND permanent storage
 */

import { DatabaseClient } from './src/lib/db';
import { config } from 'dotenv';

config();

const supabaseUrl = process.env.SUPABASE_URL!;
const supabaseKey = process.env.SUPABASE_SERVICE_KEY!;

async function testDoubleWrite() {
  console.log('🧪 Testing double-write pattern...\n');

  const db = new DatabaseClient(supabaseUrl, supabaseKey);

  // Test message
  const testMessage = {
    message_id: `test_msg_${Date.now()}`,
    conversation_id: `test_conv_${Date.now()}`,
    role: 'user',
    content: 'ctx::2025-10-24 @ 06:51 PM - [project::evna-next] - [mode::testing] - Direct test of double-write pattern!',
    timestamp: new Date(),
    client_type: 'claude_code' as const,
    metadata: {
      project: 'evna-next',
      ctx: { mode: 'testing' },
      markers: ['testing', 'double_write'],
    },
  };

  console.log('📝 Storing message with double-write...');
  console.log('   Message ID:', testMessage.message_id);
  console.log('   Conversation ID:', testMessage.conversation_id);

  try {
    await db.storeActiveContext(testMessage);
    console.log('✅ storeActiveContext completed\n');

    // Query to verify both writes
    console.log('🔍 Verifying writes...\n');

    // Check hot cache
    const { data: hotCache } = await (db as any).supabase
      .from('active_context_stream')
      .select('*')
      .eq('message_id', testMessage.message_id)
      .single();

    if (hotCache) {
      console.log('✅ Hot cache (active_context_stream):');
      console.log('   Message ID:', hotCache.message_id);
      console.log('   Persisted to long term:', hotCache.persisted_to_long_term);
      console.log('   Persisted message ID:', hotCache.persisted_message_id);
    } else {
      console.log('❌ NOT found in hot cache');
    }

    // Check permanent storage
    if (hotCache?.persisted_message_id) {
      const { data: permanentMsg } = await (db as any).supabase
        .from('messages')
        .select('*')
        .eq('id', hotCache.persisted_message_id)
        .single();

      if (permanentMsg) {
        console.log('\n✅ Permanent storage (messages):');
        console.log('   ID:', permanentMsg.id);
        console.log('   Conversation ID:', permanentMsg.conversation_id);
        console.log('   Content preview:', permanentMsg.content.substring(0, 80) + '...');
        console.log('   Project:', permanentMsg.project);
        console.log('   Markers:', permanentMsg.markers);
      } else {
        console.log('\n❌ Permanent message not found');
      }

      // Check conversation was created
      const { data: conv } = await (db as any).supabase
        .from('conversations')
        .select('*')
        .eq('id', permanentMsg.conversation_id)
        .single();

      if (conv) {
        console.log('\n✅ Conversation created:');
        console.log('   ID:', conv.id);
        console.log('   Conv ID:', conv.conv_id);
        console.log('   Title:', conv.title);
      } else {
        console.log('\n❌ Conversation not found');
      }
    } else {
      console.log('\n❌ Message was NOT persisted to permanent storage');
      console.log('   This means double-write failed');
    }

    console.log('\n🎉 Test complete!');
  } catch (error) {
    console.error('❌ Test failed:', error);
    process.exit(1);
  }
}

testDoubleWrite();
