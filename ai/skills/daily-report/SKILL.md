---
name: daily-report
description: Summarize work done in the last day from authored pull requests and Slack activity.
allowed-tools: Bash(gh *), Bash(date *), Bash(sleep *), mcp__playground-slack-mcp__get_messages(*), mcp__playground-slack-mcp__get_user_profile(*), mcp__playground-slack-mcp__get_reactions(*)
---

Check for unanswered messages from Geekbot in the DM channel and answer them.

## Step 1: Read the conversation and gather work data in parallel

Do these in parallel:
1. Read the last 3 messages from Geekbot DM channel to find the most recent unanswered question.
2. Start gathering today's work data immediately (PRs merged, PRs opened, PRs reviewed, Slack activity) using the /weekly-summary skill approach but scoped to today only.

## Step 2: Determine which question is unanswered

Analyze the conversation to find the FIRST Geekbot question that has NO user reply after it:
- Look for non-Geekbot messages between Geekbot's questions to identify already-answered questions.
- Only respond to the first unanswered question.
- Do NOT re-answer questions the user already responded to.

## Step 3: Answer the unanswered question

### "How did you feel today?" — SKIP, do not answer
This question uses interactive buttons in Slack. Tell the user to pick their answer in Slack, then re-run the skill. Do NOT send a text message for this question.

### "What did you work on today?" — Answer with work summary
Use the work data gathered in Step 1 to compose a summary.
- Do not use Markdown formatting — use plain text with typical Slack formatting (bullets with •).
- Put the most impactful highlights first at the top.
- Include: merged PRs, open PRs, notable PR reviews, important Slack discussions.
- Emphasize WHY the work matters, not just what was done.

### "Is there anything blocking your progress?" — Answer with just a dash
Send: `-`

### "Thanks Justin! You are the best!" — STOP, do not respond
This is Geekbot confirming the report is published. Do not reply.

## Step 4: Wait and check for follow-up questions

After sending a response, wait 10 seconds using `sleep 10` via Bash, then check the channel again for the next question. Repeat Steps 2-4 until Geekbot says thanks or there are no more unanswered questions.