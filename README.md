# Codex Usage Notch

> The usage notch that stays attached to your Codex window on Windows.

**Status:** `v0.1.0 — Phase 5 CLOSED / PASS; release work remains`

Codex Usage Notch is an early-stage, lightweight Windows utility that keeps account usage information close to the Codex Desktop window. The v0.1.0 application foundation, usage engine, and Phase 5 native Notch foundation are validated locally; no public build is currently available.

## Why

Checking Codex usage currently interrupts the coding flow. Existing monitors commonly live in a system tray, taskbar, global floating widget, or separate dashboard. This project explores a smaller interaction: usage information that remains visually attached to the Codex window and disappears when Codex does.

## Planned Core Experience

The intended `v0.1.0` experience is:

1. Detect the current Codex Desktop window on Windows.
2. Attach a compact usage notch to its top center.
3. Follow move, resize, maximize, restore, and monitor changes.
4. Hide when Codex is minimized, closed, or unavailable.
5. Expand on hover to show the allowance windows actually returned for the current account, including remaining percentage, reset time, and freshness state.

Allowance windows will be identified dynamically by `limitId + windowDurationMins`. The application will not assume that every account has fixed 5-hour and weekly windows, and it will not fabricate missing data.

## Windows First

The initial target is Windows 10 and Windows 11. macOS and Linux are outside the `v0.1.0` scope. The window-attached behavior—not a generic always-on-top widget—is the central product difference.

## Current Technical Status

- The product is in early development.
- The core usage-data and Windows window-attachment approaches have been validated locally.
- The Tauri/React application foundation, Codex-owned usage engine, and native Windows Notch foundation are validated through Phase 5.
- A public production build, installer, and release do not exist yet.

## Privacy Principles

The planned application is local-first and will follow these boundaries:

- No prompt collection.
- No conversation collection.
- No project-code collection.
- No hidden telemetry.
- No OpenAI password request.
- No application-managed storage of Codex tokens or credentials.
- No OCR, screenshots, or webpage DOM scraping as a usage-data source.
- Logs and diagnostics must be sanitized before sharing.

See [SECURITY.md](SECURITY.md) for reporting and disclosure guidance.

## Development Status

There is no installer or downloadable release yet. Current work is focused on the first public version described in the [public roadmap](ROADMAP.md).

## Contributing

The project is at an early foundation stage. Issues and focused proposals are welcome, but contributors should not present planned behavior as already implemented. Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening an issue or pull request.

## License

This project is licensed under the [MIT License](LICENSE).

## Disclaimer

**Independent open-source utility for Codex on Windows. Not affiliated with or endorsed by OpenAI.**

“Codex” and “OpenAI” are used only to identify the product this utility is designed to work with. This repository does not claim official API support, certification, sponsorship, or endorsement.
