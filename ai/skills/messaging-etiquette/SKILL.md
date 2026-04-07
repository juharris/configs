---
name: messaging-etiquette
description: Use this skill when sending messages on behalf of the user via Slack or other messaging platforms. Covers AI disclosure, tone, and formatting guidelines. Also covers GitHub PR commenting etiquette.
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

## Typing Indicator

Before composing a reply, add a :robot_face: reaction to the message you are responding to. This signals to the recipient that an AI-assisted reply is being drafted. After sending the reply, remove the :robot_face: reaction.

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

## GitHub Pull Requests

- **Never post PR-level comments** (i.e. never use `gh pr comment`). PR-level comments are noise.
- **Always use line-level review comments** that point to specific code in the diff. Use `gh api` to post review comments on specific lines.
- If there is no specific line to comment on, then either comment on the first changed file, but not a specific line and reconsider whether the comment is necessary at all.
