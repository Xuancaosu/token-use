# Usage-Only Token Use Implementation Notes

## 2026-05-20 Baseline

- Branch: `feature/usage-only-token-use`
- Dependency install: `pnpm install` completed. Registry downloads retried after transient `ECONNRESET` and then succeeded.
- Frontend typecheck: `pnpm typecheck` passed.
- Frontend unit tests: `pnpm test:unit` passed, 43 files and 241 tests.
- Rust toolchain: installed Rust 1.95.0 via Homebrew `rustup` because `cargo` was not available in the initial PATH.
- Rust tests: `PATH="/Users/lucsun-authing/.homebrew/opt/rustup/bin:$PATH" cargo test --manifest-path src-tauri/Cargo.toml` passed, including 1207 library tests and integration suites.

## Implementation Decisions

- Work is being done in the scratch clone at `/Users/lucsun-authing/Documents/Codex/2026-05-20/cc-swich-macos-app-token-7/cc-switch`.
- The existing usage ingestion and aggregation code remains the source of truth.
- GitHub login uses GitHub OAuth Device Flow for the desktop app. This avoids embedding a client secret in the app; only a public OAuth Client ID is needed.
- Normal local packaging does not generate updater artifacts by default. The updater public key and endpoint are kept, but production updater signing should be re-enabled only when `TAURI_SIGNING_PRIVATE_KEY` is available.

## 2026-05-20 Implementation Result

- App identity changed to `Token Use`, with bundle identifier `com.lucsun.token-use`, deep link scheme `tokenuse`, default config dir `~/.token-use`, DB file `token-use.db`, and log file `token-use.log`.
- Frontend entry changed from the old multi-tool CC Switch surface to `TokenMonitorShell`.
- Primary navigation now has only two pages: `Usage` and `Leaderboard`.
- Usage page is usage-only: today / 7 days / 30 days range switch, no traditional tabs, no app/software selector on the main path, and no leaderboard content on the page.
- Leaderboard page is separate from usage. Signed-out users only see the GitHub login gate; signed-in users can opt in/out and view today / 7 days / 30 days rankings.
- Ranking is computed from total token consumption: input + output + cache read + cache creation. Cost is displayed as auxiliary data and does not affect rank.
- SQLite schema version moved to 11 and added `ranking_profile`, `ranking_auth_session`, and `ranking_snapshots`.
- Tauri command allowlist was reduced to usage, leaderboard, init error, session sync, and theme commands.
- Tray menu was reduced to show main window and quit; provider switching and related visible actions were removed.
- GitHub login now starts a Device Flow session with the registered `Token Use` OAuth App Client ID `Ov23li3Cs7WS1Fpu523D`, opens `https://github.com/login/device`, displays the user code in the app, polls GitHub for authorization, reads `/user`, and writes the GitHub profile into the local ranking profile table.
- Runtime `TOKEN_USE_GITHUB_CLIENT_ID` still takes precedence, with `TOKEN_MONITOR_GITHUB_CLIENT_ID` accepted as a temporary compatibility fallback.
- Leaderboard backend defaults to `https://token-use.lucsun.cn`; runtime `TOKEN_USE_BACKEND_URL` can override it. The backend Docker service listens on port `6655` and exposes GitHub Device Flow, profile opt-in, snapshot upload, and leaderboard query APIs.
- The app bundle, package, binary, config directory, DB file, log file, theme key, tray id, and updater endpoint now use the `token-use` namespace.
- App icons were replaced with deterministic Token Use vector-derived assets, including PNG sizes, `icon.icns`, `icon.ico`, and the macOS status bar template icon.
- The macOS status bar item now displays today's token usage as a compact title such as `999`, `1.2K`, `12.4M`, or `1.2B`. It refreshes on startup, every 60 seconds, and after manual session usage sync.

## 2026-05-20 Final Verification

- `pnpm typecheck` passed.
- `pnpm exec vitest run tests/components/UsagePage.test.tsx tests/components/LeaderboardPage.test.tsx tests/components/TokenMonitorShell.test.tsx` passed, 3 files and 3 tests.
- `pnpm test:unit` passed, 46 files and 242 tests. Existing warning logs from legacy settings/import tests remain non-fatal.
- `PATH="/Users/lucsun-authing/.homebrew/opt/rustup/bin:$PATH" cargo test --manifest-path src-tauri/Cargo.toml` passed, 1212 library tests plus all integration suites.
- `pnpm build:renderer` passed.
- `PATH="/Users/lucsun-authing/.homebrew/opt/rustup/bin:$PATH" pnpm build` passed and produced:
  - `/Users/lucsun-authing/Documents/Codex/2026-05-20/cc-swich-macos-app-token-7/cc-switch/src-tauri/target/release/bundle/macos/Token Use.app`
  - `/Users/lucsun-authing/Documents/Codex/2026-05-20/cc-swich-macos-app-token-7/cc-switch/src-tauri/target/release/bundle/dmg/Token Use_0.1.0_aarch64.dmg`
- Renderer dev server is running at `http://127.0.0.1:3107/`. `curl --noproxy '*'` returned Vite HTML successfully.
- `node --check backend/src/server.js` passed.
- `docker build -t token-use-backend:local backend` passed.
- `docker run -p 7665:6655 token-use-backend:local` plus `curl http://127.0.0.1:7665/healthz` returned `{"ok":true,"service":"token-use-backend",...}`.
- Remote backend deployment to `10.31.0.10` completed with `REMOTE_USER=root ./scripts/deploy-token-use-backend.sh`.
- Remote Docker container `token-use-backend` is healthy and exposes `0.0.0.0:6655->6655/tcp`.
- Public HTTPS checks passed for `https://token-use.lucsun.cn/healthz` and `POST https://token-use.lucsun.cn/api/github/device/start`.
