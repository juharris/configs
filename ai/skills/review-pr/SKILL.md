---
name: review-pr
description: Review pull requests using personal guidelines
argument-hint: "[pr-number]"
allowed-tools: Read, Grep
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

## Posting Inline Comments via GitHub API

**Always write the full JSON payload to a temp file and use `--input`** to submit reviews. Using `--field 'comments=[...]'` causes `gh api` to treat the JSON array as a string, resulting in a 422 error.

When targeting specific lines, **always use `line` + `side` instead of `position`**. `position` counts every line from the `@@` hunk header and is error-prone; `line` uses the actual file line number which is reliable.

- `side: "RIGHT"` — targets a line in the new (right-hand) version of the file
- `side: "LEFT"` — targets a line in the old (left-hand) version of the file
- `line` — the actual line number in the file (read the file or use `gh pr diff` to confirm)

Example — write the payload to a file, then submit:
```bash
cat > /tmp/pr-review.json << 'ENDJSON'
{
  "event": "COMMENT",
  "body": "",
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