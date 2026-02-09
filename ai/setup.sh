#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAUDE_DIR="$HOME/.claude"
SKILLS_DIR="$CLAUDE_DIR/skills"

GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}Setting up Claude Code configuration...${NC}"

mkdir -p "$SKILLS_DIR"

# Symlink each skill directory individually
for skill in "$SCRIPT_DIR/skills"/*/; do
  skill_name=$(basename "$skill")
  target="$SKILLS_DIR/$skill_name"

  if [ -L "$target" ]; then
    echo -e "${YELLOW}Removing existing symlink: $skill_name${NC}"
    rm "$target"
  elif [ -d "$target" ]; then
    echo -e "${YELLOW}Warning: $target exists as a directory, skipping${NC}"
    continue
  fi

  ln -s "$skill" "$target"
  echo -e "${GREEN}✓ Linked skill: $target -> $skill${NC}"
done

# Symlink CLAUDE.md to AGENTS.md
if [ -L "$CLAUDE_DIR/CLAUDE.md" ]; then
  echo -e "${YELLOW}Removing existing CLAUDE.md symlink...${NC}"
  rm "$CLAUDE_DIR/CLAUDE.md"
elif [ -f "$CLAUDE_DIR/CLAUDE.md" ]; then
  echo -e "${RED}Error: $CLAUDE_DIR/CLAUDE.md exists as a file. Please remove or backup it manually.${NC}"
  exit 1
fi

ln -s "$SCRIPT_DIR/AGENTS.md" "$CLAUDE_DIR/CLAUDE.md"
echo -e "${GREEN}✓ Linked $CLAUDE_DIR/CLAUDE.md -> $SCRIPT_DIR/AGENTS.md${NC}"

echo ""
echo -e "${GREEN}✓ Setup complete.${NC} Verify with:"
echo -e "  ${BLUE}ls -la $SKILLS_DIR${NC}"
echo -e "  ${BLUE}ls -la $CLAUDE_DIR/CLAUDE.md${NC}"
