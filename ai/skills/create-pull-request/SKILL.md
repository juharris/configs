---
name: create-pull-request
description: Create pull requests using personal guidelines. Use this skill to get instructions on how to create pull requests and what to include in the pull request description.
allowed-tools: Read, Grep
---

# Git Commit Messages

When asked to make a pull request, first check for the current branch and staged changes.
Do not commit directly to main; instead, make a new branch first:
In shared repositories, such as ones in the user's company, prefix the branch name with `jus/` to indicate that the branch is being worked on by the user.

Use `gt create <branch-name> --message '<commit message>'` to create a new branch when Graphite is available, otherwise use `git switch --create <branch-name>`.

Use the /create-commit-message skill to get instructions on how to create commit messages and what to include in the commit message.

# Creating Pull Requests

Make sure to push the branch before creating the pull request.

Use the above Git commit message conventions for the pull request title.

Always make the pull request as a draft.

If a pull request template is available in the repository, then always use it and fill in the details.
The pull request template is normally in the `.github/pull_request_template.md` file or in `.github/PULL_REQUEST_TEMPLATE/`.
IMPORTANT: Always read the template directly using the Read tool — do NOT use Glob to search for it, as glob patterns can silently miss exact filenames.
1. Run `git rev-parse --show-toplevel` via Bash to get the absolute repo root path.
2. Read `<repo_root>/.github/pull_request_template.md` using the Read tool.
3. If that file doesn't exist, try reading files in `<repo_root>/.github/PULL_REQUEST_TEMPLATE/` directory.
Always summarize what was done and why it was done.

## Attribution

Read the user's `~/.claude/settings.json` file and look for the `attribution.pr` field.
If it exists, append it to the end of the PR body/description.

## Submitting

If Graphite is available, then use `gt submit --draft --view` to create the pull request and edit the description once it is created as a draft;
otherwise use `gh pr create --draft --fill` and edit the description to include the details of what was done and why it was done.
