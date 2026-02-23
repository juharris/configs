---
name: create-commit-message
description: Create commit messages using personal guidelines. Use this skill to get instructions on how to create commit messages and what to include in the commit message.
allowed-tools: Read, Grep, Bash(gh *)
---

# Git Commit Messages

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

## Commit Author

Do not add "Co-Authored-By: Claude ..." to commit messages because the user is directing the changes and reviewing all of the change closely and is responsible for all of the changes.
