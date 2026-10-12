# AI Configuration

Personal configuration files for Claude Code and other AI tools.

## Contents

- `AGENTS.md`: Personal instructions loaded by AI tools in all projects (symlinked to `~/.claude/CLAUDE.md`, `~/.codex/AGENTS.md`, `~/.copilot/copilot-instructions.md`, and `~/.pi/agent/AGENTS.md`)
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

On Windows, run it from Git Bash with [Developer Mode](https://learn.microsoft.com/en-us/windows/apps/get-started/enable-your-device-for-development) enabled, or from an elevated shell.
Git Bash copies files instead of linking them by default, so the script forces native Windows symlinks and stops if it cannot create them.
Re-running the script replaces copies that are identical to the files in this repository with symlinks.

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

1. Create a directory in `skills/` with the skill name.
2. Add a `SKILL.md` file with YAML frontmatter and instructions.
3. Run the setup steps from above.
  The skill will be available as `/<skill-name>` in Claude Code.
