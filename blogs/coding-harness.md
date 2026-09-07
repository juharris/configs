# Agentic Coding Harness

TK Fix this terrible introduction. Who has infinite patience and uneven judgment? The first sentence of every paragraph, especially the first one needs to be amazing.
Treat AI coding agents like naive junior engineers with infinite patience and uneven judgment.
That framing keeps the tooling practical.
It avoids both panic and magical thinking.

We should trust AI systems to read the same documentation developers read.
An agent does not need a private parallel universe of instructions that restates the README, CONTRIBUTING guide, and architecture docs.
It needs a short harness that tells it where to look, what standards matter most, and which mistakes carry the highest cost.

My personal `AGENTS.md` does this with a small rule:

> See README.md and CONTRIBUTING.md in the current project.
> IMPORTANT: You MUST read the README.md in the current folder (or repository root) BEFORE running any build, lint, or test command.
> Do NOT guess or make up commands; use what the project documents.

That instruction works because it points the agent back to the shared source of truth.
The same documentation helps people join the project, review changes, and debug production behavior.
When teams duplicate all of that into agent-specific files, they create another surface area in danger of becoming outdated and misleading.
New developers will not read every AI prompt file.
Experienced developers should not need to read every AI prompts file.

Agent instructions should stay brief.
They should encode durable preferences, risk boundaries, and routing rules.
They should link to human documentation for project-specific detail.

TK Improve example.
For example, `.agents/skills/commit-messages/SKILL.md` could contain the following:

```Markdown
See `docs/commit-messages.md` for guidance on writing clear and consistent commit messages.
```

## Document with Code

TK Add a part somewhere about how documentation needs to be in the same repository as the code so that it can be maintained as the code changes. It's easy to see the old documentation for old code by going back to old commits.

Keep important documentation near the code it explains.
That advice predates modern agentic coding by decades, and AI makes it more valuable.
TK This wording is so awkward. It's not my style and looks clumsy, like it was written for a tweet to rage bait people. These blog posts are real and serious and need robust writing to convey the depth and importance of the topic.
Agents work better when they can find intent in the same repository as the implementation.
Humans do too.

Avoid long documents that describe code from a distance and slowly drift away from reality.
Use the repository for shared understanding: README files, CONTRIBUTING guides, docs folders, examples, tests, and comments where they help.
Then teach agents to consult those sources before they act.

Docstrings above classes and important methods can carry a lot of weight.
They help a developer understand a component from another file, especially when an editor shows the docstring on hover.
They help agents choose the right abstraction without crawling through every implementation detail.

Start those docstrings with what the component does.
Callers care about the contract first.
Implementation details and historical reasons usually belong inside the component, near the code path where they matter.
That structure reduces accidental coupling.

Names should do as much work as they can.
Good names make comments shorter and more focused.
See [Name Code After What It Does](./naming.md) for the naming side of this argument.

### Avoid Details in Exclusively in Git Artifacts

TK Add more details and a good topic sentence.
It's tempting to put reasons in pull request descriptions, commit messages, issues, but developers and AI will likely not dig that deeply most of the time to find context.

## Linting and Formatting

TK The first line and first paragraph must explain the thesis for this section.

Formatters decide how code looks.
Linters decide whether code violates rules the team cares about.
Both tools matter because they turn subjective review comments into repeatable checks.

Formatting should be clear and automatic.
Nobody should spend review energy debating whitespace, quote style, import order, or line wrapping after the team has chosen a formatter.
The tool should decide, and the pull request should stay focused on behavior.

If there is a desire to modify the formatting rules, then changes must be discussed in a separate and clear pull request to help other clearly find and notice the proposal.
TK Add more reasons why it's important to keep the change separate such as not blocking changes like new features or bug fixes.

Linting should catch patterns that humans miss or should not need to repeat in every review.
A linter can reject unused variables, unsafe APIs, missing dependency declarations, forbidden imports, and project-specific hazards.
The strongest rules make bad code impossible to merge instead of asking reviewers to notice it every time.

TK This is a terrible first sentence for a paragraph because every first sentence must start with a clear topic that sets the stage for the rest of the paragraph. This is also click bait garbage that does not belong in a blog post especially so deep.
AI does not change that tradeoff.
It makes the old lesson louder.
Prompts can guide an agent, but automated checks constrain it.
If a rule matters enough to repeat in review, consider making it executable.

This is the whole harness in one sentence:

> Treat naive AI as a junior engineer, and give it the same documentation and guardrails that make people successful.
