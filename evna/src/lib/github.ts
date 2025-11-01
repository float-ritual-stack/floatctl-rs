/**
 * GitHub client using gh CLI for fetching PR and issue status
 */

import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

/**
 * Safely escape shell arguments for use in commands
 * Prevents command injection by wrapping in single quotes and escaping embedded quotes
 */
function escapeShellArg(arg: string | number): string {
  const str = String(arg);
  // Replace single quotes with '\'' (end quote, escaped quote, start quote)
  return `'${str.replace(/'/g, "'\\''")}'`;
}

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
  private readonly WRITE_ALLOWED_ORGS = ['float-ritual-stack'];

  constructor(repo: string) {
    this.repo = repo; // e.g., "pharmonline/pharmacy-online"
  }

  /**
   * Validate write access to a repository
   * Only allows writes to repos in WRITE_ALLOWED_ORGS
   */
  private validateWriteAccess(repo: string): void {
    const [org] = repo.split('/');
    if (!this.WRITE_ALLOWED_ORGS.includes(org)) {
      throw new Error(
        `Write access denied for repo '${repo}'. ` +
        `Only repos in these organizations are allowed: ${this.WRITE_ALLOWED_ORGS.join(', ')}`
      );
    }
  }

  /**
   * Get open PRs for a user using gh CLI
   */
  async getUserPRs(username: string): Promise<GitHubPR[]> {
    try {
      const { stdout } = await execAsync(
        `gh pr list --repo ${escapeShellArg(this.repo)} --author ${escapeShellArg(username)} --state open --json number,title,state,isDraft,url,createdAt,updatedAt,author,labels,reviewDecision,statusCheckRollup --limit 100`
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
        `gh issue list --repo ${escapeShellArg(this.repo)} --assignee ${escapeShellArg(username)} --state open --json number,title,state,url,createdAt,updatedAt,assignees,labels --limit 100`
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

  /**
   * Read a specific issue from any repository (read-only, no restrictions)
   */
  async readIssue(repo: string, number: number): Promise<string> {
    try {
      const { stdout } = await execAsync(
        `gh issue view ${escapeShellArg(number)} --repo ${escapeShellArg(repo)} --json number,title,body,state,url,createdAt,updatedAt,author,assignees,labels`
      );

      const issue = JSON.parse(stdout);

      const lines: string[] = [];
      lines.push(`# Issue #${issue.number}: ${issue.title}`);
      lines.push(`**Repository**: ${repo}`);
      lines.push(`**State**: ${issue.state}`);
      lines.push(`**URL**: ${issue.url}`);
      lines.push(`**Author**: ${issue.author.login}`);
      lines.push(`**Created**: ${new Date(issue.createdAt).toLocaleString()}`);
      lines.push(`**Updated**: ${new Date(issue.updatedAt).toLocaleString()}`);

      if (issue.assignees && issue.assignees.length > 0) {
        lines.push(`**Assignees**: ${issue.assignees.map((a: any) => a.login).join(', ')}`);
      }

      if (issue.labels && issue.labels.length > 0) {
        lines.push(`**Labels**: ${issue.labels.map((l: any) => l.name).join(', ')}`);
      }

      lines.push('');
      lines.push('## Body');
      lines.push(issue.body || '*(No description provided)*');

      return lines.join('\n');
    } catch (error) {
      throw new Error(`Failed to read issue ${repo}#${number}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  /**
   * Post a comment to an issue (write operation - restricted)
   */
  async commentIssue(repo: string, number: number, body: string): Promise<string> {
    this.validateWriteAccess(repo);

    try {
      await execAsync(
        `gh issue comment ${escapeShellArg(number)} --repo ${escapeShellArg(repo)} --body ${JSON.stringify(body)}`
      );

      return `✅ Comment posted to ${repo}#${number}`;
    } catch (error) {
      throw new Error(`Failed to comment on issue ${repo}#${number}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  /**
   * Close an issue (write operation - restricted)
   */
  async closeIssue(repo: string, number: number, comment?: string): Promise<string> {
    this.validateWriteAccess(repo);

    try {
      const cmd = comment
        ? `gh issue close ${escapeShellArg(number)} --repo ${escapeShellArg(repo)} --comment ${JSON.stringify(comment)}`
        : `gh issue close ${escapeShellArg(number)} --repo ${escapeShellArg(repo)}`;

      await execAsync(cmd);

      return `✅ Closed issue ${repo}#${number}`;
    } catch (error) {
      throw new Error(`Failed to close issue ${repo}#${number}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  /**
   * Add a label to an issue (write operation - restricted)
   */
  async addLabel(repo: string, number: number, label: string): Promise<string> {
    this.validateWriteAccess(repo);

    try {
      await execAsync(
        `gh issue edit ${escapeShellArg(number)} --repo ${escapeShellArg(repo)} --add-label ${escapeShellArg(label)}`
      );

      return `✅ Added label '${label}' to ${repo}#${number}`;
    } catch (error) {
      throw new Error(`Failed to add label to issue ${repo}#${number}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  /**
   * Remove a label from an issue (write operation - restricted)
   */
  async removeLabel(repo: string, number: number, label: string): Promise<string> {
    this.validateWriteAccess(repo);

    try {
      await execAsync(
        `gh issue edit ${escapeShellArg(number)} --repo ${escapeShellArg(repo)} --remove-label ${escapeShellArg(label)}`
      );

      return `✅ Removed label '${label}' from ${repo}#${number}`;
    } catch (error) {
      throw new Error(`Failed to remove label from issue ${repo}#${number}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
}
