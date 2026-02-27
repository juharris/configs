---
name: review-pr
description: Review pull requests using personal guidelines
argument-hint: [pr-number]
allowed-tools: Read, Grep, Bash(gh *)
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

To find the right line number: read the file with `Read` or check `gh pr diff` output, count the lines in the new file version, and confirm the number before posting.
