---
name: weekly-summary
description: Summarize work done in the last week from authored pull requests
allowed-tools: Bash(gh *)
---

# Weekly Work Summary

Summarize all work done in the last week by gathering information from pull requests authored by the current user.
Emphasize why the pull requests are important to the company and why the director and vice presidents at the company should care about the work that was done. Focus on the impact of the work and how it contributes to the company's goals and success.

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

5. Produce a summary organized by repository with the following structure:

   ### Summary Format

   **Period:** `<start_date>` to `<today>`

   For each repository that had PR activity:

   **`<owner/repo>`**

   - **Merged PRs** — list each with title, PR number (linked to URL), and a one-sentence description of what was accomplished based on the PR title and body.
   - **Open PRs** — list each with title, PR number (linked to URL), and current status.

   End with a brief **highlights** section (3-5 bullets) calling out the most significant accomplishments across all repos.
