# Agentic Coding Harness

An AI coding agent should work from the same documentation and standards as the rest of the engineering team.

> Treat an AI coding agent as a junior engineer, and give it the same documentation and guardrails that make people successful.

The agent may read and change code much faster than a person and to work effectively, it still needs context, constraints, and review.
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

*Avoid Details in Exclusively in Git Artifacts*

When a reason affects how code must be changed safely, record that reason in a nearby comment, docstring, test, or design document in the repository.
Pull requests, commit messages, and issues are valuable records of decisions,
but they are difficult to find later.
It's difficult for future maintainers and AI coding agents to check many old and seperate commits to piece together history.
Link to a longer discussion when the full history matters, but do not make the linked history the only explanation available to the code's next reader.

## Linting and Formatting

People are agents are more effective with fast deterministics checks and rules that can be applied automatically.

Formatting and linting turn recurring review expectations into checks that run consistently for every change.
That automation lets reviewers spend their time on behavior, design, and risk instead of repeating mechanical comments.

A formatter decides how code is laid out.
A linter checks whether code violates rules about correctness, safety, dependencies, or project conventions.
Formatting should be automatic once the team has chosen its rules.
Nobody should spend review time debating whitespace, quote style, import order, or line wrapping while reviewing changes to fix bugs or add new features.

Changes to formatting rules deserve a separate, clearly described pull request.
That separation makes the proposal easy to find and review.
It prevents a formatting debate from being hidden inside a feature or bug fix.
It also avoids forcing unrelated changes to wait while the team decides whether a new rule is appropriate.
Once the team agrees, the formatter can apply the rule consistently in a separate change.

Linters enforce a different kind of agreement.
They can catch unused variables, unsafe APIs, missing dependency declarations, forbidden imports, and project-specific hazards before a reviewer has to notice them.
When a rule matters enough to repeat in review, it is worth asking whether the rule can become an automatically executed check.
Documentation can describe a preference, but deterministic checks are more effective because they can enforce it for every contributor, including an AI agent.
