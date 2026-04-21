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

Slack's native mrkdwn uses `<url|anchor text>` for links, not GitHub-style `[text](url)`. The MCP `send_message` tool accepts GitHub markdown via its `markdown_text` parameter and converts it before posting; raw `chat.postMessage` / `chat.update` do no conversion. The rules below follow from that distinction:

- Sending a **new** message via the MCP `send_message` tool (`markdown_text` param): `[text](url)` works because the tool rewrites it to `<url|text>` before calling the API. Plain URLs and `<url|text>` also work.
- Sending a **new** message via raw `chat.postMessage` (the `text` field): use Slack's native `<url|text>` or a plain URL. Do NOT use `[text](url)` — the URL will auto-link but the `[text]` brackets will render as literal characters alongside the bare URL.
- **Editing** an existing message (`chat.update` with the `text` field): no conversion runs.
  - `<url|text>` is NOT parsed — the angle brackets are HTML-escaped and show up literally.
  - `[text](url)` is NOT parsed — the brackets render as literal characters.
  - **Plain URLs DO auto-link on edit.** If you don't need anchor text, just use the bare URL.
  - **If you need anchor text on edit**, pass a Block Kit `blocks` payload with an explicit `link` element:

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

### Slack bold/italic formatting

- Sending a **new** message via the MCP `send_message` tool (`markdown_text` param): the MCP converts GitHub-style markdown to Slack's native mrkdwn. `**bold**` becomes bold, `_italic_` stays italic. Beware: `*single-star*` is converted to **italic** (not bold), because the MCP maps `*` → `_`. If you want bold, use `**bold**` on send.
- **Editing** an existing message (`chat.update` with the `text` field): no conversion runs. The text must already be in Slack's native mrkdwn:
  - `*bold*` (single asterisks) — bold
  - `_italic_` — italic
  - `**bold**` renders as literal `**bold**` with the asterisks visible
- When editing a message you previously sent with `**bold**`, you MUST rewrite it as `*bold*` or the asterisks will show up in the client. The failure is silent — the API returns `ok: true` but the rendered message is broken.

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
- If there is no specific line the comment applies to, first reconsider whether the comment is necessary at all. If it is, attach it to the first changed file at the file level (not a specific line).
