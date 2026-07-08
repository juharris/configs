---
name: review-pr
description: Review pull requests using personal guidelines
argument-hint: "[pr-number]"
---

# Pull Request Review

Review PR #$0 following the guidelines in @~/workspace/configs/ai/code-review-guidelines.md and @~/workspace/configs/ai/AGENTS.md

Read those files first, then:

1. Fetch PR info with `gh pr view $0` and `gh pr diff $0`
2. Check for existing comments in the PR with `gh pr comments $0`
3. Review according to the guidelines.
4. Provide structured feedback with specific line references.
5. Only add comments to files on specific lines or the file itself or respond to existing comments if they are relevant to the review.
  Always make it clear that you are AI by prefixing comments with how you identify yourself as AI.
  When a review comment was directly influenced by a specific hint or direction from Justin in the chat, note this in the comment (e.g., "🤖 AI Review (influenced directly by Justin): ...") to distinguish it from independently generated feedback.
  Never comment directly on the pull request and only comment in files.
  Prefer to reply to existing relevant comment threads over starting new threads in a file.
6. In the final response, include a Markdown link to the reviewed PR so Justin can open it quickly.

## Review Heuristics

Watch for repeated hash lookups where the same value is read once for a `present?` guard and then read again for the method call.
This is wasteful and noisier than assigning a local variable.
For example:
```Ruby
process(hash["context_key"]) if hash["context_key"].present?
```

Prefer:
```Ruby
context_value = hash["context_key"]
process(context_value) if context_value.present?
```

### Abstractions, Encapsulation, and Duplication

Watch for repeated type checks, union checks, or `OR` conditions that spread knowledge of related classes across the codebase.
If several call sites need to treat multiple concrete classes the same way, suggest a common interface, base class,
mixin, or protocol instead of sprinkling conditionals through unrelated files.

Watch for repeated multi-line transformations or conversions.
If the same few lines appear in multiple places, suggest encapsulating them behind a clearly named method or small
object, especially when the transformation is likely to grow to support more input types later.

### Documentation and Comments

Ask for docstrings above new classes, value objects, and methods when their purpose will not be obvious from call sites.
Do not request docstrings for when the name and usage of the class is clear enough to explain its purpose.
Do not request docstrings for private classes or methods that are only a few lines long and are easy to understand in context.
This is especially useful for small data classes because editors can show the explanation when someone hovers over the class in another file.

For Markdown and long comments, prefer sentence-per-line formatting when it keeps future diffs small.
Markdown processors usually merge adjacent lines, so splitting long sentences or paragraphs can improve maintainability without changing rendered output.

When implementation details are copied from a PR description and the code would otherwise look redundant or surprising,
suggest moving the essential reason into a nearby comment or docstring.
Do not ask for comments that merely restate obvious code.

### Reviewability and Scope

Flag PRs whose title implies a narrow change while the diff changes broader or global behavior.
Global behavior changes should be configuration-driven where possible, isolated so domain owners can review them,
and easy to revert.

If a PR changes some specific business logic for a specific feature, but also changes the core code that the team relies on such auth logic or Optify feature or configuration processing, then
request that the PR be split into two: one for the specific feature change and one for the core code change.
It's important for the team to notice core changes and not to hide them in a PR that looks like it's just about a specific feature, client, or surface.

Question large refactors that change many files when a smaller backwards-compatible wrapper would preserve the existing API.
Prefer one clear public method for common workflows unless the split materially improves correctness or readability.

Prefer small, reviewable diffs.
If a change only needs to delete a duplicate line or keep existing formatting, suggest the smaller diff instead of accepting churn that makes the behavior change harder to see.

### Testing

Question tests that mostly assert framework plumbing, configuration merging, alias resolution, or foundational behavior already covered elsewhere.
Prefer behavior-level tests at the boundary that matters for the feature.

When a test relies on math or setup intended to cross a threshold, look for an explicit precondition assertion.
For example, assert that generated fixture data actually exceeds the limit before asserting that limiting behavior occurred.

### Structured Data

When code invents a custom text format that must be parsed later such as CSV or tabs separated values, suggest a standard structured format such as
JSON Lines if ordering and appendability matter.
Custom formats are often hard to parse robustly and become expensive to maintain.

## Posting Inline Comments via GitHub API

**Always write the full JSON payload to a temp file and use `--input`** to submit reviews. Using `--field 'comments=[...]'` causes `gh api` to treat the JSON array as a string, resulting in a 422 error.

**Always post reviews as PENDING (draft), never submit them.** Justin reviews and submits the comments himself.
To create a pending review, **omit the `event` field entirely** from the payload — this leaves the review in `PENDING` state.
Do NOT pass `"event": "COMMENT"`, `"event": "APPROVE"`, or `"event": "REQUEST_CHANGES"`, as any of these submits the review immediately.

When targeting specific lines, **always use `line` + `side` instead of `position`**. `position` counts every line from the `@@` hunk header and is error-prone; `line` uses the actual file line number which is reliable.

- `side: "RIGHT"` — targets a line in the new (right-hand) version of the file
- `side: "LEFT"` — targets a line in the old (left-hand) version of the file
- `line` — the actual line number in the file (read the file or use `gh pr diff` to confirm)

Example — write the payload to a file, then submit (note: no `event` field, so the review stays pending):
```bash
cat > /tmp/pr-review.json << 'ENDJSON'
{
  "comments": [
    {
      "path": "path/to/file.ts",
      "line": 435,
      "side": "RIGHT",
      "body": "🤖 AI Review: ..."
    }
  ]
}
ENDJSON
gh api repos/OWNER/REPO/pulls/NUMBER/reviews --input /tmp/pr-review.json
```

Verify the response includes `"state":"PENDING"`. If it shows `"state":"COMMENTED"` or similar, the review was accidentally submitted — check that no `event` field was present in the payload.

Use a heredoc with `'ENDJSON'` (quoted) to prevent shell interpolation of `$`, `#`, and backticks inside comment bodies.

### Verifying Line Numbers (CRITICAL — DO NOT SKIP)

**Never guess line numbers from the diff output or mental arithmetic.**
The `gh pr diff` output includes diff headers (`diff`, `index`, `---`, `+++`, `@@`) and `+`/`-` prefixes that make it easy to miscount. Reading the raw diff and eyeballing line numbers is unreliable.

**Before posting ANY comment, export the PR file to /tmp and use `Read` to verify.**

**NEVER use `git checkout` to pull PR files into the working tree.** Always use `git show` to export to `/tmp`.

```bash
git fetch origin <commit-sha> && git show <commit-sha>:<file-path> > /tmp/pr-<filename>
```

Then use the `Read` tool on `/tmp/pr-<filename>` to see actual line numbers. Find the exact line for the comment and use that number.

**Final verification:** Before submitting the review JSON, print each `line` value and the code expected at that line to confirm they match. Do NOT skip this step.

## Pull Request Titles
Pull request titles must be concise and describe what was changed.
Example of a vague title: "small quality of life improvement for dev tools".
If a title is too vague, short, or does not clearly specific a surface area with one or more "[component]" tags or a "component:" prefix, then change the title following the Git commit guidelines for the /create-commit-message skill.
If the title was changed, then comment on the first file in the pull request to politely say why the title was changed and why clear titles are important as explained in the /create-commit-message skill and share https://www.linkedin.com/posts/justindharris_git-style-share-commit-title-guidelines-activity-7302130993946140673-xCaQ to help them learn more.
