---
name: browser-usage
description: Decide whether browser use is warranted, then choose and use a browser for visible UI development, testing, and public form filling. Load before any browser, browser-automation, or computer-use action. Do not use browsers for semantic GitHub, CI, or Buildkite work when a purpose-built CLI, API, or connector can do it.
---

# Browser usage

- Default to not using a browser.
- A URL in Justin's request provides context; it does not authorize browser use.
- Never use the Codex in-app browser unless Justin explicitly asks for the in-app browser because extra authentication is required for sensitive data.
- Before using any browser that Justin did not explicitly request, explain why it is necessary and ask which browser to use.
- Prefer purpose-built CLIs, APIs, and connectors for GitHub, CI, Buildkite, and other semantic operations.
- Exhaust appropriate non-browser options before proposing browser use.
- If authentication is unavailable or expired, report the exact blocker and ask Justin what to do.
  Do not switch browser surfaces, open a login or authorization page, or create an authentication token as a fallback.
- Prefer a headed browser so Justin can see the UI and watch the interactions.
- Actually exercise the relevant user flow when working on a UI; do not rely only on source inspection or screenshots.
- Prefer `agent-browser --headed` when authentication is not required, especially for locally running apps during development.
- Before using unfamiliar `agent-browser` commands, load its version-matched instructions with `agent-browser skills get core --full`.
- Prefer browser MCP (AKA browsermcp), when available, for filling in forms on public websites that require authentication.
- Ask Justin which browser to use when authentication requirements, available sessions, or the best tool are unclear, even if another skill recommends a browser fallback.
