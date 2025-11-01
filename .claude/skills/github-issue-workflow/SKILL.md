---
name: github-issue-workflow
description: Process GitHub issues for float-ritual-stack projects with proper workflow - read, label in-progress, update with progress, commit work, close when done. Float-ism - direct-to-main commits, frequent updates, emoji-rich communication.
---

# GitHub Issue Workflow (Float-Ritual-Stack)

When asked to "work on issue X", "process issue X", or "handle float-hub#7", follow this workflow.

This is Evan's personal workflow for float-ritual-stack projects - optimized for fast iteration and rich communication.

## Step 1: Read & Start
1. **Read the issue**: `github_read_issue(repo, number)`
   - Understand what's being asked
   - Note any specific requirements

2. **Add to project board** (if not already there):
   ```bash
   # Add issue to float-hub-operations project
   gh issue view {number} --repo {repo} --json projectItems --jq '.projectItems[].id' || \
   gh project item-add 5 --owner float-ritual-stack --url https://github.com/{repo}/issues/{number}
   ```

3. **Move to "In Progress"**:
   ```bash
   # Get the project item ID for this issue
   ITEM_ID=$(gh project item-list 5 --owner float-ritual-stack --format json | \
     jq -r ".items[] | select(.content.number=={number}) | .id")

   # Move to "In Progress" status
   gh project item-edit --id $ITEM_ID \
     --project-id PVT_kwDODNDomc4BG-s0 \
     --field-id PVTSSF_lADODNDomc4BG-s0zg33j_Q \
     --single-select-option-id 47fc9ee4
   ```

4. **Mark in-progress**: `github_add_label(repo, number, "in-progress")`

5. **Post starting comment**:
   ```
   github_comment_issue(repo, number, "🤖 Starting work on this issue...")
   ```

## Step 2: Do The Work

Execute the task described in the issue. Common patterns:

### For Bridge Creation
- Read source content (GitHub issue body, daily notes, etc.)
- Synthesize with LLM (ask_evna, semantic_search, brain_boot)
- Create bridge document with proper frontmatter
- Write to `~/float-hub/float.dispatch/bridges/`

### For Code/Config Changes
- Make the changes
- Test if appropriate
- Document what was changed

### For Research/Synthesis
- Gather information
- Synthesize findings
- Create output document

## Step 3: Commit Frequently

**Float-ism: Commit often, commit fearlessly**

- **NO branching required** for float-ritual-stack repos
- Commit directly to main (these are notes, not production)
- Commit after:
  - Creating files
  - Major milestones
  - Logical checkpoints

**Good commit messages**:
```
feat(bridges): Add github-issue-workflow skill bridge
docs(evna): Document background task architecture
fix(cli): Handle missing notify-issue gracefully
```

**Use available tools**:
- `write_file` to create/update files
- `read_file` to verify changes
- For git commits, use `Bash` tool with git commands

## Step 4: Update Progress

Post progress comments to the issue as you work:

```
github_comment_issue(repo, number, "📝 Created bridge document at bridges/xyz.bridge.md

Next: Adding synthesis from recent Claude sessions...")
```

**Emoji guide**:
- 🤖 Starting work
- 📝 Writing/creating
- 🔍 Researching/searching
- ✅ Completed
- 🐛 Found issue
- 🔧 Fixing

**Update when**:
- Starting major steps
- Hitting blockers
- Completing major milestones
- Every 5-10 minutes of work (don't go silent!)

## Step 5: Finish & Close

1. **Final commit** with summary of all changes

2. **Post completion comment**:
   ```
   github_comment_issue(repo, number, "✅ Completed!

   **Created**:
   - bridges/github-issue-workflow.bridge.md

   **Summary**:
   Synthesized GitHub issue workflow into a bridge document with float-ism conventions.

   **Commits**: 3 commits pushed to main")
   ```

3. **Move to "Done"** on project board:
   ```bash
   # Get the project item ID for this issue
   ITEM_ID=$(gh project item-list 5 --owner float-ritual-stack --format json | \
     jq -r ".items[] | select(.content.number=={number}) | .id")

   # Move to "Done" status
   gh project item-edit --id $ITEM_ID \
     --project-id PVT_kwDODNDomc4BG-s0 \
     --field-id PVTSSF_lADODNDomc4BG-s0zg33j_Q \
     --single-select-option-id 98236657
   ```

4. **Remove in-progress label**:
   ```
   github_remove_label(repo, number, "in-progress")
   ```

5. **Close the issue**:
   ```
   github_close_issue(repo, number, "✅ Completed! See comments above for details.")
   ```

## Repository-Specific Rules

### float-ritual-stack/* repos (ALL)
- ✅ Commit directly to main
- ✅ Fast iteration over ceremony
- ✅ Rich emoji communication
- ✅ Frequent progress updates
- ❌ No branching required
- ❌ No PR process

These are Evan's personal projects and notes repos - optimize for flow, not gates.

### Other organizations
If working on repos outside float-ritual-stack, follow their conventions (branches, PRs, etc.)

## Common Patterns

### Pattern: Issue → Bridge
```
1. Read issue: github_read_issue
2. Add to project board (if needed)
3. Move to "In Progress" on board
4. Mark in-progress label
5. Post "🤖 Starting..."
6. Search for related context (semantic_search, brain_boot)
7. Synthesize content
8. Create bridge document (write_file)
9. Commit: "feat(bridges): Add X bridge from issue #Y"
10. Post "✅ Completed" with file path
11. Move to "Done" on board
12. Remove in-progress label
13. Close issue
```

### Pattern: Issue → Code Change
```
1. Read issue
2. Mark in-progress
3. Post "🤖 Starting..."
4. Make code changes
5. Commit: "feat(X): Implement Y per issue #Z"
6. Post progress with what changed
7. Test if needed
8. Post "✅ Completed" with summary
9. Remove in-progress, close issue
```

### Pattern: Issue → Research/Documentation
```
1. Read issue
2. Mark in-progress
3. Post "🤖 Starting..."
4. Gather information (search, read files)
5. Synthesize findings
6. Create document (daily note, bridge, etc.)
7. Commit: "docs(X): Add Y from issue #Z"
8. Post "✅ Completed" with document location
9. Remove in-progress, close issue
```

## Error Handling

If you hit blockers:
1. Post comment describing the blocker
2. Keep "in-progress" label
3. Ask for clarification or help
4. DON'T close the issue

If you can't complete:
1. Post what you accomplished
2. Post what's still needed
3. Remove "in-progress" label
4. Leave issue open

## Best Practices

✅ **DO**:
- Commit frequently (after each logical step)
- Update the issue regularly (every 5-10 mins of work)
- Use descriptive commit messages
- Use emoji in issue comments
- Post completion summaries with file paths
- Close issues when truly done

❌ **DON'T**:
- Go silent for long periods
- Make one giant commit at the end
- Close issues prematurely
- Leave "in-progress" label on closed issues
- Skip progress updates

## Float-Ism Philosophy

> "Fast iteration, rich communication, fearless commits"

These workflows optimize for:
- **Velocity** over ceremony
- **Transparency** over stealth
- **Done** over perfect

We're building in public, for ourselves, with AI assistants. Make it work, make it visible, make it flow.

---

## Appendix: Project Board Configuration

### float-hub-operations Project (ID: 5)
- **Project ID**: `PVT_kwDODNDomc4BG-s0`
- **Status Field ID**: `PVTSSF_lADODNDomc4BG-s0zg33j_Q`

**Status Options**:
- **Todo**: `f75ad846`
- **In Progress**: `47fc9ee4`
- **Done**: `98236657`

### Helpful Commands

**List all projects**:
```bash
gh project list --owner float-ritual-stack --format json
```

**Get project fields**:
```bash
gh project field-list 5 --owner float-ritual-stack --format json
```

**Check if issue is in project**:
```bash
gh issue view {number} --repo {repo} --json projectItems
```

**Get item ID from issue number**:
```bash
gh project item-list 5 --owner float-ritual-stack --format json | \
  jq -r ".items[] | select(.content.number=={number}) | .id"
```
