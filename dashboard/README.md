# Personal Dashboard

Personal Dashboard is a local, configuration-driven view of GitHub pull requests and issues in a normal browser.
It runs customizable local commands to find items, invoke coding agents and other tools, provide prompt autocomplete, stream output, and cancel work in progress.

Pull requests and issues can each have ordered buttons that are always visible and advanced buttons revealed on demand.
Commands stay next to the feature that uses them, and hovering over a button shows its fully resolved command through the native HTML `title` attribute.

Strongly typed Optify configuration can be split into focused files imported by root features.
An Options page lets each person select ordered configuration directories, such as personal and private-work directories, and choose the root features to apply.
The light, dark, or system theme is configured in an Optify feature file with the rest of the dashboard behavior.
Sections, commands, buttons, prompts, and other behavior remain in the configuration files.
A single start command launches the local service and opens the dashboard in the browser.

Delivery phases 1 through 6 are implemented.
The repository contains the Axum and React/Vite scaffold, the Options page and browser store, typed Optify configuration loading and reloads, generated schema and transport artifacts, example feature files, embedded release assets, authenticated WebSocket synchronization, configured item discovery with per-command caching and pagination, configured actions and prompts, generic process streaming and cancellation, and the dense dashboard UI.
Configured prompt autocomplete debounces requests, cancels superseded editor processes, bounds suggestions, and requires an explicit user action before applying one.

## Button templates

Pull request and issue buttons can fill in these placeholders:

| Placeholder         | Filled-in value                              | `command` | `url` |
| ------------------- | -------------------------------------------- | --------- | ----- |
| `{item.number}`     | Positive pull request or issue number        | Yes       | Yes   |
| `{item.repository}` | Normalized `owner/name` repository identifier | Yes       | Yes   |
| `{item.url}`        | Validated HTTPS URL supplied by the item     | Yes       | Yes   |
| `{prompt}`          | Text entered in the button's prompt          | Yes       | No    |

A command containing `{prompt}` must declare a `prompt`, and a declared prompt requires `{prompt}` in the command.
URL buttons cannot declare prompts and must resolve to HTTPS.
Unknown, malformed, or unavailable placeholders are configuration errors.

Command placeholders work in unquoted, single-quoted, and double-quoted Bash text.
Their values are passed as positional parameters so they cannot change the command structure.

Shell parameter expansions and regular-expression quantifiers do not need escaping.
Dashboard placeholder names start with an ASCII letter or underscore and contain only letters, numbers, periods, and underscores.
The parser therefore preserves both `${match[1]}` and numeric quantifiers such as `{36}` exactly as written:

```bash
if [[ $line =~ 'thread/start response:.*id: "([0-9a-f-]{36})"' ]]; then
  open "codex://threads/${match[1]}"
fi
```

```yaml
- label: Review
  command: >-
    codex exec '/review {item.url} {prompt}'
  prompt:
    label: Review focus
    placeholder: Add areas to inspect closely
- label: Open
  url: 'https://example.com/{item.repository}/pull/{item.number}'
```

`{autocomplete.request}` is reserved for the configured autocomplete command and is not available to buttons.

## Documentation

- [Technical specification](SPEC.md)
- [Local development and contributing](CONTRIBUTING.md)
