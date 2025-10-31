#!/usr/bin/env bun
/**
 * Quick test script for MCP resources
 * Tests daily:// resources without starting full MCP server
 */

import { readFile, readdir } from "fs/promises";
import { join } from "path";
import { homedir } from "os";

const dailyDir = join(homedir(), '.evans-notes', 'daily');

async function testToday() {
  console.log("\n🧪 Testing daily://today");
  const today = new Date().toISOString().split('T')[0];
  const notePath = join(dailyDir, `${today}.md`);
  const content = await readFile(notePath, 'utf-8');
  console.log(`✅ Found today's note (${today}): ${content.split('\n')[0].slice(0, 50)}...`);
}

async function testRecent() {
  console.log("\n🧪 Testing daily://recent");
  const recentDates: string[] = [];
  for (let i = 0; i < 3; i++) {
    const d = new Date();
    d.setDate(d.getDate() - i);
    recentDates.push(d.toISOString().split('T')[0]);
  }

  const sections = await Promise.all(
    recentDates.map(async (date) => {
      const notePath = join(dailyDir, `${date}.md`);
      try {
        const content = await readFile(notePath, 'utf-8');
        return `# ${date}\n\n${content}`;
      } catch (err) {
        return `# ${date}\n\n*(No note found)*`;
      }
    })
  );

  const combined = sections.join('\n\n---\n\n');
  console.log(`✅ Concatenated ${recentDates.length} days`);
  console.log(`   Dates: ${recentDates.join(', ')}`);
  console.log(`   Total length: ${combined.length} chars`);
}

async function testWeek() {
  console.log("\n🧪 Testing daily://week");
  const weekDates: string[] = [];
  for (let i = 0; i < 7; i++) {
    const d = new Date();
    d.setDate(d.getDate() - i);
    weekDates.push(d.toISOString().split('T')[0]);
  }

  const sections = await Promise.all(
    weekDates.map(async (date) => {
      const notePath = join(dailyDir, `${date}.md`);
      try {
        const content = await readFile(notePath, 'utf-8');
        return `# ${date}\n\n${content}`;
      } catch (err) {
        return `# ${date}\n\n*(No note found)*`;
      }
    })
  );

  const combined = sections.join('\n\n---\n\n');
  console.log(`✅ Concatenated ${weekDates.length} days`);
  console.log(`   Dates: ${weekDates.join(', ')}`);
  console.log(`   Total length: ${combined.length} chars`);
}

async function testList() {
  console.log("\n🧪 Testing daily://list");
  const files = await readdir(dailyDir);
  const noteFiles = files
    .filter((f) => /^\d{4}-\d{2}-\d{2}\.md$/.test(f))
    .map((f) => f.replace('.md', ''))
    .sort()
    .reverse()
    .slice(0, 30);

  console.log(`✅ Found ${noteFiles.length} daily notes (last 30 days)`);
  console.log(`   Most recent: ${noteFiles.slice(0, 5).join(', ')}`);
  console.log(`   JSON preview: ${JSON.stringify({ notes: noteFiles.slice(0, 3) }, null, 2)}`);
}

async function main() {
  console.log("🚀 MCP Resource Tests\n");

  try {
    await testToday();
    await testRecent();
    await testWeek();
    await testList();
    console.log("\n✅ All tests passed!");
  } catch (error) {
    console.error("\n❌ Test failed:", error);
    process.exit(1);
  }
}

main();
