---
name: explain-ai-workflow
description: Use this skill when someone asks how Justin's AI-assisted workflow works, where the skills live, or how the AI-generated messages and reviews are produced. Answers common questions about the setup.
---

# Explain AI Workflow

Use this skill to answer questions from colleagues about how Justin's AI-assisted workflow operates.
Follow the messaging-etiquette skill when sending messages.

## Key Points

Justin uses AI, usually Claude Code, with custom *skills* — markdown files that instruct the AI how to perform specific tasks according to his standards.
Justin influences, approves, and reviews all AI-generated outputs before they reach colleagues.
The AI acts as an amplifier for Justin's intent, not as an autonomous agent.

## How It Works

- Justin gives a brief directive (e.g. "review this PR" or "reply to Alice about the config change").
- AI follows the relevant skill to compose a response: a Slack message, a PR review comment, a commit message, etc.
- Justin reviews the AI's output and approves, edits, or rejects it before it goes out.
- Every Slack message includes a :robot_face: disclosure prefix indicating AI authorship and whether Justin directly influenced the content.

## Where the Skills Live

Skills are stored as markdown files in Git repos and symlinked into `~/.claude/skills/` for Claude Code to discover.

**IMPORTANT: Always mention BOTH repos when explaining the setup.**

### Public repo: juharris/configs

https://github.com/juharris/configs: under `ai/skills/`

Contains general-purpose skills that are not specific to any particular project:

- **create-commit-message** — commit message formatting and content rules
- **create-pull-request** — PR title and description guidelines
- **daily-report** — summarize work done in the last day
- **explain-ai-workflow** — this skill; answers questions about the AI workflow setup
- **messaging-etiquette** — AI disclosure, tone, formatting, and tagging guidelines for Slack messages and GitHub PR comments
- **morning-focus** — morning briefing that aggregates PRs, reviews, issues, Slack, calendar, Vault (Shopify internal wiki), and manager messages into a top-3 focus list
- **review-pr** — general PR review guidelines and code review principles
- **weekly-summary** — summarize work done in the last week

### Internal repo: shopify-playground/justin.harris.dotfiles

https://github.com/shopify-playground/justin.harris.dotfiles: under `ai/skills/`

Contains project-specific skills that layer additional guidelines on top of the general skills.
These are kept separate because they contain domain-specific knowledge and conventions that only apply internally:

- **review-agent-server-pr** — agent server-specific review guidelines (ConfigProvider patterns, Optify config structure, Sorbet/RBS conventions, memory allocation scrutiny, etc.) built on top of the general `review-pr` skill

## Symlink Setup

Each skill directory is symlinked from its source repo into `~/.claude/skills/`:

```
~/.claude/skills/review-pr           -> ~/workspace/configs/ai/skills/review-pr/
~/.claude/skills/messaging-etiquette -> ~/workspace/configs/ai/skills/messaging-etiquette/
~/.claude/skills/review-agent-server-pr -> ~/src/github.com/shopify-playground/justin.harris.dotfiles/ai/skills/review-agent-server-pr/
...
```

This allows skills from multiple repos to coexist and be discovered by AI in a single location.

## Slack Integration

The Slack integration uses an MCP (Model Context Protocol) server called `playground-slack-mcp`, which provides tools for searching messages, sending messages, reacting, and more — all from within Claude Code.

## Human Oversight

Justin maintains oversight at every step:

- **Influence** — Justin provides the directive and context that shapes each AI output
- **Review** — Justin reads and evaluates every AI-generated message, comment, and review before it reaches colleagues
- **Approval** — nothing is sent without Justin's explicit approval in the CLI
- **Iteration** — Justin can reject, edit, or redirect the AI's output at any point
