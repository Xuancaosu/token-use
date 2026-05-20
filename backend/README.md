# Token Use Backend

Lightweight leaderboard backend for the Token Use macOS app.

## Runtime

- HTTP port: `6655`
- Public domain: `https://token-use.lucsun.cn`
- Health check: `GET /healthz`
- Persistent data: `/data/token-use-backend.json`

## Docker

```bash
docker compose up -d --build
curl http://127.0.0.1:6655/healthz
```

## Current Deployment

The current server deployment target is:

- host: `10.31.0.10`
- public domain: `https://token-use.lucsun.cn`
- remote directory: `/opt/token-use-backend`
- exposed Docker port: `6655`

Deploy from the repository root:

```bash
REMOTE_USER=root ./scripts/deploy-token-use-backend.sh
```

## API

- `POST /api/github/device/start`
- `POST /api/github/device/poll`
- `GET /api/me`
- `POST /api/me/opt-in`
- `POST /api/leaderboard/snapshot`
- `GET /api/leaderboard?range=today|7d|30d`

All endpoints except GitHub device login and health check require
`Authorization: Bearer <token>`.
