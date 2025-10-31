/**
 * Search Session Tracker
 *
 * Tracks search attempts during ask_evna orchestration and decides when to
 * terminate early to prevent token explosion on negative searches.
 *
 * Problem: evna burns 138k+ tokens exhaustively searching when data doesn't exist
 * Solution: Early termination heuristics based on result quality and token usage
 */

export interface SearchAttempt {
  tool: string;
  input: any;
  resultsFound: boolean;
  resultQuality: 'high' | 'medium' | 'low' | 'none';
  tokenCost: number;
  timestamp: string;
  resultCount?: number;
  avgSimilarity?: number;
}

export interface TerminationReason {
  shouldTerminate: boolean;
  reason?: string;
  message?: string;
}

/**
 * Tracks search attempts and determines when to stop searching
 */
export class SearchSession {
  private attempts: SearchAttempt[] = [];
  private totalTokens: number = 0;
  private userQuery: string;

  // Tunable thresholds
  private readonly MAX_TOKENS_NEGATIVE = 15000;  // Hard token cap for negative searches
  private readonly CONSECUTIVE_MISSES = 3;       // Three strikes rule
  private readonly MIN_SIMILARITY = 0.3;         // Below this = "low quality"

  constructor(userQuery: string) {
    this.userQuery = userQuery;
  }

  /**
   * Record a search attempt
   */
  addAttempt(attempt: SearchAttempt): void {
    this.attempts.push(attempt);
    this.totalTokens += attempt.tokenCost;
  }

  /**
   * Main termination logic - checks all heuristics
   */
  shouldTerminate(): TerminationReason {
    // Rule 1: Hard token cap for negative searches
    const tokenCapCheck = this.checkTokenCap();
    if (tokenCapCheck.shouldTerminate) return tokenCapCheck;

    // Rule 2: Three consecutive misses
    const threeStrikesCheck = this.checkThreeStrikes();
    if (threeStrikesCheck.shouldTerminate) return threeStrikesCheck;

    // Rule 3: Declining quality trend
    const qualityTrendCheck = this.checkQualityTrend();
    if (qualityTrendCheck.shouldTerminate) return qualityTrendCheck;

    // Rule 4: Project mismatch pattern
    const projectMismatchCheck = this.checkProjectMismatch();
    if (projectMismatchCheck.shouldTerminate) return projectMismatchCheck;

    return { shouldTerminate: false };
  }

  /**
   * Rule 1: Stop if we've burned >15k tokens with no results
   */
  private checkTokenCap(): TerminationReason {
    if (this.totalTokens > this.MAX_TOKENS_NEGATIVE && this.getFoundCount() === 0) {
      return {
        shouldTerminate: true,
        reason: 'token_cap',
        message: `Searched ${this.attempts.length} sources (${this.totalTokens} tokens) but found no relevant results.`
      };
    }
    return { shouldTerminate: false };
  }

  /**
   * Rule 2: Stop after 3 consecutive "none" quality results
   */
  private checkThreeStrikes(): TerminationReason {
    if (this.attempts.length < this.CONSECUTIVE_MISSES) {
      return { shouldTerminate: false };
    }

    const recentAttempts = this.attempts.slice(-this.CONSECUTIVE_MISSES);
    const allNone = recentAttempts.every(a => a.resultQuality === 'none');

    if (allNone) {
      const toolsSearched = recentAttempts.map(a => a.tool).join(', ');
      return {
        shouldTerminate: true,
        reason: 'three_strikes',
        message: `Searched ${toolsSearched} with no results. The information may not be in accessible context.`
      };
    }

    return { shouldTerminate: false };
  }

  /**
   * Rule 3: Stop if quality is declining over last 3 attempts
   */
  private checkQualityTrend(): TerminationReason {
    if (this.attempts.length < 3) {
      return { shouldTerminate: false };
    }

    const recentAttempts = this.attempts.slice(-3);
    const trend = this.getQualityTrend(recentAttempts);

    if (trend === 'declining' && this.attempts.length >= 3) {
      return {
        shouldTerminate: true,
        reason: 'declining_quality',
        message: 'Search results are getting less relevant. Stopping to avoid token waste.'
      };
    }

    return { shouldTerminate: false };
  }

  /**
   * Rule 4: Stop if we're finding wrong project consistently
   * (e.g., asked for "floatctl" but finding "bootstrap.evna")
   */
  private checkProjectMismatch(): TerminationReason {
    // TODO: Implement project extraction from query and result comparison
    // For now, defer this heuristic
    return { shouldTerminate: false };
  }

  /**
   * Get quality trend over recent attempts
   */
  private getQualityTrend(attempts: SearchAttempt[]): 'improving' | 'stable' | 'declining' {
    if (attempts.length < 2) return 'stable';

    const qualityScore = (q: string) => {
      switch (q) {
        case 'high': return 3;
        case 'medium': return 2;
        case 'low': return 1;
        case 'none': return 0;
        default: return 0;
      }
    };

    const scores = attempts.map(a => qualityScore(a.resultQuality));

    // Check if generally declining
    let decliningCount = 0;
    for (let i = 1; i < scores.length; i++) {
      if (scores[i] < scores[i-1]) decliningCount++;
    }

    if (decliningCount >= scores.length - 1) return 'declining';
    if (decliningCount === 0) return 'improving';
    return 'stable';
  }

  /**
   * Count how many searches found results
   */
  private getFoundCount(): number {
    return this.attempts.filter(a => a.resultsFound).length;
  }

  /**
   * Get all attempts
   */
  getAttempts(): SearchAttempt[] {
    return this.attempts;
  }

  /**
   * Get total token usage
   */
  getTotalTokens(): number {
    return this.totalTokens;
  }

  /**
   * Build graceful negative response
   */
  buildNegativeResponse(): string {
    const searchedTools = [...new Set(this.attempts.map(a => a.tool))].join(', ');
    const attemptCount = this.attempts.length;

    return `I searched multiple sources (${searchedTools}, ${attemptCount} searches) but couldn't find recent work on this topic.

This could mean:
- The work hasn't been captured in accessible context yet
- It occurred outside my searchable timeframe
- The terminology might be different than expected

Would you like me to search with different terms, or check a specific timeframe/project?`;
  }
}

/**
 * Score the quality of search results
 */
export function scoreResultQuality(
  results: any[],
  resultText?: string
): 'high' | 'medium' | 'low' | 'none' {
  // No results = none
  if (results.length === 0 || resultText?.includes('**No results found**')) {
    return 'none';
  }

  // Check for similarity scores in results
  const similarities = results
    .map((r: any) => r.similarity_score || r.similarity || 0)
    .filter((s: number) => s > 0);

  if (similarities.length > 0) {
    const avgSimilarity = similarities.reduce((a: number, b: number) => a + b, 0) / similarities.length;

    if (avgSimilarity >= 0.5) return 'high';
    if (avgSimilarity >= 0.3) return 'medium';
    return 'low';
  }

  // Fallback: if we have results but no similarity scores, assume medium
  return results.length > 5 ? 'medium' : 'low';
}
