---
name: messaging-etiquette
description: Use this skill when sending or drafting messages on behalf of the user via Slack, other messaging platforms, or making social posts. Covers AI disclosure, tone, and formatting guidelines. Also covers GitHub PR commenting etiquette.
---

# Messaging Etiquette

Follow these rules whenever sending messages on behalf of the user.

## Drafts vs Sent Messages

When the user asks to "draft" a Slack message, create a native Slack draft using `tool-gateway`'s `slack_send_message_draft` MCP tool — do NOT just write proposed text in the chat conversation.
A real Slack draft appears in the compose box of the correct channel or thread, where the user can review, edit, and send it themselves.

- "draft a message", "write a draft", "make a draft" → use `tool-gateway`'s `slack_send_message_draft`
- "send a message", "reply to X", "post in #channel" → use `playground-slack-mcp`'s `send_message`
- When in doubt, prefer creating a draft. The user can always send a draft, but cannot un-send a sent message.

All disclosure, tone, formatting, and linking rules below apply to drafts as well, since the draft is what becomes the sent message.
Skip the `:robot_face:` typing-indicator reaction for drafts — drafts can sit unsent for a long time, and the reaction would falsely signal an imminent reply.
Add the reaction only when actually sending.

## AI Disclosure

Always identify the message as AI-generated.
Prefix every message with a robot emoji and disclosure text.
When the content of a message was directly influenced by a specific hint or direction from Justin in the chat, use the "influenced directly" variant. This distinguishes messages where Justin actively shaped the content from those where AI acted more autonomously.

**Slack examples:**

> 🤖 _This message was written by AI on behalf of Justin._

> 🤖 _This message was written by AI, but influenced directly and reviewed by Justin._

This applies to all messages: DMs, channel posts. Never send a message without this prefix.

**GitHub examples:**

> 🤖 *This comment was written by AI on behalf of Justin.*

> 🤖 *This comment was written by AI, but influenced directly and reviewed by Justin.*

This applies to all comments: new threads, replies, and PR review comments. Never send a message without this prefix.

## Typing Indicator

Before composing a reply in Slack that is not a draft, add a :robot_face: reaction to the message you are responding to.
This signals to the recipient that an AI-assisted reply is being composed.
After sending the reply, remove the :robot_face: reaction.

## Tone

- Write in a professional, concise tone appropriate for the channel context.
- Do not use excessive emoji beyond the required 🤖 prefix.
- Prefer speaking in the active voice.
- Use a vast vocabulary to impress colleagues and teach them new terms.
- Do not use em dashes (—). They are not Justin's style and read as an AI tell.
  Use a period and start a new sentence, a comma, parentheses, semicolon, or a colon instead, whichever fits best.

## Formatting

- In Slack, use markdown formatting (bold, italic, code blocks, links).
- Keep messages brief and to the point.
- When sharing links (PRs, issues, docs), include a short description of what the link is. Try to save people time so that they don't all need to click on the link to understand why it's relevant.

### Slack drafts (`tool-gateway`'s `slack_send_message_draft`)

Empirically verified by sending drafts to Justin's self-DM and reading the compose box visually. Findings:

- **Success signal: `draft_id` in the response, not the `result` text.** The tool ALWAYS returns `"result": "Draft message is created. They can edit it before sending."` regardless of whether anything was written. The real signal is whether the response includes a top-level `draft_id` field:
    - Response contains `draft_id` (e.g. `Dr0B2P26S2UX`) → draft actually created and visible in Slack.
    - Response contains only `widget_id` and no `draft_id` → silent no-op despite the success-sounding text. Never assume the draft landed without checking for `draft_id`.
- **First-write-wins per channel.** While a draft exists in the target channel (typed by the user or created by a prior tool call), subsequent `slack_send_message_draft` calls are silent no-ops (no `draft_id` returned). To send a new draft, the existing compose box must be cleared first. If the response is missing `draft_id`, tell the user the existing draft is blocking the new one and ask them to clear it.
- **Backtick-wrapped user mentions trigger `invalid_blocks`.** A `message` containing `` `<@USERID>` `` (Slack mention wrapped in inline-code backticks) causes the tool to error with `execution_failed: invalid_blocks`. Verified by isolating the construct in two independent tests. If you need to discuss a mention's literal syntax in a draft, write it in prose without backticks, or use a fenced code block.
- **Both Slack mrkdwn AND GitHub-style markdown render correctly.** Confirmed in the compose box for a single combined draft:
    - Slack mrkdwn: `*single-star bold*` → bold, `_underscore italic_` → italic, `~tilde strike~` → strikethrough.
    - GitHub-style: `**double-star bold**` → bold, `[text](url)` → clickable anchor link.
    - Slack-native: `<url|anchor>` → clickable anchor, plain URLs → auto-linked, `<@USERID>` → clickable mention, `` `code` `` → inline code, fenced ```` ```block``` ```` → code block, `> line` → blockquote.
- **For drafts that contain multiple URLs, prefer the explicit `<url|label>` form for every link.** Observed empirically: when a draft mixes plain URLs across top-level bullets and indented sub-bullets (e.g. several PR/issue links in a daily standup), only the first URL was rendered as a clickable link in the compose box and the rest displayed as inert text. Switching every reference to `<url|label>` made them all render as proper anchor links. Plain-URL auto-linking is reliable for a single isolated URL, but is the wrong choice when you have a list of links — always use `<url|label>` for multi-link draft messages.
- **`*single-star*` semantics DIFFER from `send_message`:** `*foo*` in a draft renders as bold (Slack's native mrkdwn behavior). The same input via `playground-slack-mcp`'s `send_message` `markdown_text` is rewritten to `_foo_` and renders as italic. Same string, different result depending on the tool — be deliberate about which one you're using when copying content between them.

### Slack link formatting

Slack's native mrkdwn uses `<url|anchor text>` for links, not GitHub-style `[text](url)`. `playground-slack-mcp`'s `send_message` tool accepts GitHub markdown via its `markdown_text` parameter and converts it before posting; the `text` parameter is a thin pass-through to raw `chat.postMessage` and does NO conversion. `chat.update` (edits) also do no conversion and have additional escaping gotchas. The rules below follow from those distinctions:

- Sending a **new** message via `playground-slack-mcp`'s `send_message` tool (`markdown_text` param): `[text](url)` works because the tool rewrites it to `<url|text>` before calling the API. Plain URLs and `<url|text>` also work.
- Sending a **new** message via `playground-slack-mcp`'s `send_message` tool (`text` param) OR raw `chat.postMessage` via `execute_js` (`text` field): these are equivalent — no conversion. Use Slack's native `<url|text>` or a plain URL. Do NOT use `[text](url)` — the brackets render as literal characters, and the URL inside the parentheses still auto-links separately, producing output like `[text](https://example.com)` with the URL underlined.
- **Editing** an existing message (`chat.update` with the `text` field): no conversion runs, AND angle-bracket escaping kicks in for link syntax specifically.
  - `<url|text>` is NOT parsed — the angle brackets are HTML-escaped to `&lt;` / `&gt;` and show up literally. Verified.
  - `[text](url)` is NOT parsed — the brackets render as literal characters; the URL inside parentheses auto-links separately. Verified.
  - **Plain URLs DO auto-link on edit.** If you don't need anchor text, just use the bare URL.
  - **`<@USERID>` mentions DO still parse on edit** — the angle-bracket escape only affects link syntax, not mention syntax. Verified.
  - **If you need anchor text on edit**, pass a Block Kit `blocks` payload with an explicit `link` element (works on both new messages and edits — verified):

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

### `chat.postMessage` options (raw API via `execute_js`)

- **`blocks` only, no `text`:** blocks render in-channel; the `text` field is used only for notifications/accessibility and is not displayed.
- **`mrkdwn: false`:** does NOT prevent formatting when Slack auto-generates blocks from text. Verified ineffective for `*bold*`, `_italic_`, `<url|text>`, `<@USERID>`. Don't rely on it to send literal-text.
- **`parse: "full"`:** AVOID for mentions. It ESCAPES existing `<@USERID>` mentions to literal `&lt;@USERID&gt;` and does NOT auto-link naked `@username` strings. Verified.
- **`link_names: true`:** converts naked `@username` strings to clickable user mentions. Use this when you have a username string but not the user ID. Verified.
- **Restricted in this enterprise workspace** (not testable here): `chat.scheduleMessage`, `chat.meMessage`, `chat.postEphemeral`. Returned `not_allowed_token_type` / `enterprise_is_restricted`.

### `send_message` with BOTH `text` and `markdown_text`

Don't pass both — pick one. When both are provided:
- The **`text` param's content wins** (the `markdown_text` body is discarded).
- BUT the `markdown_text` path's `*` → `_` rewrite is STILL applied to the `text` content. Verified: input `*star-bold-text*` was stored as `_star-bold-text_` and rendered italic.

This produces a confusing hybrid: text from one param, semantics from the other. Pass exactly one of `text` or `markdown_text`.

### Slack bold/italic formatting

- Sending a **new** message via `playground-slack-mcp`'s `send_message` tool (`markdown_text` param): the MCP converts GitHub-style markdown to Slack's native mrkdwn. `**bold**` becomes bold, `_italic_` stays italic. Beware: `*single-star*` is converted to italic (not bold), because the MCP maps `*` → `_`. If you want bold via `markdown_text`, use `**bold**` on send. Verified.
- Sending a **new** message via `playground-slack-mcp`'s `send_message` `text` param OR raw `chat.postMessage`: these are equivalent. Use Slack-native mrkdwn directly:
  - `*bold*` (single asterisks) — bold. Differs from `markdown_text` behavior.
  - `_italic_` — italic.
  - `~strike~` — strikethrough.
  - `**bold**` renders literally with the asterisks visible. Verified.
- **Editing** an existing message (`chat.update` with the `text` field): no conversion runs. The text must already be in Slack's native mrkdwn (same rules as raw `chat.postMessage`). `*bold*`, `_italic_`, `~strike~`, `` `code` `` all work; `**bold**` renders literally. Verified.
- When editing a message you previously sent with `**bold**`, you MUST rewrite it as `*bold*` or the asterisks will show up in the client. The failure is silent — the API returns `ok: true` but the rendered message is broken.

### Replying in a thread

- **`send_message` (`playground-slack-mcp`) with `thread_ts`** and **raw `chat.postMessage` with `thread_ts`** both work as expected — the reply lands in the thread with the parent's `thread_ts` preserved. Verified.
- **Reading thread replies back:** regular thread replies do NOT appear in `conversations.history`. Use `conversations.replies` (or `slack_read_thread`) to fetch them. The thread parent in `conversations.history` reports `reply_count` and `latest_reply` only.
- **`reply_broadcast: true`** (raw `chat.postMessage`): the reply gets `subtype: "thread_broadcast"` and appears in BOTH the channel timeline AND the thread. Use it sparingly — it's noisy. Verified.

### Unfurl behavior

- **Default (`unfurl_links:true`, `unfurl_media:true`)**: URLs that the workspace has an unfurl integration for (e.g. github.com via the GitHub app) get an `attachments` card appended to the message after a short delay.
- **`unfurl_links:false`, `unfurl_media:false`**: no `attachments` card. The URL is still clickable as a plain link. Use these flags when posting a link summary where the auto-card would be redundant or noisy. Verified.
- Unfurl is async — reading the message immediately after posting may show no attachments; reading seconds later shows them. Don't assert on unfurl content at post time.

### `chat.update` with BOTH `text` and `blocks`

Blocks win — the blocks content renders in the channel; the `text` field becomes the notification/accessibility fallback only.

The chat.update angle-bracket escape is STILL applied to the text field (`<url|text>` → `&lt;url|text&gt;` in stored text), but because blocks render, the escape is invisible to the reader. The escape would resurface if the message were later edited again with text-only.

**Practical recommendation when editing a message that needs anchor links:** always pass `blocks` with an explicit `link` element. Pass a plain-string `text` fallback (avoid `<url|text>` in the fallback to keep it clean — bare URLs auto-link).

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

Avoid numbered lists because people tend to later reference items by number instead of descriptive names, which can lead to confusion later.
Prefer bulleted lists with descriptive text for each item, so that people can reference items by their descriptive text instead of a number.

## References

Give references to documentation and code to support claims.
Prefer using links such as links to GitHub or documentation, but relative file paths are also acceptable when appropriate and links cannot be determined.

## GitHub Pull Requests

- **Never post PR-level comments** (i.e. never use `gh pr comment`). PR-level comments are noise.
- **Always use line-level review comments** that point to specific code in the diff. Use `gh api` to post review comments on specific lines.
- If there is no specific line the comment applies to, first reconsider whether the comment is necessary at all. If it is, attach it to the first changed file at the file level (not a specific line).
