# Contributing to Token Use

Thank you for helping improve Token Use. This project is a focused fork of CC Switch and is still early, so contributions should stay small, clear, and aligned with the usage-monitoring scope.

## Scope

Good contributions include:

- macOS UI polish for usage and leaderboard workflows
- usage aggregation fixes
- GitHub login and opt-in ranking fixes
- backend deployment and API hardening
- documentation improvements
- focused tests around changed behavior

Out of scope for this fork:

- provider switching
- API key relay features
- MCP, prompt, or skill management
- multi-tool account management
- unrelated CC Switch feature restoration

## Development Setup

Requirements:

- Node.js 20 or compatible
- pnpm
- Rust 1.85 or later
- Tauri 2 macOS prerequisites
- Docker, if you work on the leaderboard backend

Install dependencies:

```bash
pnpm install
```

Run the desktop app:

```bash
pnpm dev
```

Run the backend locally:

```bash
cd backend
docker compose up -d --build
curl http://127.0.0.1:6655/healthz
```

## Checks

Run the checks that match your change:

```bash
pnpm typecheck
pnpm test:unit
node --check backend/src/server.js
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

For a release build:

```bash
pnpm build
```

## Pull Requests

- Keep pull requests focused on one behavior or documentation area.
- Explain why the change belongs in Token Use.
- Include screenshots for UI changes when possible.
- Add or update tests when behavior changes.
- Do not include secrets, private tokens, or personal usage data.

## AI-Assisted Contributions

AI tools are fine, but the contributor is responsible for the result. Please read the diff, run the relevant checks, and be ready to explain the code you submit.

## Attribution

Token Use is forked from [CC Switch](https://github.com/farion1231/cc-switch). Please preserve original license and attribution notices when editing inherited code.
