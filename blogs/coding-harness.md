# Agentic Coding Harness

An AI coding agent should work from the same documentation and standards as the rest of the engineering team.

> Treat an AI coding agent as a junior engineer, and give it the same documentation and guardrails that make people successful.

An agent may read and change code much faster than a person and to work effectively, it still needs context, constraints, and review.
The most useful agent instructions provide that context without creating a second, private version of the project documentation.

We should expect an agent to read the same README, CONTRIBUTING guide, architecture documentation, and tests that a developer would read.
An agent does not need a private parallel universe of instructions that restates the README, CONTRIBUTING guide, and architecture docs.
An `AGENTS.md` file or skill should tell the agent where to find those sources, which project rules matter most, and what it must verify before making a change.

My personal `AGENTS.md` contains the following:

> See README.md and CONTRIBUTING.md in the current project.
> IMPORTANT: You MUST read the README.md in the current folder (or repository root) BEFORE running any build, lint, or test command.
> Do NOT guess or make up commands; use what the project documents.

That instruction points the agent to the project's source of truth.
It also gives the same direction to a new team member who needs to learn how the project works.
Agent instructions should remain short enough to review and maintain.
They should describe durable preferences, risk boundaries, and the places where more detailed guidance lives.

For example, a skill for writing commit messages in `.agents/skills/commit-messages/SKILL.md` can point to the documentation that people already use:

```Markdown
See `docs/commit-messages.md` for guidance on writing clear and consistent commit messages.
```

When the guidance changes, one document changes.
That is easier to keep accurate than a detailed copy in every agent configuration file.

## When an Agent Gets Something Wrong

When an agent makes an unexpected change,
find out why and address the underlying issue.
First, tell it not to change anything else.
Ask it to explain why it made the change, which rule or assumption led it there, and how the mistake could have been prevented.
At this stage, the agent should answer the questions rather than immediately editing the code again.

The explanation can reveal a missing or ambiguous project rule, or perhaps conflicting guidance.
After reviewing it, improve the appropriate durable source of guidance: update a README, instruction file, design document, test, or automated check.
Commit that guidance adjacent to the code or change that exposed the problem.
This makes the lesson visible to the next person and the next agent instead of relying on one conversation to preserve it.

## Document with Code

Documentation belongs in the same repository as the code it describes so that both can evolve together.
A change to an interface, configuration format, or operational procedure should update the documentation in the same change, such as a Git commit, whenever possible.
Git preserves the relationship between a version of the code and the documentation that explained it at that time.
When investigating an old behavior, we can check out an old commit and read the corresponding documentation instead of trying to reconstruct the past from memory.

Long documents maintained separately from the code tend to become outdated due to friction to update them and they are more difficult for AI coding agents to find.
README files, CONTRIBUTING guides, docs folders, examples, tests, and focused comments provide a better shared record.
Agents can use those sources when they are explicitly directed to them, and developers can use them without learning an AI-specific workflow.

Docstrings above classes and important methods are useful because they explain a component at the point where another developer encounters it.
An editor can show the description when a developer hovers over a method from another file.
An agent can use the same description to understand the component's public purpose before reading its implementation.

TK Justin Add image from IDE.

A docstring should begin with what the component does.
Callers need the contract before they need the implementation details.
The reasons for an unusual implementation belong near the code path where those reasons matter.
This keeps the public description stable and avoids coupling callers to internal history.

Names should carry as much of the contract as possible.
Good names make comments shorter and leave comments to explain decisions that are not obvious from the code.
See [Name Code After What It Does](./naming.md) for more on this distinction.

### Keep Important Reasons Near the Code

*Avoid keeping important details exclusively in Git artifacts.*

When a reason affects how code must be changed safely, record that reason in a nearby comment, docstring, test, or design document in the repository.
Pull requests, commit messages, and issues are valuable records of decisions,
but they are difficult to find later.
Future maintainers and AI coding agents should not have to search through many old and separate commits to piece together the history behind a code path.
Link to a longer discussion when the full history matters, but do not make the linked history the only explanation available to the code's next reader.

## Memories

Agent memories must be avoided for agentic software development.
Memory is a useful and entertaining feature in applications where people chat with an agent in an website or less technical application, but that use case differs from maintaining a codebase.

Memory is opaque to the rest of the engineering team.
Coding agents typically save memory locally to one device and exclusively for the context of that agent.
Development practices become bifurcated as different rules apply for different people.
People cannot review memories in a pull request, see who changed them, or easily determine which version of the code they describe.
That opaqueness makes memory a poor place for project rules and technical decisions.

Guidelines must be reviewed and committed alongside the code they govern.
The repository gives people and agents a visible history, a review process, and a way to recover the guidance that applied to an older version of the system.
If a rule matters to future changes, record it in a tracked document, test, or automated check.

Memories are convenient because they store information without requiring a documentation change.
They also become outdated, overly specific, or wrong as the code changes.
People may forget a bad memory exists, but an agent can continue applying it because it remains in its context.
The convenience is not worth making project behavior depend on information that the team cannot inspect and maintain together.

Agents must not change memories as a routine response to a coding task.
When a conversation reveals a useful project rule, first improve the appropriate checked-in documentation or automated check.

Store personal preferences to use across projects in version control like I do in [my AI configurations](https://github.com/juharris/configs/tree/main/ai).

## Linting and Formatting

People and agents are more effective with fast, deterministic checks and rules that can be applied automatically.

Formatting and linting turn recurring review expectations into checks that run consistently for every change.
That automation lets reviewers spend their time on behavior, design, and risk instead of repeating mechanical comments.

A formatter decides how code is laid out.
A linter checks whether code violates rules about correctness, safety, dependencies, or project conventions.
Formatting should be automatic once the team has chosen its rules.
Nobody should spend review time debating whitespace, quote style, import order, or line wrapping while reviewing changes that fix bugs or add new features.

Changes to formatting rules deserve a separate, clearly described pull request.
That separation makes the proposal easy to find and review.
It prevents a formatting debate from being hidden inside a feature or bug fix.
It also avoids forcing unrelated changes to wait while the team decides whether a new rule is appropriate.
Once the team agrees, the formatter can apply the rule consistently in a separate change.

Linters enforce a different kind of agreement.
They can catch unused variables, unsafe APIs, missing dependency declarations, forbidden imports, and project-specific hazards before a reviewer has to notice them.
When a rule matters enough to repeat in review, it is worth asking whether the rule can become an automatically executed check.
Documentation can describe a preference, but deterministic checks are more effective because they can enforce it for every contributor, including an AI agent.

## Make Agents Aware of These Rules

An agent needs to understand the rules explained in this article that governs its own instructions:
the `AGENTS.md` file and skill files should remain brief, and durable project guidance should go in documentation intended for people as well as agents.
Before adding detail to an agent-specific file, it should ask whether the information belongs in the README, CONTRIBUTING guide, a docs folder, a test, or another checked-in source of shared understanding.

When an agent finds guidance that already exists in human-readable documentation,
it should link to that guidance rather than copy it into `AGENTS.md` or a skill file.
When the documentation does not exist, it should prefer adding the missing explanation to the repository's documentation and keep the agent-specific instruction limited to pointing agents there.
If the correct location is unclear, the agent should look for clarification instead of choosing the easiest file to edit.

This awareness matters most when an agent makes a mistake.
The ability to edit code does not give an agent permission to continue editing after a person has challenged its approach.