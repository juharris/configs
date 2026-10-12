#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAUDE_DIR="$HOME/.claude"
CODEX_DIR="$HOME/.codex"
COPILOT_DIR="$HOME/.copilot"

AGENTS_SKILLS_DIR="$HOME/.agents/skills"
CLAUDE_SKILLS_DIR="$CLAUDE_DIR/skills"
COPILOT_SKILLS_DIR="$COPILOT_DIR/skills"

GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

SKILL_TARGETS=(
  "$AGENTS_SKILLS_DIR"
  "$COPILOT_SKILLS_DIR"
  "$CLAUDE_SKILLS_DIR"
)

CONTEXT_TARGETS=(
  "$CLAUDE_DIR/CLAUDE.md"
  "$CODEX_DIR/AGENTS.md"
  "$COPILOT_DIR/copilot-instructions.md"
  "$HOME/.pi/agent/AGENTS.md"
)

# Git Bash (MSYS2) defaults to `winsymlinks:deepcopy`, so `ln -s` silently copies instead of linking.
# Copies are not detected as links when re-running this script and do not pick up changes in this repo.
# `nativestrict` creates real Windows symlinks or fails, which requires Developer Mode or an elevated shell.
configure_windows_symlinks() {
  case "$(uname -s)" in
    MINGW* | MSYS* | CYGWIN*) ;;
    *) return 0 ;;
  esac

  export MSYS="${MSYS:+$MSYS }winsymlinks:nativestrict"
  export CYGWIN="${CYGWIN:+$CYGWIN }winsymlinks:nativestrict"

  local probe_dir
  probe_dir=$(mktemp -d)
  mkdir "$probe_dir/target"
  if ! ln -s "$probe_dir/target" "$probe_dir/link" 2>/dev/null; then
    rm -rf "$probe_dir"
    echo -e "${RED}Error: Cannot create native Windows symlinks.${NC}"
    echo -e "${RED}Enable Developer Mode (Settings > System > For developers) or run this script from an elevated shell.${NC}"
    exit 1
  fi
  rm -rf "$probe_dir"
}

# Removes a previous symlink at `target`, or a copy identical to `source` left by Git Bash's default `ln -s` behavior.
remove_previous_link() {
  local source="$1"
  local target="$2"

  if [ -L "$target" ]; then
    rm "$target"
  elif [ -e "$target" ] && diff -rq "$source" "$target" >/dev/null 2>&1; then
    rm -rf "$target"
    echo -e "${YELLOW}Removed copy of $source at $target${NC}"
  fi
}

configure_windows_symlinks

link_skills() {
  local dest_dir="$1"
  mkdir -p "$dest_dir"
  echo -e "${BLUE}Linking skills to $dest_dir${NC}"

  for skill in "$SCRIPT_DIR/skills"/*/; do
    local skill_name=$(basename "$skill")
    local target="$dest_dir/$skill_name"

    remove_previous_link "$skill" "$target"
    if [ -d "$target" ]; then
      echo -e "${YELLOW}Warning: $target exists as a directory, skipping${NC}"
      continue
    fi

    ln -s "$skill" "$target"
    echo -e "${GREEN}✓ Linked skill: $target -> $skill${NC}"
  done
}

for target_dir in "${SKILL_TARGETS[@]}"; do
  link_skills "$target_dir"
done

link_context() {
  local target="$1"
  local target_dir=$(dirname "$target")
  mkdir -p "$target_dir"

  remove_previous_link "$SCRIPT_DIR/AGENTS.md" "$target"
  if [ -f "$target" ]; then
    echo -e "${RED}Error: $target exists as a file. Please remove or backup it manually.${NC}"
    return 1
  fi

  ln -s "$SCRIPT_DIR/AGENTS.md" "$target"
  echo -e "${GREEN}✓ Linked $target -> $SCRIPT_DIR/AGENTS.md${NC}"
}

for context_target in "${CONTEXT_TARGETS[@]}"; do
  link_context "$context_target"
done

echo ""
echo -e "${GREEN}✓ Setup complete.${NC} Verify with:"
for target_dir in "${SKILL_TARGETS[@]}"; do
  echo -e "  ${BLUE}ls -la $target_dir${NC}"
done
for context_target in "${CONTEXT_TARGETS[@]}"; do
  echo -e "  ${BLUE}ls -la $context_target${NC}"
done
