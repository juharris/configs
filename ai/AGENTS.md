# The Current User

The user is an expert software developer with over 15 years of experience writing code.
They are proficient in Python, JavaScript, TypeScript, Node.js, Ruby, Rust, Java, C# & .NET, HTML, and related languages.
The user embraces configuration driven development with declarative configurations and avoid complex conditional logic with imperative code unless the code is executing the logic desired by the configuration.

# Code Style

Prefer encapsulation and re-use over duplication such as copying existing code.
Instead of copying code, encapsulate the code in a method or class and re-use that method or class where necessary.
Executives such as the CEO and CTO of the large multi-national corporation where the user works reviews all of their code so the code needs to be robust and clear.
Directors and colleagues often use the user's code as examples for how to write code in the company, so the user needs to set a good example with their code, Git branch names, and Git commit messages.

# Code Comments

Use comments to explain "WHY" some complex and obscure code is doing something, but do NOT use comments to explain "what" the code is doing.
What the code is doing should be obvious from the code itself by on the names of the methods and variables.
If the code is not obvious, use a comment to explain why it is doing something and consider encapsulating the code in a method with a name that explains what it is doing.
Comments should almost always be on the line before code instead of after code on the same line.
There are some exceptions to this rule for conventions such as Ruby RBS type hints, Python type hints, and specifying tensor dimensions.
It's much easier to be informed with comments first instead of reading confusing code and wasting time thinking about it before seeing the explanation.

Docstrings and comments above methods and classes to explain what they do are an exception to the "no what comments" rule and are encouraged.
Such comments help developers understand the purpose of a class or method at a glance without reading the implementation.
These should be encouraged, written for new classes and methods, and preserved if they are accurate.

Do not delete existing comments when making changes, unless the comments are not true anymore.
There are often existing comments that explain why the code is doing something and those comments should be preserved, unless they are no longer true.
Retain most existing comments and add new comments as necessary to explain why the code is doing something, but do not delete existing comments unless they are no longer true.

# Working Directory

Do not change the working directory unless absolutely necessary.
The `gh` CLI and `git` commands work from any subdirectory within a repository.
Prefer to stay in the current working directory even within a monorepo.
Do not `cd <repo root> && ...` unless absolutely necessary.

When using Grep, Glob, Read, or any search tool to access files within the current working directory, use relative paths or omit the path parameter entirely.
Do NOT use absolute paths to files in the current repo — absolute paths trigger unnecessary permission prompts that waste time.
Only use absolute paths for files genuinely outside the current working directory (e.g., `~/.claude/settings.json`, `/tmp/...`).

# Verifying Before Claiming

NEVER claim how a system, tool, CLI, API, or infrastructure works based on assumption or general knowledge.
Before making ANY claim about system behavior, you MUST have evidence from at least one of: reading the source code, running the command with `--help`, searching online documentation with WebSearch/WebFetch, or observing actual output.

When you don't know how something works, SEARCH ONLINE FIRST. Use WebSearch to find official documentation, blog posts, and API references.
Do not spend time speculating or exploring code when a web search would give a definitive answer faster.
This is especially important for third-party tools, infrastructure systems, gems, and platform behavior that is not defined in the current codebase.

If you still don't know after searching, say "I don't know" and explain what you searched for. Do not speculate, guess, or fabricate explanations.
Getting it wrong wastes the user's time, damages their credibility with colleagues, and can cause real harm when they act on your wrong information.

When corrected, STOP, re-read what the user said, and verify before responding again. Do not repeat the same wrong answer in different words.

# CLI Commands

NEVER guess at CLI syntax, flags, or subcommands. ALWAYS run `<command> --help` or `<command> <subcommand> --help` first and read the output before suggesting or executing any command you are not 100% certain about.
This applies to all CLIs: `gt`, `gh`, `git`, `dev`, `tec`, and any other tool.
The user's credibility depends on correct commands. A wrong flag or invented subcommand wastes time and erodes trust.

# Development Commands

@README.md

IMPORTANT: You MUST read the README.md in the current folder (or repository root) BEFORE running any build, lint, or test command.
Do NOT guess or make up commands — use what the project documents.

When the shell cannot find a documented command (e.g. `yarn`, `node`), activate the project's runtime first with `nvm use` before retrying.
Do NOT invent alternative commands — fix the environment.

# After Making Changes

Run tests and style fixes for new and changed code.

# Git Branch Names
In shared repositories, such as ones in the user's company, prefix the branch name with `jus/` to indicate that the branch is being worked on by the user.

Use `gt create <branch-name> --message '<commit message>'` to create a new branch when Graphite is available, otherwise use `git switch --create <branch-name>`.

# Amending vs. New Commits

For most repos, for small follow-up changes to an in-flight PR (review feedback, typo fixes, lint fixes, revert of a single hunk), prefer amending the existing commit via `gt modify` (or `git commit --amend` + `git push --force-with-lease`) rather than stacking a new commit on top.
This keeps World PRs to a single clean commit.

Some repos, such as optify do squash commits for pull requests, so amending is not necessary there.

# Git Commit Messages

IMPORTANT: You MUST invoke the /create-commit-message skill BEFORE writing any commit message such as when using `git commit`.
Do NOT write commit messages from memory or general knowledge — always invoke the skill first to get the exact formatting rules.

# Creating Pull Requests

IMPORTANT: You MUST invoke the /create-pull-request skill BEFORE creating any pull request such as when using `gh pr create` or `gt submit`.
Do NOT write PR titles or descriptions from memory or general knowledge — always invoke the skill first to get the exact formatting rules.

After any follow-up commit, amend, squash, or push to an already-open PR, re-read the current PR body and the current diff on GitHub and update the title/description if they no longer match. See the "Keeping the PR Description in Sync After Follow-Up Commits" section of the /create-pull-request skill for the exact steps.

# Communicating on Behalf of the User

The user's reputation is on the line when you draft messages to colleagues, create issues, or post in channels.
Before reporting a bug, issue, or system behavior to anyone:
- Verify your understanding by reading the actual code, not by guessing at how systems connect.
- Confirm with evidence (queries, code paths, test output) that what you're claiming is true.
- If you're uncertain, say so explicitly rather than presenting speculation as fact.
Getting this wrong wastes other people's time and damages the user's professional reputation.

IMPORTANT: You MUST invoke the /messaging-etiquette skill BEFORE sending any message on the user's behalf — Slack DMs, Slack channel posts, GitHub comments, GitHub PR review comments, or any other outbound communication.
Do NOT improvise AI disclosure text, attribution format, or formatting from memory — always invoke the skill first to get the exact required prefix (e.g. `🤖 _This message was written by AI on behalf of Justin._`) and the other etiquette rules.