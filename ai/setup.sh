#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAUDE_DIR="$HOME/.claude"
SKILLS_DIR="$CLAUDE_DIR/skills"
AGENTS_SKILLS_DIR="$HOME/.agents/skills"

GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

SKILL_TARGETS=(
  "$SKILLS_DIR"
  "$AGENTS_SKILLS_DIR"
)

CONTEXT_TARGETS=(
  "$CLAUDE_DIR/CLAUDE.md"
  "$HOME/.pi/agent/AGENTS.md"
)

link_skills() {
  local dest_dir="$1"
  mkdir -p "$dest_dir"
  echo -e "${BLUE}Linking skills to $dest_dir${NC}"

  for skill in "$SCRIPT_DIR/skills"/*/; do
    local skill_name=$(basename "$skill")
    local target="$dest_dir/$skill_name"

    if [ -L "$target" ]; then
      rm "$target"
    elif [ -d "$target" ]; then
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

  if [ -L "$target" ]; then
    rm "$target"
  elif [ -f "$target" ]; then
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
