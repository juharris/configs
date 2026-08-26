# Personal Dashboard Specification

Status: Proposed

## Summary

Personal Dashboard is a local web application that shows configurable GitHub pull request and issue sections in a normal browser.
Each section runs a complete command from the YAML configuration to discover its items.
Each pull request or issue receives buttons from two ordered configuration lists: buttons that are always visible and advanced buttons revealed on demand.

Buttons run complete inline command strings or open configured URLs.
The backend is not coupled to a particular coding agent, model provider, or command-line tool.
Provider-specific flags, prompts, permissions, and output remain in the command strings where they are easy to find and change.

All user-authored application configuration lives in Optify feature files loaded from one or more directories selected in the browser.
Each selected root feature should import small, focused feature files so related configuration remains easy to find and review.
Rust types define the configuration contract, and a generated local JSON Schema validates every feature file at runtime and in VS Code.

## Design principles

- Keep each complete command next to the section, button, or autocomplete feature that runs it.
- Keep focused feature files small and give each one a clear importing root feature.
- Prefer a duplicated readable command over a registry entry that requires jumping elsewhere in the file.
- Represent commands as strings so long invocations remain easy to copy, paste, and edit.
- Keep Rust and TypeScript behavior provider-neutral.
- Add a new local tool by changing YAML rather than adding an action kind or provider branch.
- Treat configuration as trusted local code while treating discovered item data and browser messages as untrusted input.
- Keep one WebSocket per browser tab and send all application requests through it.
- Keep transport handling, message routing, command execution, and UI presentation encapsulated behind focused structs.

## Goals

- Make discovery commands, cache TTLs, sections, button labels, button ordering, refresh intervals, and autocomplete declaratively configurable.
- Load configuration from multiple ordered directories and merge multiple ordered root features selected in the browser.
- Provide an Options page for Optify directories and active features.
- Select a light, dark, or system theme through strongly typed Optify configuration.
- Refresh connected tabs automatically after Optify successfully rebuilds changed configuration files.
- Support customizable non-interactive Bash commands, including pipelines and redirection.
- Stream standard output, standard error, completion, failure, timeout, and cancellation events to the UI.
- Show the fully resolved command for every command button through its native HTML `title` attribute.
- Provide low-latency prompt autocomplete through a separately configured local command.
- Fail with a precise error when configuration, a command template, an executable, or a required value is invalid.
- Provide one development command and one release executable that starts the local service and opens the browser UI.

## Non-goals

- Hosting the dashboard as a remote or multi-user service.
- Providing built-in behavior for a specific coding agent or model provider.
- Inventing provider-specific action kinds, event formats, session management, or sandbox policies.
- Editing Optify feature-file contents through the UI in the first release.
- Managing Git branches or worktrees independently of configured commands.
- Providing a named command registry or deduplicating similar command strings.
- Running interactive terminal applications that require a TTY.
- Storing access tokens or other credentials in the YAML file.
- Exposing command execution to a LAN, remote browser, or hosted frontend.

## Framework decision

Use [Axum](https://docs.rs/axum/latest/axum/) for a loopback-only Rust HTTP server, with React, TypeScript, and Vite for the frontend.
Axum's [WebSocket support](https://docs.rs/axum/latest/axum/extract/ws/) provides the full-duplex connection needed for requests, command output, autocomplete, cancellation, acknowledgements, and server-initiated confirmation requests.
Release builds use [rust-embed](https://docs.rs/rust-embed/latest/rust_embed/) to compile the Vite assets into the Rust executable.

| Option | Advantages | Tradeoffs | Decision |
| --- | --- | --- | --- |
| Axum + React/Vite | Normal browser UI, full-duplex WebSockets, TypeScript ecosystem, and one release executable with embedded assets | Requires a loopback listener, an explicit message protocol, reconnect behavior, and browser-origin defenses | Selected |
| Tauri 2 + React/Vite | One packaged desktop application and native local-process access | Hosts the UI in a system WebView rather than the user's normal browser | Not selected |
| Leptos full stack | Rust across client and server | Adds Rust/Wasm UI complexity without improving this local-command use case | Not selected |

The release server binds to `127.0.0.1` on stable public port `5173` and opens the canonical `http://127.0.0.1:5173` URL in the default browser.
It fails clearly when the port is occupied instead of silently selecting a random port.
The Vite development server uses the same public host and port, with `strictPort` enabled, and proxies to the development-only Axum listener on `127.0.0.1:3000`.
A stable origin is required because browser storage is scoped to the scheme, host, and port and must remain available after a restart.
Remote access would materially change the command-execution threat model and is outside the first release.

## Technology choices

### Rust backend

- Axum for HTTP, WebSocket upgrades, and static frontend responses.
- [Optify](https://docs.rs/optify/) for recursive feature-file discovery, schema validation, typed loading, and change watching.
- rust-embed for compiling production frontend assets into the executable.
- Schemars for generating JSON Schema from the Rust configuration types.
- Serde and serde_json for configuration, command output, and WebSocket messages.
- thiserror for explicit application errors.
- Tokio for asynchronous processes, timeouts, cancellation, and concurrent refreshes.
- tower-http for HTTP tracing, limits, and response security headers.
- tracing for structured local diagnostics.
- ts-rs for generating TypeScript transport types from Rust.
- webbrowser for opening the loopback URL after the server is ready.

Application code must not add a second YAML parser or JSON Schema validation path alongside Optify.
Dependency versions will be pinned in `Cargo.lock` and `pnpm-lock.yaml` when the application is scaffolded.

### Frontend

- React and TypeScript for components and state.
- Vite for the development server and production assets.
- Vitest and Testing Library for frontend tests.
- Plain CSS variables and small reusable components initially.
- concurrently for starting Axum and Vite through one development script and terminating both when either fails.

The `pnpm --dir dashboard run dev` script starts both application layers.
Vite's documented [`server.proxy`](https://vite.dev/config/server-options#server-proxy) forwards `/bootstrap` and the `/ws` WebSocket upgrade to Axum so the browser uses one origin.
The production build runs Vite before Cargo so rust-embed receives the current frontend assets.

## Architecture

The browser never supplies an executable or command string.
It sends a configured button position, configuration revision, prompt value, and stable item reference.
The backend resolves the corresponding validated configuration and item snapshot before it constructs a process invocation.

```text
Normal browser
  -> React and one same-origin WebSocket per tab
    -> ConnectionSession
      -> MessageRouter
        -> focused request handlers
          -> ButtonService
          -> AutocompleteService
          -> ItemService
          -> ProcessRunner
          -> Optify-backed ConfigService
```

The Axum upgrade handler validates the request and creates a `ConnectionSession`.
The connection session owns authentication, socket lifecycle, and bounded outbound delivery.
`MessageRouter` delegates each typed request variant to one focused handler struct.
Handlers translate transport values and call application services; they do not parse command lines, launch processes, or write to sockets.

Planned backend modules are:

- `autocomplete`: debounces, supersedes, and cancels configured autocomplete commands.
- `buttons`: resolves button configuration and presentation metadata for an item.
- `commands`: compiles Bash templates, resolves command invocations, and formats display previews.
- `config`: owns the active Optify `OptionsWatcher`, applies Optify setup changes, and publishes immutable validated configuration snapshots.
- `connections`: owns per-tab sessions, bounded queues, and targeted or broadcast delivery.
- `errors`: defines stable error codes and safe user-facing context.
- `items`: runs section discovery and validates normalized GitHub items.
- `messages`: defines generated request, response, event, and UI-request types.
- `processes`: owns process lifecycle, output limits, cancellation, and timeouts.
- `router`: maps each client request variant to one injected handler.
- `state`: owns section snapshots and active-run state.

No module is named after or branches on a coding-agent provider.
A provider-specific invocation is data in the YAML command string.

## Configuration contract

### Options page and browser-local state

The Options page is always available from the dashboard navigation and contains these sections in order:

1. **Optify directories** provides an ordered list of absolute configuration-directory paths.
2. **Optify features** provides an ordered list of root feature names to merge.

The dashboard route redirects to the Options page when valid directory and feature arrays are unavailable.
After valid Optify setup exists, the page remains available for later changes.

The browser persists exactly two values in `localStorage`:

| Key | Stored value | Purpose |
| --- | --- | --- |
| `personal-dashboard.config-directories` | JSON `string[]` | Ordered absolute paths to Optify configuration directories |
| `personal-dashboard.features` | JSON `string[]` | Ordered Optify root feature names to merge |

The frontend combines those values only in memory and on the wire:

```ts
type OptifySetup = {
  configDirectories: string[];
  features: string[];
};
```

There is no local-storage wrapper object, version field, or third application key.
Theme, sections, buttons, commands, prompts, refresh intervals, drafts, run history, and every other application option belong in Optify feature files or transient in-memory state.
If a future incompatible storage shape is necessary, it should use new key names and an explicit migration based on that concrete requirement.

The page lets a person add, edit, remove, and reorder both Optify lists.
Directory paths are editable text inputs because ordinary browser directory pickers do not reliably expose absolute filesystem paths.
The UI requires at least one directory and one feature, rejects blank entries, and preserves the entered ordering.
Client-side checks provide immediate feedback, but the backend treats both arrays as untrusted input.

After WebSocket authentication, selecting **Apply Optify configuration** sends both arrays in a typed setup request.
The backend requires each directory to be an absolute, readable directory and each feature name to be non-empty.
It passes the directories unchanged to [`OptionsWatcher::build_from_directories_with_schema_and_options`](https://docs.rs/optify/1.3.3/optify/provider/struct.OptionsWatcher.html#method.build_from_directories_with_schema_and_options) and passes the features unchanged to `get_all_options`.
Only after the backend accepts the complete setup and returns the active configuration revision does the browser replace the two persisted arrays.
A submitted setup that exactly matches the active ordered arrays returns the current revision without rebuilding the watcher or refreshing sections.
A rejected Optify setup remains as an editable form draft, while the previously accepted arrays and dashboard stay active.

Changing either Optify list builds and validates a candidate watcher and merged configuration before replacing the active watcher.
The old watcher, setup, and application snapshot remain active if directory loading, schema validation, feature resolution, deserialization, or application validation fails.
After a successful atomic replacement, the service increments the configuration revision, refreshes every section, and broadcasts the new setup and dashboard state to authenticated tabs.
The active Optify setup is process-wide even though the two lists are persisted by the browser.

Missing or malformed Optify setup opens the Options page without inventing paths or feature names.
The service uses a stable loopback origin in development and release builds so the same origin can read these values after a restart.
It must not fall back to an ephemeral port because the different origin would have different `localStorage`.

### Files

- `dashboard/configs/` is the repository's example configuration directory.
- `dashboard/configs/dashboard.yaml` is the example `dashboard` root feature.
- `dashboard/configs/dashboard/` contains focused example features imported by that root.
- `dashboard/configs/.optify/schema.json` is generated from Rust types and committed.
- `.vscode/settings.json` associates the local schema with every JSON, YAML, and YML feature file below `dashboard/configs/`.
- `.vscode/extensions.json` recommends `optify-config.optify` and `redhat.vscode-yaml`.

The schema is generated output, not another feature.
Its `.optify/schema.json` location follows Optify's documented [custom-schema convention](https://github.com/juharris/optify/blob/main/README.md#custom-schemas).
The feature-file wrapper references Optify's standard schema for `imports`, `metadata`, and other envelope fields, while its `options` schema is generated from the dashboard Rust types.
The application passes this one schema path to the multi-directory watcher, so every feature file in every selected directory is checked against the same contract and external directories do not need another runtime schema copy.
Release executables embed the committed generated schema and materialize it in a process-lifetime temporary directory because Optify 1.3.3 accepts a filesystem path and rereads that path when its watcher rebuilds.
The configuration service owns that temporary directory for at least as long as any active watcher can use it.

Optify recursively loads `.json`, `.yaml`, and `.yml` files below every selected directory.
Selected directories form one feature namespace, so canonical feature names should be unique across them.
A root feature should import a small set of focused files through their canonical, path-derived feature names.
Each focused file should normally have one importing root so ownership and navigation stay clear.
Optify applies a root's imports in their listed order before applying that root's own options.
The ordered root features selected in the browser are then merged to produce the application configuration.
Optify merges objects recursively, while later features replace earlier scalar and array values.
Feature order therefore matters, including for `sections` and button lists.

The VS Code association is:

```json
{
  "json.schemaDownload.enable": true,
  "json.schemas": [
    {
      "fileMatch": [
        "dashboard/configs/**/*.json"
      ],
      "url": "./dashboard/configs/.optify/schema.json"
    }
  ],
  "json.validate.enable": true,
  "yaml.schemas": {
    "./dashboard/configs/.optify/schema.json": [
      "dashboard/configs/**/*.{yaml,yml}"
    ]
  },
  "yaml.validate": true,
  "yaml.yamlVersion": "1.2"
}
```

This repository association covers the example directory.
A separate personal or private-work configuration repository should associate the same generated schema with its own feature-file globs in that workspace's VS Code settings.
Runtime schema validation applies regardless of editor configuration.

### Loading and validation

For initial Optify setup or a directory or feature change, the configuration service performs these steps:

1. Validate the submitted directory and feature arrays without changing active state.
2. Build a candidate `OptionsWatcher` with the ordered directories, `dashboard/configs/.optify/schema.json`, and explicit `WatcherOptions` by calling `build_from_directories_with_schema_and_options`.
3. Immediately register an `OptionsWatcher::add_listener` callback on the candidate.
4. Retrieve the merged options for the ordered feature names with `get_all_options`.
5. Deserialize the merged value into the complete `RootConfig`.
6. Reject unknown fixed-shape fields.
7. Compile every Bash command and URL template.
8. Validate cross-field requirements such as prompt configuration and unique section IDs.
9. Verify the configured Bash executable.
10. Atomically publish the candidate watcher, accepted Optify setup, and first immutable configuration snapshot.

Initial loading is fail-fast.
There is no fallback configuration.
The active `OptionsWatcher` remains alive until a later valid Optify setup atomically replaces it and watches every selected directory recursively.

Optify's listener type is an `Arc<dyn Fn(&HashSet<PathBuf>) + Send + Sync>`.
The callback receives the changed paths after Optify has successfully rebuilt and replaced its provider.
Because the callback runs synchronously on Optify's watcher thread, it only copies the changed paths into a Tokio watch channel.
An asynchronous `ConfigReloadService` coalesces bursts, retrieves the newly merged options for the active ordered feature list, performs application validation, and publishes the next immutable snapshot.

After a snapshot is accepted, the service increments the configuration revision, refreshes section state, and broadcasts a `configuration_reloaded` event to every authenticated tab over its existing WebSocket.
The snapshot includes the configured theme so initial loading, watcher reloads, and Optify feature changes update every connected tab from the same validated configuration.
Running commands keep the resolved configuration captured when they started, while new requests use the new revision.
If application validation fails, the UI retains the previous application snapshot and receives a safe configuration error.

Optify does not replace its provider or invoke listeners when a changed file fails provider or schema rebuilding.
In that case the application continues using the previous configuration, while VS Code diagnostics and the service log identify the invalid edit.

There is no `schema_version` field.
The installed application and its generated schema define the supported configuration shape.
If an incompatible change eventually needs migration support, it should be designed from a concrete migration requirement rather than adding a version field preemptively.

### Root options

Optify feature files use a top-level `options` field and may also use standard Optify envelope fields such as `imports` and `metadata`.
Individual feature files validate against a partial dashboard-options type so imported fragments do not need to repeat unrelated settings.
The options merged from the complete ordered feature list must deserialize into the complete `RootConfig`.
The keys inside `options` are:

| Key | Type | Purpose |
| --- | --- | --- |
| `appearance` | object | Defines the dashboard theme |
| `application` | object | Defines concurrency, output, refresh, and timeout limits |
| `autocomplete` | object | Defines the one inline command used for prompt suggestions |
| `buttons` | object | Defines `always` and `advanced` lists for issues and pull requests |
| `sections` | ordered list | Defines each item section, complete discovery command, cache TTL, and page size |

Fixed-shape objects reject unknown fields.
Required values have no implicit fallback unless the generated schema documents a safe default.
Secrets and required environment values never receive fallback values.

### Illustrative configuration

The repository example splits configuration by responsibility and gives every focused feature one importing root:

```text
dashboard/configs/
  .optify/schema.json
  dashboard.yaml
  dashboard/
    appearance.yaml
    application.yaml
    autocomplete.yaml
    buttons.yaml
    sections.yaml
```

`dashboard/configs/dashboard.yaml` is the root feature selected in the browser:

```yaml
imports:
  - dashboard/appearance
  - dashboard/application
  - dashboard/autocomplete
  - dashboard/buttons
  - dashboard/sections
```

Imports use canonical feature names derived from relative paths without file extensions.
Aliases are unnecessary for names that already closely match their paths.

`dashboard/configs/dashboard/appearance.yaml` contains the browser theme:

```yaml
options:
  appearance:
    theme: system
```

`theme` is a required enum with `light`, `dark`, and `system` values.
`system` follows `prefers-color-scheme` and responds when the operating-system preference changes.

`dashboard/configs/dashboard/application.yaml` contains application limits:

```yaml
options:
  application:
    command_timeout_seconds: 1800
    default_refresh_seconds: 300
    max_concurrent_commands: 4
    max_output_bytes_per_run: 10485760
    shell: /bin/bash
```

`dashboard/configs/dashboard/autocomplete.yaml` keeps the complete autocomplete command next to its settings:

```yaml
options:
  autocomplete:
    command: >-
      claude --print '{autocomplete.request}'
    debounce_milliseconds: 300
    instruction: >-
      Suggest concise details that improve the draft instructions without
      changing their intent or starting the work.
    minimum_characters: 20
```

`dashboard/configs/dashboard/buttons.yaml` contains the complete issue and pull-request button lists:

```yaml
options:
  buttons:
    issues:
      always:
        - label: Investigate
          command: >-
            cd; pi --print 'Investigate {item.url}. {prompt}'
          prompt:
            label: Additional instructions
            placeholder: Add context or constraints
      advanced:
        - label: Start work
          command: >-
            cd; claude --print 'Implement {item.url}. {prompt}'
          confirm: true
          prompt:
            label: Implementation details
            placeholder: Add constraints or acceptance criteria
        - label: Open
          url: '{item.url}'

    pull_requests:
      always:
        - label: Review
          command: >-
            cd; codex exec '/review {item.url} {prompt}'
          prompt:
            label: Review focus
            placeholder: Add areas to inspect closely
      advanced:
        - label: Second opinion
          command: >-
            cd; claude --print 'Review {item.url}. {prompt}'
          prompt:
            label: Review focus
            placeholder: Add areas to inspect closely
        - label: Open
          url: '{item.url}'
```

`dashboard/configs/dashboard/sections.yaml` keeps each complete discovery command with its section:

```yaml
options:
  sections:
    - id: authored_pull_requests
      title: My pull requests
      item_kind: pull_request
      cache_ttl_seconds: 300
      command: >-
        gh search prs
        --author @me
        --state open
        --json assignees,author,isDraft,labels,number,repository,state,title,updatedAt,url
      items_per_page: 6

    - id: requested_reviews
      title: Reviews requested
      item_kind: pull_request
      cache_ttl_seconds: 300
      command: >-
        gh search prs
        --review-requested @me
        --state open
        --json assignees,author,isDraft,labels,number,repository,state,title,updatedAt,url
      items_per_page: 6

    - id: assigned_issues
      title: Assigned issues
      item_kind: issue
      cache_ttl_seconds: 300
      command: >-
        gh search issues
        --assignee @me
        --state open
        --json assignees,author,labels,number,repository,state,title,updatedAt,url
      items_per_page: 6
```

YAML folded strings keep long commands readable while producing one command-line string.
The focused files intentionally repeat similar button and discovery commands rather than creating a command registry.
Changing a section query or an agent invocation does not require finding a separate registry entry and understanding which other features reference it.

A private-work directory can follow the same pattern with a separate root such as `work-dashboard.yaml` importing files below `work-dashboard/`.
Selecting `dashboard` followed by `work-dashboard` applies personal configuration first and work configuration second.
Because Optify replaces arrays rather than merging their elements, a work `sections` or button list replaces the corresponding earlier list in full.

### Buttons

`buttons.issues` and `buttons.pull_requests` each contain exactly two ordered lists:

- `always` is rendered beside every matching item.
- `advanced` is rendered only after the user expands that item's advanced actions.

Either list may be empty.
The disclosure control is omitted when `advanced` is empty.
List order is display order.

Each button has a visible `label` and exactly one of:

- `command`, containing the complete inline command to run.
- `url`, containing a URL template to open in a new browser tab.

A command button may also define `confirm` and `prompt`.
A prompt declaration requires the command to contain `{prompt}`, and `{prompt}` requires a prompt declaration.
Commands that should change directories use ordinary Bash `cd` syntax in the configured string.
The dashboard does not inspect local checkouts or disable actions based on their presence.

Buttons do not need configured IDs.
The UI identifies a button by item kind, list name, list index, and configuration revision.
The backend rejects a request from an obsolete configuration revision instead of running a button that may have moved after reload.

## Command strings

### Compilation

A command is stored as one YAML string and executed by the configured Bash executable.
The string may use ordinary Bash syntax, including quoting, pipelines, conditionals, redirection, and environment-variable expansion.

At configuration load, `CommandTemplate` parses every placeholder and compiles it into a Bash positional-parameter reference.
At execution, the corresponding values are passed as separate arguments to Bash.
Resolved item and prompt values are never concatenated into the executable script.

The compiler supports placeholders in unquoted, single-quoted, and double-quoted text while preserving the surrounding word.
For example, this remains one prompt argument:

```bash
codex exec '/review {item.url} {prompt}'
```

The command above is semantically equivalent to a fixed script whose URL and prompt are positional parameters.
Characters in either value cannot terminate a quote, add an argument, start a substitution, or introduce another command.
Malformed Bash quoting and malformed placeholders are configuration errors.

### Templates

Templates use single braces.
The compiler replaces placeholders with positional-parameter references before any item or browser value is available.
Resolving a placeholder therefore never changes Bash structure.

Initial placeholders are:

- `{autocomplete.request}` for the complete generated autocomplete request.
- `{item.number}` for the positive issue or pull request number.
- `{item.repository}` for the normalized `owner/name`.
- `{item.url}` for the validated HTTPS item URL.
- `{prompt}` for the optional text entered for the selected button.

Each placeholder is valid only where its context exists.
Unknown placeholders, malformed braces, unavailable context, and empty command templates are errors.
There is no `{section.query}`; the complete discovery query stays directly in the section command.

### Resolution and preview

`CommandResolver` produces one immutable `ResolvedCommand` containing the Bash executable, compiled script, and positional values.
Both `ProcessRunner` and `CommandPreviewFormatter` consume that same value.

The preview formatter uses the compiled template to render every dynamic value with Bash-safe quoting for display.
The title shows the resolved configured command rather than the internal positional-parameter plumbing.
The preview is never parsed or executed.

Every command button receives the preview through its native HTML `title` attribute.
A URL button receives its complete validated URL as `title`.
If resolution fails for an item, the button is disabled and its `title` contains the same failure reason rather than a fabricated command.

Clicking a button sends only its position, configuration revision, prompt, and item reference through the WebSocket.
The browser never sends a command string, executable, or arguments.

## Item discovery

Every section contains its complete discovery command.
The backend does not append a query, flags, fields, or executable.

A successful discovery command writes a JSON array to standard output.
The selected `item_kind` determines whether each object is validated as a pull request or issue.
The normalized item contains the repository, number, title, URL, author, labels, state, update time, and available review or assignment fields.
The item `url` is its one destination link and is also the value exposed through `{item.url}` to configured buttons.
Discovery commands may replace a provider's returned URL while normalizing their output; the dashboard does not apply a second link-template layer.
An optional `source` string may disambiguate otherwise identical item references without changing provider-neutral backend behavior.

The example uses `gh search`, but a custom command may produce the same JSON shape.
No provider-specific discovery command is hidden in Rust.

Sections refresh independently and paginate their current items using the section's required positive `items_per_page` value.
Each section's required positive `cache_ttl_seconds` value controls its in-memory discovery cache.
Successful normalized results are cached by the complete command, shell, item kind, and TTL so a configuration reload reuses unchanged queries while a changed query executes immediately.
Failures are not cached.
A valid result atomically replaces that section's items.
A non-zero exit, timeout, invalid JSON, or invalid item preserves the previous data, marks it stale, and displays a sanitized error with a retry button.
Non-zero exits include bounded, control-character-filtered standard error so local command failures remain actionable.
Refreshes for the same section are coalesced and global process concurrency is bounded by configuration.

## Command execution

`ProcessRunner` launches only a `ResolvedCommand` produced from the current validated configuration.
It never accepts an executable or argument vector from a WebSocket message.

The runner:

- Invokes the configured Bash executable with the fixed compiled script and separate positional values.
- Captures standard output and standard error concurrently.
- Emits bounded text chunks and a final exit status.
- Enforces global concurrency, per-run output, and timeout limits.
- Starts each command in a process group so cancellation terminates its descendants.
- Records whether a run completed, failed, timed out, or was cancelled.

Command output is generic text in the first release.
The backend does not parse one coding agent's event stream, capture provider session IDs, or infer provider state.
Commands that need agent-specific permissions, modes, or output flags express them in their inline command strings.

The YAML file is trusted local code.
The UI shows the exact resolved invocation before it is run, and `confirm: true` requires an explicit confirmation for that button.
The dashboard does not try to infer whether an arbitrary local command is read-only or destructive.

## WebSocket application protocol

HTTP is limited to frontend assets, non-cacheable bootstrap data from `/bootstrap`, and the `/ws` upgrade.
Each browser tab owns one application WebSocket for its lifetime.
Vite may have a separate development-only connection for hot-module replacement.

Every application request uses the WebSocket, including:

- Authenticate the connection.
- Apply ordered configuration directories and root features.
- Retrieve dashboard and active-run snapshots.
- Refresh a section.
- Preview or run a button.
- Cancel a run.
- Request or cancel autocomplete.

The transport uses JSON text messages.
Serde tagged enums define the protocol, and ts-rs generates matching TypeScript types.

Client-to-server envelopes are:

- `authenticate`, containing the bootstrap token and supported protocol version.
- `request`, containing a unique request ID and one typed request variant.

Server-to-client envelopes are:

- `connection_ready`, containing connection, protocol, setup status, optional active configuration, and event-cursor information.
- `response`, correlated to a request ID.
- `error`, with a stable code, safe message, retryability, and optional field path.
- `event`, with an event ID, sequence number, and typed payload.

Run events cover queued, started, output, cancellation requested, completed, failed, timed out, and cancelled states.
A cancel response confirms that cancellation was requested.
The later terminal event confirms the final process state.

### Connection lifecycle

The frontend retrieves a high-entropy, short-lived, single-use token from the same-origin `/bootstrap` endpoint.
It sends that token in the first WebSocket message.
Unauthenticated connections may send no service requests and close after a short deadline.
The token is not placed in a URL, persisted, or logged.

After authentication, the frontend reads the two Optify arrays and sends them through the typed setup request.
No configuration-dependent request is accepted until the connection has synchronized with the process-wide active setup.

One reader task validates incoming envelopes.
One writer task serializes outbound messages from a bounded priority queue.
No application service writes directly to a socket.

The frontend reconnects with bounded exponential backoff and provides its last processed event cursor.
Each reconnect obtains a new bootstrap token.
The server replays retained events when possible and otherwise requires fresh dashboard and active-run snapshots.
Mutating requests are not replayed automatically.

Multiple tabs create independent connections.
Responses and run events return to their originating tab, while shared state changes may be broadcast to all authenticated tabs.

### Handler encapsulation

`ConnectionSession` owns authentication, parsing, queueing, and socket lifecycle.
`MessageRouter` delegates each request enum variant to a focused handler such as `ApplyOptifySetupHandler`, `CancelRunHandler`, `PreviewButtonHandler`, `RefreshSectionHandler`, or `RunButtonHandler`.
Handlers receive application services through constructor-injected traits and return typed results.

Services publish events through an `EventPublisher` trait implemented by the connection registry.
Router tests verify that every request variant has one handler.
Handler tests use fake services without starting Axum or a WebSocket.

One frontend `WebSocketClient` owns the raw browser socket, request correlation, timeouts, and reconnect behavior.
One `OptifySetupStore` owns parsing and writing the two permitted `localStorage` keys.
React components call typed client methods and consume state through reducers.
Components never construct protocol envelopes, access the raw socket, or call `localStorage` directly.

## Prompt autocomplete

Autocomplete is another configured command, not a built-in model integration.
The `autocomplete` object defines the complete command, instruction, debounce interval, and minimum draft length.

After the debounce interval, the browser sends the current draft, cursor or selection, item reference, and selected button position.
The backend creates a bounded text request containing the configured instruction, button label, item URL, repository, and draft.
That complete text is supplied through the compiled `{autocomplete.request}` positional parameter.

A successful command's bounded standard output becomes the suggested edit.
Standard error is diagnostic output and is not inserted into the prompt.
The first release does not require a provider-specific JSON or event format.

Only one autocomplete process per editor remains active.
A newer request cancels the older process, and the UI ignores events whose request ID is no longer current.
Suggestions are optional edits and never execute a button.

Prompt drafts, generated autocomplete requests, and suggestions are excluded from logs.

## User interface

The first release has an Options page at `/options`, one dashboard page, and a run drawer.

- The dashboard navigation includes an **Options** link whenever the dashboard is available.
- The Options page presents Optify directories followed by Optify features.
- Directory and feature rows have explicit labels and keyboard-accessible controls for adding, removing, and reordering values.
- Applying Optify changes reports path, schema, feature, and application-validation errors without discarding the last active setup.
- The validated `appearance.theme` option controls the Options page, dashboard page, and run drawer.
- Theme CSS variables and the `color-scheme` property are applied to the document root whenever a configuration snapshot is accepted.
- The `system` theme follows `prefers-color-scheme` without changing the configured value.
- Sections appear in merged configuration order and show title, item count, refresh status, stale status, and last successful refresh.
- Items show repository, number, title, author, labels, a compact accessible status icon, and relevant review or assignment state.
- Draft, open, merged, and closed status icons are visually distinct and expose their full status label to assistive technology.
- Each section paginates independently using its configured item count.
- Every `always` button appears in merged configuration order.
- A per-item accessible disclosure control reveals `advanced` buttons in merged configuration order.
- Every button uses the native HTML `title` attribute for its resolved command, validated URL, or disabled reason.
- The application does not implement a separate custom tooltip component for command previews.
- Buttons with prompt configuration open an editor before execution.
- Prompt editors show autocomplete, superseded, connection, and error states without blocking typing.
- Runs show the full command, live output, elapsed time, a Cancel button, exit status, and final state.
- The page shows connection and resynchronization status and disables command execution while disconnected.
- Keyboard navigation, visible focus, reduced motion, and semantic controls are required.

Section, item, and process text is untrusted.
It is rendered as text by default.
Any Markdown renderer must disable raw HTML and unsafe URL schemes.

## Browser and process security

A browser-accessible service that launches local processes must:

- Bind only to `127.0.0.1`.
- Validate the exact loopback `Host` header to reject DNS-rebinding requests.
- Validate `Origin` before WebSocket upgrade and reject missing, null, remote, or unexpected origins.
- Use same-origin Vite proxies during development rather than weakening origin checks.
- Authenticate the first WebSocket message before accepting service requests.
- Send no permissive CORS headers.
- Apply a restrictive content security policy and deny framing and MIME sniffing.
- Limit WebSocket frame size, decoded message size, request rate, process count, duration, and captured output.
- Accept Optify setup only as typed directory and feature-name lists.
- Accept only typed configuration positions and item references for configuration-dependent requests.
- Resolve commands exclusively from the current validated configuration snapshot.
- Compile placeholders into Bash positional parameters so dynamic values cannot alter script structure.
- Validate HTTPS URLs before returning or opening them.
- Open external URLs with `noopener` and `noreferrer`.
- Never store or log bootstrap tokens, credentials, prompt text, full environments, or credential files.
- Never infer a secret or environment-variable fallback.

The WebSocket bootstrap token prevents another local web page from silently driving the service.
It does not make an unsafe YAML command safe.
The local configuration author remains responsible for the commands intentionally placed in the file.

## Runtime state and observability

The first release keeps discovered items, replay buffers, and active-run state in memory.
Durable history and a local database are deferred until a concrete retention requirement exists.

Rotating structured logs are written to the platform application-data directory rather than the repository.
Logs include operation IDs, section IDs, button positions, durations, exit status, and sanitized error categories.
They exclude command prompts, item bodies, autocomplete text, environment values, and raw credential-bearing output.

## Development and quality gates

The package scripts inside `dashboard/` provide the contributor interface:

- `pnpm --dir dashboard run build` builds the frontend and Rust executable with embedded assets.
- `pnpm --dir dashboard run check` runs formatting, Clippy, Rust tests, TypeScript checking, ESLint, frontend tests, and generated-artifact checks.
- `pnpm --dir dashboard run dev` starts Axum and Vite together.
- `pnpm --dir dashboard run schema:generate` regenerates the committed JSON Schema.
- `pnpm --dir dashboard run start` starts the release server and opens the browser.
- `pnpm --dir dashboard run test` runs Rust and frontend tests.

Tests must cover:

- Configuration loading from multiple ordered directories, focused imports, unknown fields, missing fields, invalid templates, and atomic application snapshots.
- Ordered root-feature precedence, including recursive object merges and full array replacement.
- Generated schema and TypeScript bindings remaining current.
- Recursive config discovery and local schema associations for JSON, YAML, and YML files.
- Options-page editing, validation, stable-origin persistence, and exactly two permitted `localStorage` array keys.
- Strongly typed light, dark, and system theme configuration, system-preference changes, and invalid-value handling.
- Theme updates after initial loading, watcher reloads, and ordered feature changes without browser-local persistence.
- Idempotent Optify setup synchronization that does not rebuild the watcher for unchanged arrays.
- Atomic watcher replacement after a valid Optify setup change and retention of the previous setup, watcher, and snapshot after an invalid change.
- `OptionsWatcher::add_listener` handoff, changed paths, successful reload, burst coalescing, and WebSocket refresh events.
- Failed Optify rebuilds retaining the previous provider without a listener event.
- Compiling quoted Bash command strings, pipelines, redirection, and long folded YAML strings.
- Placeholder behavior in unquoted, single-quoted, and double-quoted contexts.
- Exact command-preview quoting, URL titles, and disabled reasons.
- Inline discovery commands and normalized item validation using checked-in fixtures.
- Successful discovery caching, command-key changes, and TTL expiration using fake executables.
- Generic process output, concurrency, limits, timeout, cancellation, and non-zero exit.
- Loopback binding, host and origin checks, bootstrap authentication, and response headers.
- Message routing, request correlation, event ordering, reconnect resynchronization, and queue bounds.
- Autocomplete debounce, supersession, cancellation, stale-response rejection, and bounded text output.
- UI loading, stale, empty, error, disconnected, prompt, streaming, cancellation, and button-title states.

Tests use fake executables and temporary directories.
They must not require network access or a real coding-agent account.
Fixtures and helpers should cover meaningful behavior without cloning nearly identical tests for different command names.

## Delivery sequence

1. Scaffold Axum, React, TypeScript, Vite, package scripts, embedded assets, and shared transport types.
2. Implement the Options page, Optify setup store, typed multi-directory Optify configuration, the `OptionsWatcher` listener, schema generation, directory-wide VS Code mappings, template compilation, and atomic reload.
3. Implement authenticated WebSocket sessions, typed routing, bounded queues, and reconnect resynchronization.
4. Implement Bash template compilation, discovery sections, item normalization, and the dashboard UI.
5. Implement generic process streaming, prompts, button execution, command previews, confirmation, and cancellation.
6. Implement configured autocomplete and accessibility and security smoke coverage.

## Acceptance criteria

- `pnpm --dir dashboard run dev` is the only command needed to start both application layers after dependencies are installed.
- A release build produces one executable that serves the embedded UI and opens it in the user's normal browser.
- Each tab uses one authenticated WebSocket for all application requests, responses, command output, autocomplete, and confirmation requests.
- The server rejects unexpected hosts, origins, and unauthenticated WebSocket messages.
- Development and release restarts preserve Optify setup by serving from stable loopback origins and never falling back to a random port.
- The browser stores only ordered configuration-directory and root-feature `string[]` values in `localStorage`.
- The Options page presents directories followed by features.
- The Options page supports adding, editing, removing, and reordering both Optify arrays.
- The strongly typed `appearance.theme` Optify option accepts only `light`, `dark`, or `system`.
- Accepted configuration changes update the theme in connected tabs, and `system` follows operating-system changes.
- Optify loads and watches every selected directory recursively and applies the selected root features in order.
- Focused features are imported by root features through canonical path-derived names.
- The generated schema validates every loaded feature at runtime, and VS Code applies it to every JSON, YAML, and YML file below `dashboard/configs/`.
- A valid Optify setup change atomically replaces the active watcher, setup, and snapshot.
- An invalid Optify setup change retains the previous watcher, setup, and snapshot.
- A successful Optify watcher rebuild refreshes section state and every connected tab through `OptionsWatcher::add_listener`.
- An invalid watched edit retains the previous provider and application snapshot.
- The configuration has no `schema_version`, named command registry, provider-specific action kind, or argument-array syntax.
- Every discovery command and button command is a complete inline YAML string at its point of use.
- Pull requests and issues each render their configured `always` buttons and reveal `advanced` buttons on demand.
- Every button's native HTML `title` contains its fully resolved command, validated URL, or disabled reason.
- The browser cannot supply or alter an executable or argument vector.
- Any compatible non-interactive local command works without provider-specific Rust or TypeScript changes.
- Prompt autocomplete runs the configured command, cancels superseded work, and ignores stale results.
- Command output streams live and can be cancelled without a provider-specific output parser.
- No credential or secret is stored in the YAML file or returned to the frontend.
