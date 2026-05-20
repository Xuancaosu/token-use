## Summary / 概述

<!-- Briefly describe what changed and why it belongs in Token Use. -->
<!-- 简要说明改动内容，以及为什么它属于 Token Use。 -->

## Area / 范围

<!-- Usage, menu bar, GitHub login, leaderboard, backend, docs, packaging, etc. -->
<!-- 用量统计、菜单栏、GitHub 登录、排行、后端、文档、打包等。 -->

## Screenshots / 截图

<!-- Add screenshots for UI changes when possible. -->
<!-- UI 改动尽量附截图。 -->

## Checks / 检查

- [ ] `pnpm typecheck`
- [ ] `pnpm test:unit`
- [ ] `node --check backend/src/server.js` if backend changed
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` if Tauri/Rust changed
- [ ] `pnpm build` if packaging changed

## Privacy / 隐私

- [ ] This change does not upload raw local request logs.
- [ ] This change does not expose tokens, secrets, or personal usage data.
