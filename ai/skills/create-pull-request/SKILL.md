---
name: create-pull-request
description: Create pull requests using personal guidelines. Use this skill to get instructions on how to create pull requests and what to include in the pull request description.
allowed-tools: Read, Grep
---

# Git Commit Messages

When asked to make a pull request, first check for the current branch and staged changes.
Do not commit directly to main (unless explicitly asked); instead, make a new branch first:
In shared repositories, such as ones in the user's company, prefix the branch name with `jus/` to indicate that the branch is being worked on by the user.

Use `gt create <branch-name> --message '<commit message>'` to create a new branch when Graphite is available, otherwise use `git switch --create <branch-name>`.

Use the /create-commit-message skill to get instructions on how to create commit messages and what to include in the commit message.

## Creating Pull Requests

Make sure to push the branch before creating the pull request.

Use the /create-commit-message skill conventions for the pull request title.

Always make the pull request as a draft.

## Pull Request Description

If a pull request template is available in the repository, then always use it and fill in the details.
The pull request template is normally in the `.github/pull_request_template.md` file or in `.github/PULL_REQUEST_TEMPLATE/`.
IMPORTANT: Always read the template directly using the Read tool — do NOT use Glob to search for it, as glob patterns can silently miss exact filenames.
1. Run `git rev-parse --show-toplevel` via Bash to get the absolute repo root path.
2. Read `<repo_root>/.github/pull_request_template.md` using the Read tool.
3. If that file doesn't exist, try reading files in `<repo_root>/.github/PULL_REQUEST_TEMPLATE/` directory.
Always summarize what was done and why it was done.
Do not hard-wrap lines in the PR description. GitHub renders markdown and handles wrapping automatically.

## Attribution

Read the user's `~/.claude/settings.json` file and look for the `attribution.pr` field.
If it exists, append it to the end of the PR body/description.

## Requesting Reviews

Never guess a person's GitHub handle. GitHub handles are almost never derivable from a person's name.
GitHub aliases can be found in the Git history and most people set their names in their GitHub profile.
For Shopify repos, the company's internal Vault tools can be used to find a person by their email or name and then read the GitHub field from the response before requesting a review with `gh pr edit --add-reviewer`.

## Submitting

If Graphite is available, then use `gt submit --draft --view` to create the pull request and edit the description once it is created as a draft;
otherwise use `gh pr create --draft --fill` and edit the description to include the details of what was done and why it was done.

## Keeping the PR Description in Sync After Follow-Up Commits

After any additional commit or push to an already-open PR (review feedback, squashes, reverts, amends, etc.), always:

1. Re-fetch the current PR body with `gh pr view <number> --repo <owner>/<repo> --json body -q .body` so you see what is actually on GitHub (not what you originally drafted as there are often changes in GitHub).
2. Re-read the PR's current diff (e.g. `git diff origin/main...HEAD --stat` or `gh pr diff <number> --repo <owner>/<repo>`) so the description describes the code that now exists, not what existed at initial submission.
3. If the Summary or Test plan no longer matches the diff (a feature was reverted, scope shrunk, new files were added, etc.), update the PR body with `gh pr edit <number> --repo <owner>/<repo> --body "..."`.
4. If the PR title no longer describes the diff, update it with `--title` in the same `gh pr edit` call.

This applies equally whether the commit was added via `gt modify`, `gt squash`, `git commit --amend`, or a new commit — if the diff on GitHub has changed, the description probably needs to change too.

## Reporting Back to the User

This is about your chat reply to the user, not the PR body itself.

### If Graphite was used

If — and only if — the pull request was created with `gt submit`, conclude your response to the user with the Graphite link as the very last line.
`gt submit` prints a URL like `https://app.graphite.com/github/pr/<owner>/<repo>/<number>` — capture that URL from the submit output and end the message with it (plain URL or a markdown link is fine).

### If Graphite was not used

Conclude with a github.com link.
Do not include a Graphite link when the PR was created with `gh pr create` or any other non-Graphite flow; do not construct a Graphite URL from the repo slug and PR number.
