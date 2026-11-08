---
name: browser-usage
description: Decide whether browser use is warranted, then choose and use a browser for visible UI development, testing, and public form filling. Load before any browser, browser-automation, or computer-use action. Do not use browsers for semantic GitHub, CI, or Buildkite work when a purpose-built CLI, API, or connector can do it.
---

# Browser usage

- Default to not using a browser.
- A URL in the request provides context; it does not authorize browser use.
- Never use the Codex in-app browser unless explicitly asked for the in-app browser because extra authentication is required for sensitive data.
- Before using any browser that was not explicitly requested, explain why it is necessary and ask which browser to use.
- Prefer purpose-built CLIs, APIs, and connectors for GitHub, CI, Buildkite, and other semantic operations.
- Exhaust appropriate non-browser options before proposing browser use.
- If authentication is unavailable or expired, report the exact blocker and ask what to do.
  Do not switch browser surfaces, open a login or authorization page, or create an authentication token as a fallback.
- Prefer a headed browser so the user can see the UI and watch the interactions.
- Actually exercise the relevant user flow when working on a UI; do not rely only on source inspection or screenshots.
- Prefer `agent-browser --headed` when authentication is not required, especially for locally running apps during development.
- Use dark mode, so launch headed browsers with `agent-browser --headed --color-scheme dark --args "--force-dark-mode"`.
  `--color-scheme dark` makes pages report `prefers-color-scheme: dark`, and `--force-dark-mode` makes Chrome's own UI dark.
  Both matter in WSL, where Linux Chrome cannot read the Windows theme and otherwise renders light.
- Before using unfamiliar `agent-browser` commands, load its version-matched instructions with `agent-browser skills get core --full`.
- Prefer browser MCP (AKA browsermcp), when available, for filling in forms on public websites that require authentication.
- Ask which browser to use when authentication requirements, available sessions, or the best tool are unclear, even if another skill recommends a browser fallback.
