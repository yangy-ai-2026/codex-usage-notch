# Engineering Rules for Agents and Contributors

This file defines the default engineering rules for the entire repository. Follow the user-approved task scope first, then these rules and the public project documentation.

## 1. Project Scope

- Project: **QuotaStrip**.
- Current release target: `v0.1.0`.
- Platform strategy: Windows-first, targeting Windows 10 and Windows 11.
- Core positioning: **The usage notch that stays attached to your Codex window on Windows.**
- The project is in development. Never describe planned, prototyped, or unverified behavior as implemented, downloadable, released, or production-ready.
- Keep `v0.1.0` focused on Codex usage visibility and the window-attached Notch experience; do not expand it into a general AI usage dashboard.

## 2. Technical Direction

The approved implementation direction is Tauri, React, TypeScript, Rust, Windows Win32/DWM APIs, and Codex app-server.

- The Rust/native layer owns Codex app-server integration, process and binary capability detection, Windows window discovery/tracking, DPI and monitor geometry, lifecycle recovery, and native window behavior.
- React owns presentation, component state, and user interaction for the Notch and usage panel.
- React must not own HWND discovery, WinEvent processing, DPI coordinate conversion, window-follow timing, or lifecycle recovery.
- Keep native and UI contracts explicit so the usage engine and window tracker can be tested without the final visual layer.

## 3. Privacy and Security Boundaries

Do not:

- Read, copy, expose, or persist OpenAI/Codex credentials.
- Read tokens from `auth.json` or equivalent authentication files.
- Read prompts, conversations, or user project code as a usage-data source.
- Use OCR, screenshot scraping, webpage DOM scraping, or CDP injection.
- Modify, inject into, or patch the Codex process or installation.
- Add hidden telemetry or unsanitized diagnostic logging.

Prefer narrowly scoped Codex-owned/local capabilities. Authentication must remain managed by Codex; the application must not become a credential broker or expose arbitrary app-server method passthrough.

## 4. Usage Data Contract

- Never assume every account has fixed `5h + weekly` limits.
- Display only the allowance windows actually returned for the current account.
- Normalize each window by `limitId + windowDurationMins`.
- Treat `primary` and `secondary` as protocol slots, not fixed business meanings.
- Preserve missing windows as `partial`; never fill, copy, or invent quota/reset data.
- Derive `remainingPercent` as `100 - usedPercent`, clamped to valid percentage bounds.
- Keep context/token usage separate from account allowance usage.
- Model and present `fresh`, `stale`, `partial`, `unavailable`, and `error` as distinct states.
- Do not claim server freshness when only a client-side last-successful-read time is available.

## 5. Windows Attachment Contract

The defining behavior is:

```text
Codex window -> Notch attached -> move/resize follows
             -> minimize/close hides -> relaunch reattaches
```

- Use Codex window geometry and event-driven Windows APIs as the primary attachment mechanism.
- Do not use fixed screen coordinates as the primary positioning strategy.
- Do not require users to drag the Notch to keep it aligned with Codex.
- Do not degrade the product into a generic always-on-top desktop widget.
- The system tray may provide secondary controls, but it must not become the primary usage interface.
- Avoid focus stealing, title-bar control obstruction, sustained jitter, and DPI drift.

## 6. Development Rules

- Before starting, read the documents relevant to the requested task and current release scope.
- Change only files required by the active task; avoid unrelated refactors and formatting churn.
- Do not implement work from a later phase without explicit authorization.
- Prefer the smallest implementation that satisfies a verified contract.
- Never invent test results, runtime evidence, compatibility claims, or manual acceptance.
- Record automated verification separately from required manual verification.
- If a required Gate cannot be proven, stop and report the real blocker and current evidence.
- Maintain explicit awareness of Windows 10/11, per-monitor DPI, mixed scaling, multi-monitor behavior, focus, and lifecycle recovery.

## 7. Git Rules

- Agents do not commit, push, create tags, or create GitHub Releases by default.
- Git publication actions remain user-controlled unless the user gives explicit, single-task authorization.
- Do not rewrite, discard, or overwrite unrelated user changes.
- Never submit `.internal/` or any credential, local diagnostic artifact, generated log, or private account data.
- Before handoff, report `git status --short` and distinguish tracked changes from ignored local evidence.

## 8. Public vs. Internal Documentation

- Root-level documentation is public and written for users and contributors.
- `.internal/` is ignored local project-management and validation material; keep it out of public Git history.
- Do not expose private phase workflows, local evidence, or internal experiment details in public README or ROADMAP content.
- Public files must not link to or depend on `.internal/` paths.
- Promote only stable, audience-relevant contracts into public documentation.

## 9. Validation Standard

Every development task handoff must state:

- **Goal**
- **Files changed**
- **Automated verification** and its actual result
- **Manual verification requirement** and whether it was performed
- **Known limitations**
- **Result:** `PASS`, `CONDITIONAL PASS`, or `FAIL`

Use `PASS` only when all required evidence exists. Use `CONDITIONAL PASS` for explicitly bounded, unresolved conditions, and `FAIL` when a required product or engineering contract is not met.
