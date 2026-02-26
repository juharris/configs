---
name: create-pull-request
description: Create pull requests using personal guidelines. Use this skill to get instructions on how to create pull requests and what to include in the pull request description.
allowed-tools: Read, Grep, Bash(gh *)
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
If it cannot be found in those locations, such as in a monorepo or when the working directory is a subdirectory, the template may be at the repository root — use `git rev-parse --show-toplevel` to find the root and search there.
Always summarize what was done and why it was done.

If Graphite is available, then use `gt submit --draft --view` to create the pull request and edit the description once it is created as a draft;
otherwise use `gh pr create --draft --fill` and edit the description to include the details of what was done and why it was done.
