---
name: weekly-summary
description: Summarize work done in the last week from authored pull requests
allowed-tools: Bash(gh *), mcp__playground-slack-mcp__get_messages(*), mcp__playground-slack-mcp__get_user_profile(*), mcp__playground-slack-mcp__get_reactions(*)
---

# Weekly Work Summary

Summarize all work done in the last week by gathering information from pull requests authored by the current user.
Emphasize why the pull requests are important to the company and why the director, vice presidents, CTO, and CEO at the company should care about the work that was done.
Focus on the impact of the work and how it contributes to the company's goals and success.

Check for the current user's Slack messages in the last week with the most reactions or threads they started with many comments to find important work they did that may not be captured in pull requests, such as important discussions or decisions they contributed to.

Also look for pull requests they reviewed or commented on a lot.
Look for pull requests they approved or requested changes. 

Write the entire summary in a single markdown code block so that it can easily be copied into another tool.

## Steps

1. Determine the date 7 days ago: `date -v-7d +%Y-%m-%d` (macOS) or `date -d "7 days ago" +%Y-%m-%d` (Linux).

2. Search for PRs **merged** in the last week across all repos:
   ```
   gh search prs --author @me --merged-at ">=$DATE" --merged --json repository,title,body,url,mergedAt,number --limit 100
   ```

3. Search for PRs **created** in the last week that are still open:
   ```
   gh search prs --author @me --created ">=$DATE" --state open --json repository,title,body,url,createdAt,number --limit 100
   ```

4. For any PR whose body is empty or truncated in the search results, fetch the full details:
   ```
   gh pr view <number> --repo <owner/repo> --json title,body,url,state,mergedAt,createdAt
   ```

5. Search for the current user's notable Slack activity using the Slack MCP tools:
   - Use `mcp__playground-slack-mcp__get_user_profile` to get the current user's Slack user ID.
   - Use `mcp__playground-slack-mcp__get_messages` to find the user's messages from the period with the most reactions or lengthy threads, focusing on important discussions, decisions, or announcements.
   - Use `mcp__playground-slack-mcp__get_reactions` to identify which messages received significant engagement.

6. Produce a summary organized by repository with the following structure:

   **Period:** `<start_date>` to `<today>`

   For each repository that had PR activity:

   **`<owner/repo>`**

   - **Merged PRs** — list each with title, PR number (linked to URL), and a one-sentence description of what was accomplished based on the PR title and body.
   - **Open PRs** — list each with title, PR number (linked to URL), and current status.
   - **Notable Reviews** — list any PRs where the user was a reviewer, especially those they approved or requested changes on, with links and brief descriptions.

   After the repository sections, include a **Notable Slack Activity** section with:
   - Messages that received many reactions or sparked significant threads
   - Important discussions, decisions, or announcements the user contributed to
   - Brief context on why each was impactful

   End with a brief **highlights** section (3-5 bullets) calling out the most significant accomplishments across all repos and Slack activity.
