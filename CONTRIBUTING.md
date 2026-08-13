# Contributing to Codex Usage Notch

Thanks for helping build Codex Usage Notch.

## Development Status

`v0.1.0` is in development, and the application foundation is being built. Before proposing implementation work, review the [public roadmap](ROADMAP.md).

## Issues and Feature Requests

- Search existing issues before opening a new one.
- Use the Bug Report template for reproducible defects and the Feature Request template for scoped proposals.
- Keep proposals aligned with the Windows-first, Codex-window-attached `v0.1.0` scope.
- Security vulnerabilities must follow [SECURITY.md](SECURITY.md), not a public issue.

Never submit tokens, cookies, credentials, auth files, private prompts, conversations, account identifiers, or private project code. Sanitize logs and screenshots before attaching them.

## Branches and Pull Requests

- Create a focused branch such as `fix/<short-description>` or `feature/<short-description>`.
- Keep each pull request limited to one concern.
- Explain what changed, why it is needed, and how it was verified.
- Update documentation when a public contract or user-visible behavior changes.
- Include screenshots only for UI changes, and remove private data first.
- Do not claim unimplemented behavior or imply OpenAI affiliation or endorsement.

Maintainers may ask contributors to split broad pull requests or move work to the appropriate roadmap phase.

## Testing

Run the checks relevant to the files you changed and record the exact commands and results in the pull request. New behavior should include tests when a testable implementation exists. Documentation-only changes should verify links, formatting, and terminology. For the current scaffold, run `npm.cmd run build` and `cargo test --manifest-path src-tauri/Cargo.toml` when the local Windows C++ linker is available; record environment limitations separately from code results.

## Security and Privacy

- Do not add credential-reading, prompt-reading, conversation-reading, project-code collection, OCR, screenshot scraping, or webpage DOM scraping as a shortcut.
- Do not persist Codex credentials or copy them into application-owned storage.
- Keep logs minimal and sanitized.
- Never commit `.env` files, auth files, generated diagnostic logs, or account data.

By participating, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
