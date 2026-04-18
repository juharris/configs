---
name: morning-focus
description: Pull together the signals that should shape my day — PRs, reviews, issues, Slack, calendar, Shopify internal Vault projects, and messages from others, especially my manager — then recommend a priority focus list. Run via `/morning-focus`.
---

# Morning Focus

Produce a concise morning briefing that tells Justin Harris (GitHub alias: juharris) what to focus on today.
Gather signals from GitHub, Slack, Google Calendar, and Shopify internal Vault in parallel, then synthesize them into clear sections followed by a top-3 focus list.

## Principles

- **Parallelize aggressively.** All the data-gathering tool calls are independent — issue them in a single batch.
- **Silence is a signal.** An item with no recent activity from Justin that others are waiting on is a higher priority than a noisy thread he is already engaged in.
- **Who is blocked on Justin?** Teammates waiting on his review, reply, or decision outrank work only he cares about.
- **Don't parrot raw data.** Every item in the output must carry Justin's interpretation (why it matters, who's blocked, what's stale), not just a title and link.
- **Be terse.** Each bullet is one line. No preamble, no closing summary.

## Step 1: Compute dates and look up identities (in parallel)

Run all of the following in one tool-call batch:

1. `date +%Y-%m-%d` — today's date.
2. `date -v-3d +%Y-%m-%dT%H:%M:%S%z` — 3-day lookback cutoff (macOS).
3. Get Justin's Vault profile (current user). The response already includes his **Slack ID** and **manager's ID and name** — you do NOT need a separate Slack user lookup for Justin.

## Step 2: Gather signals (in parallel)

Fire all of these in one batch. Do not serialize them.

### GitHub — PRs and issues

Important: `gh search prs` and `gh pr view` expose different `--json` fields. If a query errors with "Unknown JSON field", run the same command with an invalid field (e.g. `--json _`) to get the current list of valid fields, then retry. Do not hardcode field lists from memory. In particular, review decision and CI status are typically only available on `gh pr view <number> --repo <owner/repo>`, so follow up per-PR on the 1–3 most interesting open PRs (usually your most recent non-draft PR awaiting review).

- **My open PRs:**
  `gh search prs --author @me --state open --json repository,title,url,number,createdAt,updatedAt,isDraft,commentsCount --limit 50`
- **PRs awaiting my review:**
  `gh search prs --review-requested @me --state open --json repository,title,url,number,author,createdAt,updatedAt --limit 50`
- **PRs I've reviewed recently that may need another pass:**
  `gh search prs --reviewed-by @me --state open --updated ">=$DATE_3D" --json repository,title,url,number,author,updatedAt --limit 50`
- **Issues assigned to me:**
  `gh search issues --assignee @me --state open --json repository,title,url,number,updatedAt --limit 50`
- **Issues in shop/issues-sidekick mentioning me recently:**
  `gh search issues --repo shop/issues-sidekick --mentions @me --state open --updated ">=$DATE_3D" --json title,url,number,updatedAt --limit 30`

### Slack — overnight and the last 3 days

- Fetch all unread DMs, mentions, and threads.
- Fetch outstanding saved Slack tasks/reminders (filter for saved/open items).
- Fetch Justin's own recent Slack messages since the 3-day cutoff, to detect threads he replied to that now have follow-ups. Use `count: 50` or lower — higher counts can exceed the token limit; if the response reports truncation with a file path, read that file in chunks rather than retrying with the same count.
- Search Slack for messages from his manager over the last 3 days — messages that need attention. Use `from:@manager.handle after:YYYY-MM-DD` (plain date, no time). Do NOT combine `from:` and `to:` in the same query — Slack search returns no results for that pattern; instead rely on `from:@manager` then filter to threads that mention Justin or are in DMs with him. If the manager's Slack handle isn't available, search Slack users by the manager's name to find it.

### Calendar — today's schedule

- Fetch the calendar events between 2 days ago and 3 days from now, including attendees.

### Vault — active work context

- Fetch Justin's Vault contributions since the 3-day cutoff — active Vault projects and recent contributions. Use this to identify which projects he is currently driving.

## Step 3: Identify unaddressed items

Cross-reference the raw data to find what actually needs attention. An item is "unaddressed" when:

- **PRs I authored:** someone commented or requested changes in the last 3 days and Justin hasn't pushed a commit or reply since.
- **PRs awaiting review:** requested >24h ago with no review from Justin, or the author pinged again after a prior review.
- **Issues:** assigned to Justin with no linked PR or recent comment from him.
- **Slack threads:** someone asked Justin a question or mentioned him, and his user ID does not appear in the replies after the mention.
- **Manager messages:** any DM or mention from the manager without a reply from Justin in the same thread.
- **Saved items:** due today or overdue.

Skip items where Justin has clearly already responded or where the thread has since concluded.

## Step 4: Produce the briefing

Output in this exact structure. Use GitHub-flavored markdown. No emojis unless Justin's tone elsewhere uses them.

```
# Morning focus — <YYYY-MM-DD>

## Top 3
1. **<Title>** — <one-line reasoning: who's blocked, what's stale, why it matters>. <link>
2. ...
3. ...

## Calendar today
- <HH:MM–HH:MM> <title> — <note if prep needed or if it eats a focus block>
- ...

## PRs — mine
- <status emoji-free tag like [stale 3d]> <title> (<repo>#<number>) — <what's needed from me>
- ...

## PRs — awaiting my review
- <age> <title> (<repo>#<number>) by <author> — <why it matters>
- ...

## Issues
- <title> (<repo>#<number>) — <next action>
- ...

## Slack — unaddressed
- **<channel or DM>** — <one-line summary of what's unanswered> (from <person>, <age>)
- ...

## From <manager name>
- <one-line summary> (<age>, <link or channel>)
- ... (omit this section entirely if nothing)

## Active Vault projects
- <project title> — <current phase/status, recent activity note>
- ...
```

### Rules for the top 3

- Pick from across sections — the top 3 should reflect real priorities, not one source.
- Each item's reasoning must name a concrete consequence: "<person> is blocked", "release cutoff is <date>", "production issue", "stale 5d and legal needs a decision", etc.
- If a manager message is unaddressed, it is almost always in the top 3 unless clearly FYI.
- If there are fewer than 3 genuinely important items, say so rather than padding.

### Rules for the sections

- **Omit any section that has no items.** An empty "## Issues" section is noise.
- Cap each section at ~7 items. If there are more, list the 7 most recent or highest-priority and add a `- … and N more` line.
- Age format: `<Nd>` for days, `<Nh>` for hours. Use the most recent activity time, not creation time.
- Link format: use the `[text](url)` markdown link style so the output is clickable in IDE and terminal.
- **Pull request links: use Graphite URLs**, not github.com. Format: `https://app.graphite.com/github/pr/<owner>/<repo>/<number>`. This applies to every PR reference in the briefing — "PRs — mine", "PRs — awaiting my review", and any PR mentioned inline in top-3 reasoning or Slack context. Use github.com links only for issues and non-PR references.

## Step 5: Offer next actions

After the briefing, ask Justin one short question: whether he wants to dive into item #1, see more context on a specific item, or dismiss the briefing. Do not auto-start any follow-up work.
