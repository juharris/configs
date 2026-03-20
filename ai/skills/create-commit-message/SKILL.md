---
name: create-commit-message
description: Create commit messages using personal guidelines. Use this skill to get instructions on how to create commit messages and what to include in the commit message.
allowed-tools: Read, Grep
---

# Git Commit Messages

Clear Git commit messages and pull request titles are important to get the team's attention quickly and ensure the right people notice changes.
These guidelines are similar to and inspired by Linux Kernel and the Git project's commit message guidelines.
Proper titles are important for the Git history, Slack notifications, and email notifications where people often use tags in a title or email subject to filters notifications based on their interests.
There is more inspiration here: https://cbea.ms/git-commit/

## Before Committing

Never commit directly to the default branch (e.g. `main`, `master`). If on the default branch, create a new branch first:
- In shared repositories, prefix the branch name with `jus/`.
- Use `gt create <branch-name> --message '<commit message>'` when Graphite is available, otherwise use `git switch --create <branch-name>`.

## Format

Follow these commit title conventions:

## Format

- Use `[component]` tags for changes within a specific component or feature area
- Keep titles under 120 characters
- Use imperative mood (command form): "Add", "Fix", "Remove", "Update", "Refactor"

## Imperative Verb Examples:

- **Add** - new feature or file
- **Enable/Disable** - toggle features or flags

## Examples

Good:

- `[config][pulse] Set max num artifacts to 3`
- `[db][conversations] Add scenario column`

## Attribution

Read the user's `~/.claude/settings.json` file and look for the `attribution.commit` field.
If it exists, append it to the end of the PR body/description.
