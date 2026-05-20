#!/usr/bin/env bash
set -euo pipefail

REMOTE_HOST="${REMOTE_HOST:-10.31.0.10}"
REMOTE_USER="${REMOTE_USER:-${USER}}"
REMOTE_DIR="${REMOTE_DIR:-/opt/token-use-backend}"
LOCAL_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARCHIVE="/tmp/token-use-backend-deploy.tar.gz"

tar -C "${LOCAL_ROOT}" -czf "${ARCHIVE}" backend

ssh "${REMOTE_USER}@${REMOTE_HOST}" "mkdir -p '${REMOTE_DIR}'"
scp "${ARCHIVE}" "${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_DIR}/token-use-backend-deploy.tar.gz"
ssh "${REMOTE_USER}@${REMOTE_HOST}" "
  set -euo pipefail
  cd '${REMOTE_DIR}'
  tar -xzf token-use-backend-deploy.tar.gz
  cd backend
  docker compose up -d --build
  docker compose ps
  curl -fsS http://127.0.0.1:6655/healthz
"
