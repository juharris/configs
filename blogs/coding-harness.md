# Agentic Coding Harness

Treat AI like a naive junior engineer.
As we use more intelligent AI, trust these systems to use the same documentation developers read.

Files for AI such as AGENTS.md and skills should be brief with references to documentation meant for people too.
Information for AI agents should encourage the right behavior while referring to information in README.md, CONTRIBUTING.md, files in a `docs/` folder containing documentation of a team's shared understanding which is also great for new team members to understand.

My personal AGENTS.md contains the following:

> See README.md and CONTRIBUTING.md in the current project.
> IMPORTANT: You MUST read the README.md in the current folder (or repository root) BEFORE running any build, lint, or test command.
> Do NOT guess or make up commands — use what the project documents.

Many projects duplicate documented instructions into AGENTS.md or skills which is redundant and hard to maintain.
New team members will ignore this AI oriented information because they will be overwhelmed with documentation.

## Document with Code

Over 70 year old idea.
TK Say when code comments were invented with a link to a reference on when the first comments in code were given.

Avoid too detailed documentation maintained independently of code.
Keeping documentation in the same code repository helps, if AI coding assistants know to check the documentation when the code changes.
Brief comments such as docstrings above classes and most methods to explain what they do is helpful for AI and of course for people when they're in another file and they hover over the method in their IDE.

TK For Justin: Add image of hovering over a method in IDE.

Documentation must always start with what the component does because that is what callers care about.
Ideally what methods and classes do should be obvious and it would be nice to have code with not comments.
Naming is hard.
See [another blog of mine](./naming.md) for help with naming.
This rule is similar to the common guideline to start a paragraph with a topic sentence or the main idea.
Ideally, how the component works and why it works that way should be obvious, but these comments can go within the components and usually should not be in the docstring because other developers should not be concerned with such details as it can lead to confusing coupling.

## Linting and Formatting

TK Explain the difference.

Many are are saying that it's best to have specific linters and formatting checks to help AI code.
Agentic prompting and preferring hard constraints and linters or trade-offs of comments in code is the same old advice about manual vs. automated checks.
TK Find references of those discussing linters like they're a new idea.
This can be summarized as:

> Treat naive AI as a junior engineer.