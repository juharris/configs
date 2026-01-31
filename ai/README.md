# AI Configuration

Personal configuration files for Claude Code and other AI tools.

## Contents

- `AGENTS.md`: Personal instructions loaded by Claude Code in all projects (symlinked to `~/.claude/CLAUDE.md`)
- `code-review-guidelines.md`: Guidelines for the `/review-pr` skill
- `skills/`: Custom Claude Code skills
- `setup.sh`: Script to create symlinks for a new machine

## Setup on a New Machine

### 1. Clone or copy this directory

```bash
mkdir -p ~/workspace/configs
# Copy or clone this ai/ directory to ~/workspace/configs/ai/
```

### 2. Run the setup script

```bash
~/workspace/configs/ai/setup.sh
```

### 3. Verify the setup

```bash
ls -la ~/.claude
```

## Usage

In any repository with Claude Code:

```
/review-pr 123
```

This will review PR #123 using your personal guidelines.

## Adding New Skills

1. Create a directory in `skills/` with the skill name
2. Add a `SKILL.md` file with YAML frontmatter and instructions
3. The skill will be available as `/<skill-name>` in Claude Code
