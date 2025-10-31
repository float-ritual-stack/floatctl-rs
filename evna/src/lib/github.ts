/**
 * GitHub client using gh CLI for fetching PR and issue status
 */

import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

export interface GitHubPR {
  number: number;
  title: string;
  state: string;
  isDraft: boolean;
  url: string;
  createdAt: string;
  updatedAt: string;
  author: string;
  labels: string[];
  reviewDecision?: string;
  statusCheckRollup?: string;
}

export interface GitHubIssue {
  number: number;
  title: string;
  state: string;
  url: string;
  createdAt: string;
  updatedAt: string;
  assignees: string[];
  labels: string[];
}

export class GitHubClient {
  private repo: string;

  constructor(repo: string) {
    this.repo = repo; // e.g., "pharmonline/pharmacy-online"
  }

  /**
   * Get open PRs for a user using gh CLI
   */
  async getUserPRs(username: string): Promise<GitHubPR[]> {
    try {
      const { stdout } = await execAsync(
        `gh pr list --repo ${this.repo} --author ${username} --state open --json number,title,state,isDraft,url,createdAt,updatedAt,author,labels,reviewDecision,statusCheckRollup --limit 100`
      );

      return JSON.parse(stdout);
    } catch (error) {
      // Note: No console.error here - MCP uses stderr for JSON-RPC
      return [];
    }
  }

  /**
   * Get issues assigned to a user using gh CLI
   */
  async getUserIssues(username: string): Promise<GitHubIssue[]> {
    try {
      const { stdout } = await execAsync(
        `gh issue list --repo ${this.repo} --assignee ${username} --state open --json number,title,state,url,createdAt,updatedAt,assignees,labels --limit 100`
      );

      return JSON.parse(stdout);
    } catch (error) {
      // Note: No console.error here - MCP uses stderr for JSON-RPC
      return [];
    }
  }

  /**
   * Format PR status as markdown
   */
  formatPRStatus(prs: GitHubPR[]): string {
    if (prs.length === 0) {
      return '**No open PRs**';
    }

    const lines: string[] = ['## Open Pull Requests\n'];

    prs.forEach(pr => {
      let statusIcon = '🔍';
      let statusText = 'Review';

      if (pr.isDraft) {
        statusIcon = '📝';
        statusText = 'Draft';
      } else if (pr.reviewDecision === 'APPROVED') {
        statusIcon = '✅';
        statusText = 'Approved';
      } else if (pr.reviewDecision === 'CHANGES_REQUESTED') {
        statusIcon = '🔄';
        statusText = 'Changes Requested';
      }

      lines.push(`- ${statusIcon} **${statusText}** [#${pr.number}](${pr.url}): ${pr.title}`);
      lines.push(`  - Updated: ${new Date(pr.updatedAt).toLocaleString()}`);

      if (pr.statusCheckRollup) {
        const checksStatus = pr.statusCheckRollup === 'SUCCESS' ? '✅ Passing' :
                           pr.statusCheckRollup === 'FAILURE' ? '❌ Failing' :
                           '⏳ Pending';
        lines.push(`  - Checks: ${checksStatus}`);
      }

      if (pr.labels.length > 0) {
        lines.push(`  - Labels: ${pr.labels.map(l => typeof l === 'string' ? l : (l as any).name).join(', ')}`);
      }
      lines.push('');
    });

    return lines.join('\n');
  }

  /**
   * Format issue status as markdown
   */
  formatIssueStatus(issues: GitHubIssue[]): string {
    if (issues.length === 0) {
      return '**No assigned issues**';
    }

    const lines: string[] = ['## Assigned Issues\n'];

    issues.forEach(issue => {
      lines.push(`- [#${issue.number}](${issue.url}): ${issue.title}`);
      lines.push(`  - Updated: ${new Date(issue.updatedAt).toLocaleString()}`);

      if (issue.labels.length > 0) {
        lines.push(`  - Labels: ${issue.labels.map(l => typeof l === 'string' ? l : (l as any).name).join(', ')}`);
      }
      lines.push('');
    });

    return lines.join('\n');
  }

  /**
   * Get comprehensive status for a user
   */
  async getUserStatus(username: string): Promise<string> {
    try {
      const [prs, issues] = await Promise.all([
        this.getUserPRs(username),
        this.getUserIssues(username),
      ]);

      const lines: string[] = ['# GitHub Status\n'];
      lines.push(this.formatPRStatus(prs));
      lines.push(this.formatIssueStatus(issues));

      return lines.join('\n');
    } catch (error) {
      // Note: No console.error here - MCP uses stderr for JSON-RPC
      return `**GitHub Error**: ${error instanceof Error ? error.message : String(error)}`;
    }
  }
}
