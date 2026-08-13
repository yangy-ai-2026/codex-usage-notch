# Security Policy

## Supported Versions

There is no public application release yet. Security fixes are accepted for the current development line only. This policy will be updated when versioned releases are published.

| Version | Supported |
|---|---|
| Unreleased development line | Yes, best effort |
| Published releases | None yet |

## Reporting a Vulnerability

After the GitHub repository is created, use GitHub's private vulnerability reporting feature under the repository's **Security** tab. Include a concise description, affected component, reproduction steps, and impact.

If private vulnerability reporting is not yet available, open a public issue containing only a request for a private maintainer contact. Do **not** include vulnerability details or sensitive data in that issue.

Please allow maintainers reasonable time to acknowledge, investigate, and coordinate a fix before public disclosure. Response targets will be documented after the maintainer channel and release process exist.

## Never Share Publicly

Do not paste or upload any of the following to an Issue, Discussion, pull request, log, or screenshot:

- Tokens, cookies, passwords, API keys, or credentials.
- Codex or OpenAI auth files, including `auth.json` or equivalent files.
- Account identifiers, private email addresses, or billing data.
- Private prompts, conversations, session content, or project code.
- Unsanitized logs or dumps that may contain any of the above.

If sensitive data is posted accidentally, revoke or rotate it immediately and ask a maintainer to remove the content from public history.

## Privacy Boundary

Codex Usage Notch is designed not to read or collect prompts, conversations, or project code. Its usage-data design does not require OCR, screenshots, webpage DOM scraping, an OpenAI password, or application-managed credential storage.

The planned application will delegate authentication to a Codex-owned local process. That process may use Codex's existing login internally; this project must not expose arbitrary protocol passthrough or copy the underlying credentials.

## Scope

Security reports may include credential exposure, unsafe logging, unintended data collection, insecure update or packaging behavior, privilege-boundary problems, and vulnerabilities in the window-attachment or usage-data components. General bugs and feature requests should use the public issue templates.
