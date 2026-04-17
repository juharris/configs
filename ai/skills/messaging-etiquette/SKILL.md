---
name: messaging-etiquette
description: Use this skill when sending messages on behalf of the user via Slack or other messaging platforms. Covers AI disclosure, tone, and formatting guidelines. Also covers GitHub PR commenting etiquette.
---

# Messaging Etiquette

Follow these rules whenever sending messages on behalf of the user.

## AI Disclosure

Always identify the message as AI-generated.
Prefix every message with a robot emoji and disclosure text.
When the content of a message was directly influenced by a specific hint or direction from Justin in the chat, use the "influenced directly" variant. This distinguishes messages where Justin actively shaped the content from those where AI acted more autonomously.

**Slack examples:**

> 🤖 _This message was written by AI on behalf of Justin._

> 🤖 _This message was written by AI and influenced directly by Justin._

This applies to all messages: DMs, channel posts. Never send a message without this prefix.

**GitHub examples:**

> 🤖 *This comment was written by AI on behalf of Justin.*

> 🤖 *This comment was written by AI and influenced directly by Justin.*

This applies to all comments: new threads, replies, and PR review comments. Never send a message without this prefix.

## Typing Indicator

Before composing a reply in Slack, add a :robot_face: reaction to the message you are responding to.
This signals to the recipient that an AI-assisted reply is being drafted.
After sending the reply, remove the :robot_face: reaction.

## Tone

- Write in a professional, concise tone appropriate for the channel context.
- Do not use excessive emoji beyond the required 🤖 prefix.
- Prefer speaking in the active voice.
- Use a vast vocabulary to impress colleagues and teach them new terms.

## Formatting

- In Slack, use markdown formatting (bold, italic, code blocks, links).
- Keep messages brief and to the point.
- When sharing links (PRs, issues, docs), include a short description of what the link is. Try to save people time so that they don't all need to click on the link to understand why it's relevant.

### Slack link formatting

- Sending a **new** message (`chat.postMessage` / `send_message`): use markdown `[text](url)` or Slack's `<url|text>` — both render as clickable links. The server parses the text into link blocks automatically.
- **Editing** an existing message (`chat.update`): the server does NOT re-parse `<url|text>` syntax. Passing plain text with that syntax will display the raw angle-bracket string in the client. To get a working link on edit, you MUST pass a Block Kit `blocks` payload with an explicit `link` element:

  ```json
  {
    "type": "rich_text",
    "elements": [{
      "type": "rich_text_section",
      "elements": [
        { "type": "text", "text": "See " },
        { "type": "link", "url": "https://...", "text": "this post" },
        { "type": "text", "text": " for details." }
      ]
    }]
  }
  ```

  Also pass a `text` fallback for notifications/accessibility, but the `blocks` payload is what renders.

### Linking to a Slack message in a thread

Use the canonical permalink format: `https://{workspace}.slack.com/archives/{channel_id}/p{ts_without_dot}?thread_ts={root_ts}&cid={channel_id}`.
- `ts_without_dot` is the reply message's timestamp with the `.` removed (16 digits).
- `thread_ts` is the thread root message's timestamp (with the dot).
- Without `thread_ts` and `cid`, the link may fail to open the correct thread context.
- When embedding the URL in a Slack message being sent as raw text, leave the `&` as-is (don't HTML-encode to `&amp;`).
- When in doubt, call `chat.getPermalink` to get the canonical URL — but note that some enterprise workspaces restrict this API, in which case construct the URL manually using the pattern above.


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
