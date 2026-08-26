# Personal Dashboard Development Notes

The application has not been scaffolded yet.
The commands below define the intended contributor workflow and will become available as the implementation described in [SPEC.md](SPEC.md) lands.

## Prerequisites

- Install the current stable Rust toolchain with `rustup`.
- Install Node.js and `pnpm`; supported versions will be pinned in the repository toolchain files.
- Install and authenticate the [GitHub CLI](https://cli.github.com/manual/).
- Install any coding agents or other local programs referenced by the selected configuration directories.
- Install the recommended VS Code extensions for YAML diagnostics and completion.

The dashboard reuses the authentication already owned by each configured command.
Do not put tokens, credentials, or secret fallback values in the YAML file or repository environment files.

## Install and run

Run commands from the existing repository working directory.

```sh
pnpm --dir dashboard install
pnpm --dir dashboard run dev
```

`pnpm --dir dashboard run dev` starts Axum and Vite together and stops both if either fails.
Vite proxies `/bootstrap` and `/ws` to Axum, so contributors do not need a second terminal or a separately managed backend.
Open the loopback URL printed by Vite in a normal browser.
The development origin uses a stable port so Optify setup persists across restarts.
If that port is occupied, stop the conflicting process and rerun the command rather than switching to a random port.

On first use, open the Options page and add one or more absolute configuration-directory paths under **Optify directories**.
For example, the first directory might contain personal configuration while the second contains private-work configuration.
Then add the ordered root feature names under **Optify features** and select **Apply Optify configuration**.
The browser persists only the directory `string[]` and feature `string[]` in `localStorage`; theme, sections, buttons, commands, repositories, prompts, and all other options remain in Optify files.

Every program named by an inline command must be available to the process that starts the server.
Missing executables, required environment variables, authentication, and configuration values must produce clear failures rather than implicit fallbacks.

Check GitHub authentication independently with:

```sh
gh auth status
```

## Configuration files

Edit YAML, YML, or JSON feature files in any directory selected on the Options page to change the theme, sections, discovery commands, repository paths, prompt autocomplete, and pull request or issue buttons.
Optify discovers supported files recursively in every selected directory.
Keep a root feature small and use it to import focused features that each own one area of configuration.
Each focused file should normally be imported by one root feature so its ownership and path are obvious.

The repository example uses this layout:

```text
dashboard/configs/
  dashboard.yaml
  dashboard/
    appearance.yaml
    application.yaml
    autocomplete.yaml
    buttons.yaml
    repositories.yaml
    sections.yaml
```

`dashboard/configs/dashboard.yaml` groups the focused files by canonical path-derived feature name:

```yaml
imports:
  - dashboard/appearance
  - dashboard/application
  - dashboard/autocomplete
  - dashboard/buttons
  - dashboard/repositories
  - dashboard/sections
```

Do not add aliases that only restate a canonical path with slightly different punctuation.
A focused appearance feature tracks the theme with a strongly typed value:

```yaml
options:
  appearance:
    theme: system
```

Use `light`, `dark`, or `system`.
The `system` value follows the operating-system preference.

A personal directory and private-work directory can each have their own root feature and focused imports.
Optify applies imports in their listed order before the importing root's own options.
Select those roots in the order they should be applied.
Optify recursively merges objects, while each later scalar or array replaces the earlier value.
Reordering roots can therefore replace an entire `sections`, `always`, or `advanced` list.

Command values are YAML strings and should remain next to the feature that runs them.
Use YAML's folded block style for long commands:

```yaml
command: >-
  gh search prs
  "author:@me state:open"
  --json number,repository,state,title,updatedAt,url
```

Do not introduce a named command registry merely to deduplicate similar strings.
Keeping a complete command at its point of use makes personal configuration easier to read and change.

## Schema and live reload

Optify validates every discovered feature file and exposes the options merged from the selected root features to the strongly typed Rust configuration layer.
The Rust configuration types and their Optify feature-file wrapper are the source of truth for the generated JSON Schema at `dashboard/configs/.optify/schema.json`.
After changing a configuration type, regenerate the schema:

```sh
pnpm --dir dashboard run schema:generate
```

Commit the Rust change and updated `dashboard/configs/.optify/schema.json` together.
Do not edit the generated schema manually.
`pnpm --dir dashboard run check` fails when generated schema or TypeScript bindings differ from the committed files.

The schema location follows Optify's [custom-schema recommendation](https://github.com/juharris/optify/blob/main/README.md#custom-schemas).
The repository `.vscode/settings.json` maps it to `dashboard/configs/**/*.json` and `dashboard/configs/**/*.{yaml,yml}`.
VS Code should report unknown fields, missing values, and type errors in every configuration feature file.
A personal or private-work configuration repository outside this workspace should associate the same generated schema with its own feature-file globs in that repository's VS Code settings.

The backend passes the ordered Optify directories to [`OptionsWatcher::build_from_directories_with_schema_and_options`](https://docs.rs/optify/1.3.3/optify/provider/struct.OptionsWatcher.html#method.build_from_directories_with_schema_and_options) with the generated schema and passes the ordered feature list to `get_all_options`.
Its [`add_listener`](https://docs.rs/optify/1.3.3/optify/provider/struct.OptionsWatcher.html#method.add_listener) callback triggers an application configuration reload and a WebSocket UI refresh after Optify successfully rebuilds the provider.
An invalid edit leaves the previous provider and UI configuration active; correct the editor diagnostic or server-log error and save again.

Changing Optify directories or features creates and validates a candidate watcher before replacing the active one.
An invalid directory, feature, or merged configuration leaves the previous Optify setup, watcher, and UI configuration active.
Editing the imported appearance feature updates connected tabs after the watcher accepts the configuration reload.

## Checks

Run the aggregate quality gate after making changes:

```sh
pnpm --dir dashboard run check
```

Run the test suites without all static checks while iterating:

```sh
pnpm --dir dashboard run test
```

Build and start the production-style executable before handing off startup or embedded-asset changes:

```sh
pnpm --dir dashboard run build
pnpm --dir dashboard run start
```

`pnpm --dir dashboard run start` launches the compiled Rust server, serves the embedded frontend, and opens its loopback URL.

Backend tests must use fake executables and checked-in command-output fixtures.
They must not require live GitHub or model-provider access.

## Development conventions

- Keep Axum handlers thin and put behavior in focused Rust structs and services.
- Keep WebSocket lifecycle, message routing, command execution, and presentation formatting behind separate interfaces.
- Add members, methods, modules, routes, and configuration fields in mostly alphabetical order.
- Keep provider-specific behavior in inline YAML commands rather than Rust branches, action kinds, or output types.
- Compile placeholders into Bash positional parameters so item and prompt values never become executable script text.
- Resolve button previews from the same compiled command invocation used for execution.
- Preserve the Git staging area; do not stage generated files or other changes automatically.
- Reuse meaningful fixtures and helpers instead of duplicating nearly identical tests.

## Troubleshooting

If configuration diagnostics do not appear, confirm that the workspace is opened at the repository root and that `redhat.vscode-yaml` is enabled.

If a configured program works in a terminal but not in the dashboard, inspect the environment of the process that started the server.
Launchers and non-interactive processes may not inherit the same `PATH` as a terminal.

If startup rejects the configuration, fix the first schema or template error and rerun `pnpm --dir dashboard run dev`.
The application does not substitute sample configuration, guessed paths, or missing environment values.
