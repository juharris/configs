# Personal Dashboard

Personal Dashboard is a local, configuration-driven view of GitHub pull requests and issues in a normal browser.
It runs customizable local commands to find items, invoke coding agents and other tools, provide prompt autocomplete, stream output, and cancel work in progress.

Pull requests and issues can each have ordered buttons that are always visible and advanced buttons revealed on demand.
Commands stay next to the feature that uses them.
Selecting a command button opens a pre-run view of the backend-resolved command, including its working directory, item values, and any configured prompt text, before an explicit **Run** action executes it.

Strongly typed Optify configuration can be split into focused files imported by root features.
An Options page lets each person select ordered configuration directories, such as personal and private-work directories, and choose the root features to apply.
The light, dark, or system theme is configured in an Optify feature file with the rest of the dashboard behavior.
Sections, commands, buttons, prompts, and other behavior remain in the configuration files.
A single start command launches the local service and opens the dashboard in the browser.

Delivery phases 1 through 7 are implemented.
The repository contains the Axum and React/Vite scaffold, the Options page and browser store, typed Optify configuration loading and reloads, generated schema and transport artifacts, example feature files, embedded release assets, authenticated WebSocket synchronization, configured item discovery with per-command caching and pagination, configured actions and prompts, generic process streaming and cancellation, and the dense dashboard UI.
Configured prompt autocomplete debounces requests, cancels superseded editor processes, bounds suggestions, and requires an explicit user action before applying one.
Button processes remain server-owned across WebSocket reconnects.
The output limit truncates the retained display text while the service continues draining the command to completion.
A compact dashboard control opens `/logs`, which keeps the 100 most recent commands in memory for the lifetime of the local service.
Each entry shows the exact backend-resolved command, status, exit code, and bounded output, so a failed command can be copied and run manually elsewhere.
The dashboard does not persist this history to disk.
Optional [chat](#chat) runs coding-agent conversations from a chat client, with one thread per conversation.

Configure `application.working_directories` as a non-empty ordered list of absolute directories that exist on the dashboard host.
Every issue and pull-request command picker defaults to the first directory and lets the person running it select another configured directory.
The backend accepts only configured values, includes the selected directory in the command preview, and starts the command with that directory as its current working directory.
URL buttons open directly and do not show the picker.

## Button templates

Pull request and issue buttons can fill in these placeholders:

| Placeholder         | Filled-in value                              | `command` | `url` |
| ------------------- | -------------------------------------------- | --------- | ----- |
| `{item.number}`     | Positive pull request or issue number        | Yes       | Yes   |
| `{item.repository}` | Normalized `owner/name` repository identifier | Yes       | Yes   |
| `{item.url}`        | Validated HTTPS URL supplied by the item     | Yes       | Yes   |
| `{prompt}`          | Text entered in the button's prompt          | Yes       | No    |

A command containing `{prompt}` must declare a `prompt`, and a declared prompt requires `{prompt}` in the command.
Prompt labels, placeholders, and optional defaults come directly from that button's configuration.
When `default` is configured, the prompt input and resolved command preview start with that editable value.
Buttons without a configured prompt still show their resolved command before running and do not render prompt controls.
URL buttons cannot declare prompts and must resolve to HTTPS.
Unknown, malformed, or unavailable placeholders are configuration errors.

Set `detached: true` on a command button that only launches work for another application to own.
The dashboard reports an immediate launch failure when it can, otherwise marks the command **Started** after a short startup window and releases its timeout, cancellation, output-streaming, and concurrency ownership.
Later output and exit status belong to the launched application and are not reported by the dashboard.

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
  detached: true
  prompt:
    default: Start in a new work tree
    label: Review focus
    placeholder: Add areas to inspect closely
- label: Open
  url: 'https://example.com/{item.repository}/pull/{item.number}'
```

`{autocomplete.request}` is reserved for the configured autocomplete command and is not available to buttons.

## Chat

The optional `chat` configuration lets you start and continue coding-agent conversations from a chat client, such as a private conversation with yourself in Slack.
The dashboard has no built-in chat or agent integration.
Configured commands listen for messages, run agent turns, and post replies, so the same feature works with any chat client or agent CLI that a script can drive.

The listener is a long-running command that first confirms its connection, then prints one JSON object per message:

```json
{"ready": true}
{"id": "https://example.slack.com/archives/D123/p1700000000000100", "text": "/ai fix the flaky test", "thread": null}
```

`thread` is the `id` of the message that started the thread, or `null` outside a thread.
The listener decides which messages the dashboard may act on, so print only your own messages from a conversation that nobody else can post in.

A start message outside a thread starts a conversation, and the conversation's replies go to that message's thread.
Each later reply in the thread is a prompt for the same conversation.
Command names are configured; these examples use the names from the configuration in the [technical specification](SPEC.md#chat).

| Command           | In a conversation thread                                                    | Outside a thread                              |
| ----------------- | --------------------------------------------------------------------------- | --------------------------------------------- |
| `/ai <prompt>`    | Queues the prompt                                                           | Starts a conversation                         |
| any other text    | Queues the text unchanged, including agent commands such as skills          | Ignored                                       |
| `/steer <prompt>` | Interrupts the running turn and starts the next turn with this prompt first | Explains that steering requires a thread      |
| `/stop`           | Interrupts the running turn and clears the queue                            | Interrupts every turn and clears every queue  |
| `/model [name]`   | Shows or sets the model for the next turn                                   | Shows or sets the model for new conversations |
| `/status`         | Shows the conversation's state, model, and queue                            | Lists running conversations                   |
| `/help`           | Lists the commands                                                          | Lists the commands                            |

Queuing is the default.
A conversation runs one turn at a time, and the next turn delivers every message that arrived in the meantime.
Steering interrupts the agent process with `SIGINT` and then resumes the same session, because a non-interactive agent command cannot accept new text mid-turn.
Start a message with a space when your chat client would otherwise intercept a leading `/`.

The dashboard replies with each turn's final standard output, a failure summary, or a command result.
Every reply starts with `chat.reply.prefix`, and messages that start with the prefix are ignored so the dashboard does not answer itself.
An optional acknowledgement command, such as adding a reaction, confirms that a message was queued.

Chat commands can fill in these placeholders:

| Placeholder                 | Filled-in value                                              | Commands                            |
| --------------------------- | ------------------------------------------------------------ | ----------------------------------- |
| `{conversation.id}`         | ID of the message that started the conversation              | `agent`, `acknowledgement`, `reply` |
| `{conversation.model}`      | Model selected for the turn                                  | `agent`                             |
| `{conversation.session_id}` | Dashboard-generated UUID for starting and resuming a session | `agent`                             |
| `{message.id}`              | ID of the message being acknowledged                         | `acknowledgement`                   |
| `{message.text}`            | Messages delivered by the turn, separated by blank lines     | `agent`                             |
| `{reply.text}`              | Prefixed reply text                                          | `reply`                             |

`agent.start` runs the first turn and `agent.resume` runs later turns with the same `{conversation.session_id}`.
Conversations are kept in memory, so restarting the dashboard forgets them.
Supply ordered `--config-dir` and `--feature` arguments to [validate configuration at startup](CONTRIBUTING.md#install-and-run).
The server keeps the selected setup in memory for its lifetime.
A configured chat listener must report `{"ready": true}` within `listener.startup_timeout_seconds` before the dashboard announces readiness.
On failure, it runs the optional `listener.recovery` command with its own `recovery_timeout_seconds` deadline and retries up to `listener.restart_attempts` times, waiting `listener.restart_seconds` between attempts.
Recovery prints its deadline immediately and elapsed time every 10 seconds.
A recovery wrapper can stream safe JSON progress, repair instructions, and failure reasons using the [recovery protocol](SPEC.md#recovery-protocol).
Instructions appear immediately and remain in the final failure message, including after a timeout or failed reconnection.
A failed repair or exhausted connection attempts stops the dashboard with a nonzero exit and a safe error message.
This also applies when a connected listener later disconnects.
Without startup arguments, the server waits for a browser to supply the Optify setup.

## Documentation

- [Technical specification](SPEC.md)
- [Local development and contributing](CONTRIBUTING.md)
