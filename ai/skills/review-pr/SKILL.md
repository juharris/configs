---
name: review-pr
description: Review pull requests using personal guidelines
disable-model-invocation: true
argument-hint: [pr-number]
allowed-tools: Read, Grep, Bash(gh *)
---

# Pull Request Review

Review PR #$0 following the guidelines in `~/workspace/configs/ai/code-review-guidelines.md`.

Read that file first, then:

1. Fetch PR info with `gh pr view $0` and `gh pr diff $0`
2. Check for existing comments in the PR with `gh pr comments $0`
3. Review according to your guidelines.
4. Provide structured feedback with specific line references.
5. Add comments or respond to existing comments if they are relevant to the review. Always make it clear that you are AI by prefixing comments with "AI: ".
