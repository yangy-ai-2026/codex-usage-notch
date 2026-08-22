<p align="center">
  <img src="assets/quotastrip-mark.svg" width="64" height="64" alt="QuotaStrip mark">
</p>

<h1 align="center">QuotaStrip</h1>

<p align="center"><a href="README.md">English</a> | <a href="README.zh-CN.md">简体中文</a></p>

<p align="center"><strong>Keep Codex usage visible at a glance.</strong><br>A native Windows notch that stays attached to your Codex window.</p>

<p align="center">
  <a href="https://github.com/yangy-ai-2026/quotastrip/releases">Releases</a>&nbsp;&nbsp;·&nbsp;&nbsp;
  <a href="#how-it-works">How it works</a>&nbsp;&nbsp;·&nbsp;&nbsp;
  <a href="https://github.com/yangy-ai-2026/quotastrip/issues">Issues</a>&nbsp;&nbsp;·&nbsp;&nbsp;
  <a href="https://github.com/yangy-ai-2026/quotastrip">GitHub</a>
</p>

QuotaStrip keeps a compact usage notch attached to the Codex Desktop window on Windows, so account allowance information stays close to where you work.

<p align="center">
  <img src="assets/quotastrip-hero-readme.png" width="900" alt="QuotaStrip attached to the top center of a Codex window">
</p>

## Key Features

<sub>Four native behaviors, one window-attached surface.</sub>

<table>
  <tr>
    <td width="50%" valign="top">
      <strong>01&nbsp; / &nbsp;QUICK GLANCE</strong><br><br>
      Instantly see usage near the Codex window without breaking your flow.
    </td>
    <td width="50%" valign="top">
      <strong>02&nbsp; / &nbsp;HOVER TO EXPAND</strong><br><br>
      Expand the notch to inspect remaining percentage, reset time, and data state.
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <strong>03&nbsp; / &nbsp;NATIVE ATTACHMENT</strong><br><br>
      Follow Codex movement, resizing, maximize/restore, and monitor changes.
    </td>
    <td width="50%" valign="top">
      <strong>04&nbsp; / &nbsp;LOCAL-FIRST PRIVACY</strong><br><br>
      No prompt, conversation, or project-code collection. No hidden telemetry.
    </td>
  </tr>
</table>

## Screenshots

### Compact

<sub>A line-sized status surface stays attached to the window top.</sub>

<p align="center">
  <img src="assets/quotastrip-compact.png" width="900" alt="QuotaStrip compact status attached to a Codex window">
</p>

### Hover

<sub>Hover keeps the usage surface available without leaving the window.</sub>

<p align="center">
  <img src="assets/quotastrip-hover.png" width="900" alt="QuotaStrip hover state attached to a Codex window">
</p>

### Click

<sub>Click opens the detailed allowance view.</sub>

<p align="center">
  <img src="assets/quotastrip-click.png" width="900" alt="QuotaStrip clicked detail state attached to a Codex window">
</p>

## Installation

> [!NOTE]
> QuotaStrip `v0.1.0` is available from [QuotaStrip v0.1.0](https://github.com/yangy-ai-2026/quotastrip/releases/tag/v0.1.0).

Download the Windows NSIS installer `QuotaStrip_0.1.0_x64-setup.exe` and verify it against [SHA256SUMS.txt](https://github.com/yangy-ai-2026/quotastrip/releases/download/v0.1.0/SHA256SUMS.txt) before installation. `v0.1.0` is unsigned, so Windows SmartScreen may display a warning; download only from the official Release and verify the SHA256 checksum.

## How It Works

1. QuotaStrip finds the current Codex Desktop window.
2. A native Windows overlay is positioned at that window's top center and tracks its geometry.
3. Usage data is read through Codex-owned local capabilities and presented without collecting prompts, conversations, or project code.

## Architecture

```text
Codex-owned local usage source
        ↓
QuotaStrip usage/runtime layer
        ↓
Native Windows overlay and window tracking
        ↓
Compact / Hover / Click UI
```

QuotaStrip reads allowance data through Codex-owned local capabilities, then normalizes and presents it in a native Windows overlay attached to the Codex window. Compact, Hover, and Click are presentation states of that surface. The local-first boundary excludes prompts, conversations, project code, and application-managed credentials.

## Windows Requirements

- Windows 10 or Windows 11
- Codex Desktop

QuotaStrip is Windows-first. macOS and Linux are outside the `v0.1.0` scope.

## Privacy

QuotaStrip is local-first and does not collect prompts, conversations, or project code. It does not request OpenAI passwords, manage Codex credentials, use OCR or screen scraping, or send hidden telemetry.

Allowance windows are displayed only when returned for the current account. QuotaStrip does not assume fixed quota windows or fabricate missing usage data.

## FAQ

### Does QuotaStrip require an OpenAI API key?

No. Authentication remains with Codex; QuotaStrip does not request or manage OpenAI credentials.

### Does it read my prompts or conversations?

No. QuotaStrip does not collect prompts, conversations, or project code.

### Does it work on macOS or Linux?

No. `v0.1.0` is Windows-first; macOS and Linux are outside its scope.

### What happens when Codex is closed?

QuotaStrip needs a detectable Codex Desktop window to attach its overlay. When Codex is opened again, the window tracker can attach to the new window.

### Where will `v0.1.0` be downloaded?

Download `v0.1.0` from [QuotaStrip v0.1.0](https://github.com/yangy-ai-2026/quotastrip/releases/tag/v0.1.0), the official GitHub Release.

### Why might Windows show a SmartScreen warning?

`v0.1.0` is unsigned. Download only from the official GitHub Release and verify the published SHA256 checksum before trusting an installer.

## Troubleshooting

### QuotaStrip is not visible

Confirm that Codex Desktop is running and has a usable window. Then open the QuotaStrip tray menu and select **Show Overlay**.

### The tray icon is present, but the overlay is not

Use **Show Overlay** first. If the overlay still does not appear, fully quit QuotaStrip from the tray menu, start it again, and ensure that Codex Desktop is open.

### I need to verify an installer

Obtain the installer only from [QuotaStrip v0.1.0](https://github.com/yangy-ai-2026/quotastrip/releases/tag/v0.1.0) and compare it with the accompanying `SHA256SUMS.txt` file.

### Windows warns about an installer

Treat SmartScreen or signing warnings cautiously. Do not bypass a warning unless the installer came from the official GitHub Releases page and its published SHA256 checksum matches.

## Verification and Current Limitations

The Windows CI workflow verifies version consistency, builds the Windows application and NSIS installer, and verifies the installer artifact and SHA256 checksum.

`v0.1.0` is published for Windows. The binary is unsigned, so Windows SmartScreen may display a warning; verify the SHA256 checksum before installation.

## Roadmap

- `v0.1.0` — Codex usage engine, Windows-attached Notch, hover usage panel, tray controls, and a reproducible Windows NSIS installer.
- Later — stability improvements, broader Windows compatibility, and user-feedback-driven enhancements.

See [ROADMAP.md](ROADMAP.md) for the current public roadmap.

## Contributing

Focused contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening an issue or pull request.

## Security

For vulnerability reporting and disclosure guidance, see [SECURITY.md](SECURITY.md).

## License

QuotaStrip is licensed under the [MIT License](LICENSE).

## Disclaimer

**Independent open-source utility for Codex on Windows. Not affiliated with or endorsed by OpenAI.**

“Codex” and “OpenAI” are used only to identify the product this utility is designed to work with. This repository does not claim official API support, certification, sponsorship, or endorsement.
