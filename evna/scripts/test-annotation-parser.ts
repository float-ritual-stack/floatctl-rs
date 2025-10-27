#!/usr/bin/env tsx
/**
 * Test harness for annotation parser
 *
 * Usage:
 *   tsx scripts/test-annotation-parser.ts
 */

import { AnnotationParser } from '../src/lib/annotation-parser.js';

const parser = new AnnotationParser();

// Test cases
const testCases = [
  {
    name: 'Direct project annotation',
    content: 'project::rangle/pharmacy - some text',
  },
  {
    name: 'Project in ctx block',
    content: 'ctx::2025-10-23 @ 01:55 PM - [project::evna-next/dev-tooling] - User requested test harness',
  },
  {
    name: 'Project in ctx block (pharmacy)',
    content: 'ctx::2025-10-23 @ 12:59 PM - [project::rangle/pharmacy] - [issue::551]',
  },
  {
    name: 'Multiple annotations',
    content: 'ctx::2025-10-23 @ 02:01 PM - [project::evna-next/test] - [eureka::found_it] - Testing parser',
  },
];

console.log('🧪 Testing Annotation Parser\n');

testCases.forEach((testCase, idx) => {
  console.log(`--- Test ${idx + 1}: ${testCase.name} ---`);
  console.log(`Input: ${testCase.content}`);

  const metadata = parser.extractMetadata(testCase.content);

  console.log(`Project: ${metadata.project || 'none'}`);
  console.log(`Metadata:`, JSON.stringify(metadata, null, 2));
  console.log('');
});
