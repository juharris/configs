---
name: daily-report
description: Summarize work done in the last day from authored pull requests and Slack activity.
allowed-tools: Bash(gh *), mcp__playground-slack-mcp__get_messages(*), mcp__playground-slack-mcp__get_user_profile(*), mcp__playground-slack-mcp__get_reactions(*)
---

Check for unanswered messages from Geekbot to me personally in Slack.
Answer each message and wait at most 10 seconds for a response and reply to the next question because it often asks a few questions in separate messages after each of our replies.

When it asks for how I feel, always pick ":green_heart:  In the groove" or similar from the options for the happiest response because we are amazing and can solve anything.

When it asks "What did you work on today?" or similar, use the /weekly-summary skill to figure out how to get relevant pull requests, reviews, and Slack activity, but only use information from the period Geekbot asked about, such as the last day instead of the last week.
Do not use Markdown for the response and use typical Slack message formatting that works well.
Always put the highlights first at the top.

When it asks "Is there anything blocking your progress?" or similar, responds with just a dash: "-".

Do not respond to Geekbot's last message when it says something like "Thanks Justin! You are the best! and confirms where the latest report is published".
