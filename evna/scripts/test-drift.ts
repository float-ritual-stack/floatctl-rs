import { AnnotationParser } from '../src/lib/annotation-parser.js';
import { canonicalizeProject } from '../src/lib/canonicalize.js';

const parser = new AnnotationParser();

const cases = [
  {
    name: 'Trailing-pipe drift (the #22-row bug)',
    content: 'project::rangle/pharmacy | mode::audit | checking PRs',
    expect: 'rangle/pharmacy',
  },
  {
    name: 'Trailing-pipe in ctx block',
    content: 'ctx::2026-04-12 @ 10:00 AM - [project::floatty] | mode::session-close',
    expect: 'floatty',
  },
  {
    name: 'Case drift (FLOAT → float canonical)',
    content: 'project::FLOAT - planning session',
    expect: 'float', // via canonicalize lowercase path
  },
  {
    name: 'Prose-as-project rescue (long sentence in project field)',
    content: 'project::X] markers which were the biggest metadata gap. sysops-log project coverage went from 30% → 44%',
    shouldLookLikeProse: true,
  },
  {
    name: 'Normal project survives',
    content: 'project::floatty-ai-outline-explorer - audit',
    expect: 'floatty-ai-outline-explorer',
  },
];

let failures = 0;
cases.forEach((c, i) => {
  const meta = parser.extractMetadata(c.content);
  const got = meta.project;
  if (c.shouldLookLikeProse) {
    // Prose-as-project rescue leaves the raw string (long) so downstream can decide to NULL.
    // Success = project is very long OR null (both are fine signals).
    const pass = !got || got.length > 80 || /[.!?]\s/.test(got);
    console.log(`${pass ? '✅' : '❌'} ${i+1}. ${c.name}`);
    console.log(`   got: ${got?.slice(0, 80) ?? 'null'}`);
    if (!pass) failures++;
  } else {
    const pass = got === c.expect;
    console.log(`${pass ? '✅' : '❌'} ${i+1}. ${c.name}`);
    console.log(`   expect: ${c.expect}, got: ${got}`);
    if (!pass) failures++;
  }
});

// Direct canonicalize tests
console.log('\n--- canonicalize.ts ---');
const canonCases: Array<[string, string | null]> = [
  ['rangle/pharmacy |', 'rangle/pharmacy'],
  ['FLOATTY', 'floatty'],
  ['float / consciousness-tech', 'float/consciousness-tech'],
  ['  spaces  ', 'spaces'],
  ['', null],
];
for (const [input, expected] of canonCases) {
  const got = canonicalizeProject(input);
  const pass = got === expected;
  console.log(`${pass ? '✅' : '❌'} canonicalize(${JSON.stringify(input)}) = ${JSON.stringify(got)}, expected ${JSON.stringify(expected)}`);
  if (!pass) failures++;
}

process.exit(failures > 0 ? 1 : 0);
