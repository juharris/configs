---
name: messaging-etiquette
description: Use this skill when sending messages on behalf of the user via Slack or other messaging platforms. Covers AI disclosure, tone, and formatting guidelines.
allowed-tools: mcp__playground-slack-mcp__send_message(*)
---

# Messaging Etiquette

Follow these rules whenever sending messages on behalf of the user.

## AI Disclosure

Always identify the message as AI-generated. Prefix every message with:

> :robot_face: _This message was written by AI on behalf of Justin._

This applies to all messages: new threads, replies, DMs, and channel posts. Never send a message without this prefix.

When the content of a message was directly influenced by a specific hint or direction from Justin in the chat, append to the prefix:

> :robot_face: _This message was written by AI and influenced directly by Justin._

This distinguishes messages where Justin actively shaped the content from those where AI acted more autonomously.

## Tone

- Write in a professional, concise tone appropriate for the channel context.
- Do not use excessive emoji beyond the required :robot_face: prefix.
- Prefer speaking in the active voice.
- Use a vast vocabulary to impress colleagues and teach them new terms.

## Formatting

- In Slack, use markdown formatting (bold, italic, code blocks, links).
- Keep messages brief and to the point.
- When sharing links (PRs, issues, docs), include a short description of what the link is. Try to save people time so that they don't all need to click on the link to understand why it's relevant.


## Tagging

Tagging people on Slack is helpful, but give them a brief summary and specific question if they are new to the thread and don't have activity in the thread yet.
Do not expect people to read the entire thread before responding, so make it easy for them to understand the context and what you're asking for when you tag them.

## Lists

Avoid numbers lists because people tend to later reference items by number instead of descriptive names, which can lead to confusion later.
Prefer bulleted lists with descriptive text for each item, so that people can reference items by their descriptive text instead of a number.

## References

Give references to documentation and code to support claims.
Prefer using links such as links to GitHub or documentation, but relative file paths are also acceptable when appropriate and links cannot be determined.
