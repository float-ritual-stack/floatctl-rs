/**
 * Ollama prompts for active context synthesis
 * Externalized for easy tweaking
 */

export interface ActiveContextSynthesisPromptOptions {
  query: string;
  contextText: string;
  maxWords?: number;
  tweetSize?: number;  // Chars for "other activity" summary
}

/**
 * Build synthesis prompt for Ollama
 * Two-part output: relevant synthesis + other recent activity tweet
 */
export function buildActiveContextSynthesisPrompt(options: ActiveContextSynthesisPromptOptions): string {
  const { query, contextText, maxWords = 500, tweetSize = 280 } = options;

  return `
Synthesize the following recent activity context in relation to this query: "${query}"

## Part 1: Relevant Synthesis

Instructions:
- Focus ONLY on content directly relevant to the query
- Exclude messages that just repeat what the user said
- Preserve :: annotation context when relevant (ctx::, project::, mode::, issue::, persona::)
- For technical queries: highlight implementation details, decisions, files changed
- For archaeological queries: surface consciousness tech patterns, meta-observations
- Provide concise synthesis highlighting relevant patterns, decisions, or context
- If nothing is relevant, say "No directly relevant recent activity found"
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
${contextText}

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
