---
name: peer-feedback
description: Gather evidence of interactions with a peer and draft structured peer feedback with ratings and written responses.
argument-hint: <person's name>
---

# Peer Feedback

Draft peer feedback for **$0** by gathering evidence of interactions and producing thoughtful, specific responses.

## Overview

Peer feedback forms typically include:
- **Rating questions** (scaled 1–5) covering craft excellence, ownership of outcomes, trust/collaboration, and execution speed — each with an optional note
- **A team-fit question** about how strongly you'd advocate to work with this person
- **A craft impact narrative** asking how they applied their skills over the review period with specific examples
- **A growth narrative** asking what would unlock even greater impact

Your job is to gather as much evidence as possible, then draft responses grounded in real interactions.

## Step 1: Identify the Person

Look up the person to confirm their identity and role. Gather:
- Their full name, title, team, and discipline
- Their GitHub handle
- Their Slack user ID

## Step 2: Gather Evidence of Interactions

Search broadly across all available channels to find evidence of working together. Cast a wide net — the more specific examples, the better the feedback.

### GitHub Interactions

Search for pull requests, code reviews, issues, and discussions where both of you interacted:

- **PRs they authored that I reviewed or commented on:**
  ```
  gh search prs --author <their-github-handle> --reviewed-by @me --json repository,title,url,number,mergedAt --limit 50
  gh search prs --author <their-github-handle> --commenter @me --json repository,title,url,number,mergedAt --limit 50
  ```

- **PRs I authored that they reviewed or commented on:**
  ```
  gh search prs --author @me --reviewed-by <their-github-handle> --json repository,title,url,number,mergedAt --limit 50
  gh search prs --author @me --commenter <their-github-handle> --json repository,title,url,number,mergedAt --limit 50
  ```

- **Issues and discussions where we both participated:**
  ```
  gh search issues --commenter <their-github-handle> --involves @me --json repository,title,url,number --limit 50
  ```

- For significant PRs, fetch the full details and review comments to understand the nature of the interaction:
  ```
  gh pr view <number> --repo <owner/repo> --json title,body,url,reviews,comments
  ```

### Slack Interactions

Search Slack for conversations and threads involving this person:

- Search for direct message history with the person
- Search for threads in shared channels where you both participated
- Look for messages mentioning or from this person in channels you share
- Check for threads they started that received significant engagement or that you replied to
- Look for discussions, decisions, or announcements they contributed to

### Other Sources of Interaction

- **Shared project work:** Check if you've contributed to the same projects, repositories, or initiatives
- **Pair programming or collaborative sessions:** Look for commits, branches, or discussions suggesting close collaboration
- **Technical discussions:** Look for RFC-style documents, design discussions, or architecture decisions you both participated in
- **Mentorship or knowledge sharing:** Look for instances where they helped you or you helped them learn something

## Step 3: Analyze and Categorize Evidence

Organize the gathered evidence into themes that map to the feedback dimensions:

1. **Craft excellence** — Quality of their code, technical decisions, thoroughness of reviews, depth of knowledge
2. **Ownership of outcomes** — Did they drive projects end-to-end? Did they go beyond just their assigned work?
3. **Trust and collaboration** — How they interact with others, whether they elevate the team, reliability, communication quality
4. **Execution speed** — Turnaround on reviews, pace of shipping, ability to unblock others quickly

## Step 4: Draft Feedback

### Rating Questions (1–5 scale)

For each rating dimension, suggest a score and draft an optional note grounded in specific evidence. Only include notes where you have concrete examples — it's better to skip a note than write a generic one.

### Team-Fit Question

Based on your overall experience, suggest the appropriate response for how strongly you'd advocate to have this person on your team.
Ground this in the pattern of evidence, not a single interaction.

### Craft Impact Narrative

Write a detailed response covering:
- **Specific examples** of how they applied their craft skills (reference real PRs, reviews, discussions, or projects by name)
- **Why each example matters** — connect their work to broader team or organizational impact
- **Patterns you've observed** — recurring strengths that go beyond one-off instances

Use a tone that is honest, specific, and constructive. Avoid vague praise like "they're great" — always tie it back to evidence.

### Growth Narrative

Write a thoughtful response about what would unlock greater impact. This should be:
- **Specific and actionable** — not generic advice like "keep doing what you're doing"
- **Grounded in observations** — based on patterns you've seen, not hypotheticals
- **Framed positively** — as opportunities for growth, not criticisms
- If you genuinely don't see clear areas for growth from the evidence, say so honestly and suggest areas where you lack visibility

## Output Format

Present the drafted feedback in a clear format that can be easily copied into the feedback form:

1. **Evidence Summary** — brief inventory of interactions found (PRs reviewed, Slack threads, shared projects, etc.)
2. **Rating Suggestions** — each dimension with score and optional note
3. **Team-Fit Suggestion** — recommended response with rationale
4. **Craft Impact Narrative** — full draft ready to paste
5. **Growth Narrative** — full draft ready to paste

Write the craft impact and growth narratives in markdown code blocks so they can be easily copied.

## Guidelines

- Be specific. Generic feedback is not useful.
- Every claim should be traceable to a real interaction or observation.
- If you lack evidence for a dimension, say so — it's better to skip than fabricate.
- The review period is typically the last 6 months. Focus evidence gathering on that window.
- Write in first person as if the current user is the author.
- Keep the tone professional, honest, and constructive.
