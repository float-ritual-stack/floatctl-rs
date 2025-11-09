#!/usr/bin/env bun
import { getAllProjectsContextInjection } from './src/lib/claude-projects-context.js';

console.log("Testing getAllProjectsContextInjection...\n");

const context = await getAllProjectsContextInjection();

console.log('Context length:', context.length, 'chars');
console.log('\n=== FIRST 2000 CHARS ===');
console.log(context.substring(0, 2000));
console.log('\n=== LAST 1000 CHARS ===');
console.log(context.substring(context.length - 1000));
