# Personal Dashboard

Personal Dashboard is a local, configuration-driven view of GitHub pull requests and issues in a normal browser.
It runs customizable local commands to find items, invoke coding agents and other tools, provide prompt autocomplete, stream output, and cancel work in progress.

Pull requests and issues can each have ordered buttons that are always visible and advanced buttons revealed on demand.
Commands stay next to the feature that uses them, and hovering over a button shows its fully resolved command through the native HTML `title` attribute.

Strongly typed Optify configuration can be split into focused files imported by root features.
An Options page lets each person select ordered configuration directories, such as personal and private-work directories, and choose the root features to apply.
The light, dark, or system theme is configured in an Optify feature file with the rest of the dashboard behavior.
Sections, commands, buttons, prompts, repositories, and other behavior remain in the configuration files.
A single start command launches the local service and opens the dashboard in the browser.

Delivery phases 1 through 3 are implemented.
The repository contains the Axum and React/Vite scaffold, the Options page and browser store, typed Optify configuration loading and reloads, generated schema and transport artifacts, example feature files, embedded release assets, and authenticated WebSocket synchronization.
The dashboard item and command workflows remain in later delivery phases.

## Documentation

- [Technical specification](SPEC.md)
- [Local development and contributing](CONTRIBUTING.md)
