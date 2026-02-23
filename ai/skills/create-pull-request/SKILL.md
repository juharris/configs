---
name: create-pull-request
description: Create pull requests using personal guidelines. Use this skill to get instructions on how to create pull requests and what to include in the pull request description.
allowed-tools: Read, Grep, Bash(gh *)
---

# Git Commit Messages

Use the /create-commit-message skill to get instructions on how to create commit messages and what to include in the commit message.

# Creating Pull Requests

Make sure to push the branch before creating the pull request.

Use the above Git commit message conventions for the pull request title.

Always make the pull request as a draft.

If a pull request template is available in the repository, then always use it and fill in the details.
The pull request template is normally in the `.github/pull_request_template.md` file in the repository or in `.github/PULL_REQUEST_TEMPLATE/`.
Always summarize what was done and why it was done.

If Graphite is available, then use `gt submit --draft --view` to create the pull request and edit the description once it is created as a draft;
otherwise use `gh pr create --draft --fill` and edit the description to include the details of what was done and why it was done.
