# Security Policy

## Supported Versions

Token Use is in active early development. Security fixes are applied to the latest commit and latest release line only.

| Version | Supported |
| --- | --- |
| Latest 0.x | Yes |
| Older snapshots | No |

## Reporting a Vulnerability

Please do not report security vulnerabilities through public GitHub issues.

Use GitHub Security Advisories for this repository when available. If advisories are not enabled yet, contact the maintainer privately and include:

- vulnerability summary
- affected version or commit
- reproduction steps
- expected impact
- whether the issue affects local data, GitHub login, backend tokens, or leaderboard privacy

## Areas of Interest

Security-sensitive areas include:

- GitHub Device Flow login
- backend bearer tokens
- leaderboard opt-in state
- uploaded aggregate usage snapshots
- local SQLite usage data
- app update metadata

## Disclosure

Please allow time for triage and a fix before public disclosure. Reporters can be credited in release notes unless they prefer to remain anonymous.
