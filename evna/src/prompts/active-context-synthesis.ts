/**
 * Ollama prompts for active context synthesis
 * Externalized for easy tweaking
 */

export interface ActiveContextSynthesisPromptOptions {
  query: string;
  contextText: string;
  maxWords?: number;
  tweetSize?: number;  // Chars for "other activity" summary
  projectFilter?: string;  // If set, context is already project-filtered
  peripheralContext?: string;  // Optional ambient context (daily notes, other projects)
}

/**
 * Build synthesis prompt for Ollama
 * Two-part output: relevant synthesis + other recent activity tweet
 */
export function buildActiveContextSynthesisPrompt(options: ActiveContextSynthesisPromptOptions): string {
  const { query, contextText, maxWords = 500, tweetSize = 280, projectFilter, peripheralContext } = options;

  const projectNote = projectFilter 
    ? `\n\nNOTE: Context is already filtered to project "${projectFilter}" - all results shown are relevant to that project.`
    : '';

  return `
Synthesize the following recent activity context in relation to this query: "${query}"${projectNote}

## Part 1: Relevant Synthesis

Instructions:
- Be INCLUSIVE for broad queries (e.g., "show all ctx::" should surface ALL ctx:: markers)
- For specific queries (e.g., issue numbers, file names), focus on direct relevance
- If project filter is active, ALL shown content is already project-scoped - don't over-filter
- Preserve :: annotation context (ctx::, project::, mode::, issue::, persona::)
- For technical queries: highlight implementation details, decisions, files changed
- For archaeological queries: surface consciousness tech patterns, meta-observations
- Provide concise synthesis highlighting relevant patterns, decisions, or context
- Use peripheral context (daily notes, other projects) for ambient awareness when helpful
- Only say "No directly relevant recent activity found" if there's genuinely NOTHING matching the query scope
- Keep under ${maxWords} words

## Part 2: Other Recent Activity

After the main synthesis, add a brief "tweet-sized" summary of other notable recent activity that isn't directly related to the query but provides ambient awareness of what else is happening.

Format:
---
**Other recent activity:** [${tweetSize} char max summary of unrelated but notable work]

Instructions for Part 2:
- Keep to ~${tweetSize} characters (tweet-sized)
- Highlight different projects/topics from the query
- Note any significant state changes, completions, or patterns
- Skip if nothing notable outside query scope

Recent activity:
${contextText}${peripheralContext || ''}

Output format:
[Part 1: Your relevant synthesis here]

---
**Other recent activity:** [Your tweet-sized summary of unrelated work here]
`.trim();
}

/**
 * Alternative: Minimal prompt (for faster inference)
 */
export function buildMinimalSynthesisPrompt(query: string, contextText: string): string {
  return `
Filter this recent activity to show ONLY content relevant to: "${query}"

Exclude:
- Messages echoing the user's query
- Unrelated topics
- Repetitive context

Highlight:
- Relevant patterns and decisions
- Key context for the query

Activity:
${contextText}

Relevant synthesis:
`.trim();
}

/**
 * Prompt tuning presets
 */
export const SYNTHESIS_PRESETS = {
  // Default: balanced relevance + ambient awareness
  default: {
    maxWords: 500,
    tweetSize: 280,
    temperature: 0.5,
  },
  
  // Focused: strict relevance only, no ambient awareness
  focused: {
    maxWords: 300,
    tweetSize: 0,  // Disable "other activity" section
    temperature: 0.3,
  },
  
  // Ambient: loose relevance, rich ambient awareness
  ambient: {
    maxWords: 600,
    tweetSize: 400,
    temperature: 0.7,
  },
} as const;
