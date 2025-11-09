#!/usr/bin/env bun
/**
 * Test script to verify Claude projects context injection
 */

import { AskEvnaAgent } from "./src/tools/ask-evna-agent.js";

const agent = new AskEvnaAgent();

console.log("Testing ask_evna with context injection...\n");

const result = await agent.ask({
  query: `What recent Claude Code projects can you see in your context? 
  
DO NOT use any tools. Only look at what's already in your system prompt.
List the project names and most recent activity you can see.`,
  timeout_ms: 10000,
  include_projects_context: true,
  all_projects: false, // Just evna project
});

console.log("\n=== RESULT ===");
console.log(result.response);
console.log("\nSession ID:", result.session_id);
