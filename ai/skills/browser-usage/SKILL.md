---
name: browser-usage
description: Choose and use a browser for visible UI development, testing, and public form filling. Use when we need to open, inspect, interact with, or verify a web UI.
---

# Browser usage

- Prefer a headed browser so Justin can see the UI and watch the interactions.
- Actually exercise the relevant user flow when working on a UI; do not rely only on source inspection or screenshots.
- Prefer `agent-browser --headed` when authentication is not required, especially for locally running apps during development.
- Before using unfamiliar `agent-browser` commands, load its version-matched instructions with `agent-browser skills get core --full`.
- Prefer browser MCP (AKA browsermcp), when available, for filling in forms on public websites that require authentication.
- Ask Justin which browser to use when authentication requirements, available sessions, or the best tool are unclear.
