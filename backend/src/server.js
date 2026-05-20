import crypto from "node:crypto";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { URL } from "node:url";

const PORT = Number(process.env.PORT || "6655");
const HOST = process.env.HOST || "0.0.0.0";
const DATA_DIR = process.env.DATA_DIR || "/data";
const DATA_FILE = process.env.DATA_FILE || path.join(DATA_DIR, "token-use-backend.json");
const GITHUB_CLIENT_ID =
  process.env.GITHUB_OAUTH_CLIENT_ID || process.env.TOKEN_USE_GITHUB_CLIENT_ID || "Ov23li3Cs7WS1Fpu523D";
const TOKEN_TTL_SECONDS = Number(process.env.TOKEN_TTL_SECONDS || 60 * 60 * 24 * 90);
const CORS_ORIGIN = process.env.CORS_ORIGIN || "*";

const GITHUB_DEVICE_CODE_URL = "https://github.com/login/device/code";
const GITHUB_ACCESS_TOKEN_URL = "https://github.com/login/oauth/access_token";
const GITHUB_USER_URL = "https://api.github.com/user";
const GITHUB_DEVICE_GRANT_TYPE = "urn:ietf:params:oauth:grant-type:device_code";
const VALID_RANGES = new Set(["today", "7d", "30d"]);

let writeQueue = Promise.resolve();

function nowSeconds() {
  return Math.floor(Date.now() / 1000);
}

function emptyStore() {
  return {
    schemaVersion: 1,
    users: {},
    sessions: {},
    tokens: {},
    snapshots: {
      today: {},
      "7d": {},
      "30d": {},
    },
  };
}

async function loadStore() {
  try {
    const raw = await fs.readFile(DATA_FILE, "utf8");
    const parsed = JSON.parse(raw);
    return {
      ...emptyStore(),
      ...parsed,
      users: parsed.users || {},
      sessions: parsed.sessions || {},
      tokens: parsed.tokens || {},
      snapshots: {
        ...emptyStore().snapshots,
        ...(parsed.snapshots || {}),
      },
    };
  } catch (error) {
    if (error.code === "ENOENT") return emptyStore();
    throw error;
  }
}

async function saveStore(store) {
  await fs.mkdir(path.dirname(DATA_FILE), { recursive: true });
  const tmp = `${DATA_FILE}.${process.pid}.tmp`;
  await fs.writeFile(tmp, `${JSON.stringify(store, null, 2)}\n`);
  await fs.rename(tmp, DATA_FILE);
}

async function updateStore(mutator) {
  let result;
  writeQueue = writeQueue.then(async () => {
    const store = await loadStore();
    cleanupStore(store);
    result = await mutator(store);
    await saveStore(store);
  });
  await writeQueue;
  return result;
}

function cleanupStore(store) {
  const now = nowSeconds();
  for (const [id, session] of Object.entries(store.sessions)) {
    if ((session.expiresAt || 0) + 3600 < now) delete store.sessions[id];
  }
  for (const [hash, token] of Object.entries(store.tokens)) {
    if ((token.expiresAt || 0) < now) delete store.tokens[hash];
  }
}

function tokenHash(token) {
  return crypto.createHash("sha256").update(token).digest("hex");
}

function publicProfile(user) {
  return {
    userId: user.userId,
    githubLogin: user.githubLogin,
    displayName: user.displayName,
    avatarUrl: user.avatarUrl || null,
    optedIn: Boolean(user.optedIn),
    joinedAt: user.joinedAt,
  };
}

function setCors(res) {
  res.setHeader("Access-Control-Allow-Origin", CORS_ORIGIN);
  res.setHeader("Access-Control-Allow-Methods", "GET,POST,DELETE,OPTIONS");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type,Authorization");
}

function sendJson(req, res, status, payload) {
  setCors(res);
  res.statusCode = status;
  res.setHeader("Content-Type", "application/json; charset=utf-8");
  res.end(JSON.stringify(payload));
  console.log(`${new Date().toISOString()} ${req.method} ${req.url} ${status}`);
}

function sendError(req, res, status, message, code = "error") {
  sendJson(req, res, status, { error: code, message });
}

async function readJson(req) {
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  if (chunks.length === 0) return {};
  const raw = Buffer.concat(chunks).toString("utf8");
  if (!raw.trim()) return {};
  return JSON.parse(raw);
}

function assertRange(range) {
  if (!VALID_RANGES.has(range)) {
    const error = new Error(`Unsupported leaderboard range: ${range}`);
    error.status = 400;
    throw error;
  }
}

function normalizeSnapshot(input) {
  assertRange(input.range);
  const snapshot = {
    range: input.range,
    windowStart: Number(input.windowStart || 0),
    windowEnd: Number(input.windowEnd || 0),
    inputTokens: Math.max(0, Number(input.inputTokens || 0)),
    outputTokens: Math.max(0, Number(input.outputTokens || 0)),
    cacheReadTokens: Math.max(0, Number(input.cacheReadTokens || 0)),
    cacheCreationTokens: Math.max(0, Number(input.cacheCreationTokens || 0)),
    requestCount: Math.max(0, Number(input.requestCount || 0)),
    totalCostUsd: String(input.totalCostUsd ?? "0"),
  };
  snapshot.totalTokens =
    snapshot.inputTokens +
    snapshot.outputTokens +
    snapshot.cacheReadTokens +
    snapshot.cacheCreationTokens;
  return snapshot;
}

async function githubFormPost(url, fields) {
  const body = new URLSearchParams(fields);
  const response = await fetch(url, {
    method: "POST",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/x-www-form-urlencoded",
      "User-Agent": "Token Use",
    },
    body,
  });
  const text = await response.text();
  let json;
  try {
    json = JSON.parse(text);
  } catch {
    json = { raw: text };
  }
  if (!response.ok) {
    const error = new Error(`GitHub request failed: ${response.status}`);
    error.status = 502;
    error.body = json;
    throw error;
  }
  return json;
}

async function fetchGithubUser(accessToken) {
  const response = await fetch(GITHUB_USER_URL, {
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${accessToken}`,
      "User-Agent": "Token Use",
      "X-GitHub-Api-Version": "2022-11-28",
    },
  });
  const json = await response.json();
  if (!response.ok) {
    const error = new Error(`GitHub user request failed: ${response.status}`);
    error.status = 502;
    error.body = json;
    throw error;
  }
  return json;
}

async function requireAuth(req) {
  const value = req.headers.authorization || "";
  const match = value.match(/^Bearer\s+(.+)$/i);
  if (!match) {
    const error = new Error("Missing bearer token");
    error.status = 401;
    throw error;
  }
  const hash = tokenHash(match[1]);
  const store = await loadStore();
  cleanupStore(store);
  const token = store.tokens[hash];
  if (!token || token.expiresAt < nowSeconds()) {
    const error = new Error("Invalid or expired bearer token");
    error.status = 401;
    throw error;
  }
  const user = store.users[token.userId];
  if (!user) {
    const error = new Error("Authenticated user no longer exists");
    error.status = 401;
    throw error;
  }
  return { store, user, tokenHash: hash };
}

async function handleStartGithubLogin(req, res) {
  const device = await githubFormPost(GITHUB_DEVICE_CODE_URL, {
    client_id: GITHUB_CLIENT_ID,
    scope: "read:user",
  });
  const id = crypto.randomUUID();
  const expiresAt = nowSeconds() + Number(device.expires_in || 900);
  await updateStore((store) => {
    store.sessions[id] = {
      id,
      deviceCode: device.device_code,
      expiresAt,
      createdAt: nowSeconds(),
      completedAt: null,
      authToken: null,
      userId: null,
      error: null,
    };
  });
  sendJson(req, res, 200, {
    sessionId: id,
    userCode: device.user_code,
    verificationUri: device.verification_uri,
    expiresAt,
    intervalSeconds: Number(device.interval || 5),
  });
}

async function handlePollGithubLogin(req, res) {
  const { sessionId } = await readJson(req);
  if (!sessionId || typeof sessionId !== "string") {
    return sendError(req, res, 400, "sessionId is required", "invalid_request");
  }

  const store = await loadStore();
  const session = store.sessions[sessionId];
  if (!session) return sendError(req, res, 404, "Login session not found", "not_found");
  if (session.completedAt && session.userId) {
    const user = store.users[session.userId];
    return sendJson(req, res, 200, {
      status: "completed",
      profile: user ? publicProfile(user) : null,
      authToken: session.authToken,
      retryAfterSeconds: null,
      message: null,
    });
  }
  if (session.expiresAt <= nowSeconds()) {
    await updateStore((next) => {
      if (next.sessions[sessionId]) next.sessions[sessionId].error = "expired";
    });
    return sendJson(req, res, 200, {
      status: "expired",
      profile: null,
      authToken: null,
      retryAfterSeconds: null,
      message: "GitHub device code expired.",
    });
  }

  const token = await githubFormPost(GITHUB_ACCESS_TOKEN_URL, {
    client_id: GITHUB_CLIENT_ID,
    device_code: session.deviceCode,
    grant_type: GITHUB_DEVICE_GRANT_TYPE,
  });

  if (token.error) {
    if (token.error === "authorization_pending" || token.error === "slow_down") {
      return sendJson(req, res, 200, {
        status: "pending",
        profile: null,
        authToken: null,
        retryAfterSeconds: Number(token.interval || (token.error === "slow_down" ? 10 : 5)),
        message: token.error_description || null,
      });
    }
    const status = token.error === "access_denied" ? "denied" : "expired";
    await updateStore((next) => {
      if (next.sessions[sessionId]) next.sessions[sessionId].error = token.error;
    });
    return sendJson(req, res, 200, {
      status,
      profile: null,
      authToken: null,
      retryAfterSeconds: null,
      message: token.error_description || token.error,
    });
  }

  if (!token.access_token) {
    return sendError(req, res, 502, "GitHub response did not include access_token", "github_error");
  }

  const githubUser = await fetchGithubUser(token.access_token);
  const userId = `github:${githubUser.id}`;
  const authToken = crypto.randomBytes(32).toString("base64url");
  const expiresAt = nowSeconds() + TOKEN_TTL_SECONDS;
  const result = await updateStore((next) => {
    const previous = next.users[userId];
    const user = {
      userId,
      githubLogin: githubUser.login,
      displayName: githubUser.name || githubUser.login,
      avatarUrl: githubUser.avatar_url || null,
      optedIn: Boolean(previous?.optedIn),
      joinedAt: previous?.joinedAt || nowSeconds(),
      updatedAt: nowSeconds(),
    };
    next.users[userId] = user;
    next.tokens[tokenHash(authToken)] = {
      userId,
      createdAt: nowSeconds(),
      expiresAt,
    };
    next.sessions[sessionId] = {
      ...next.sessions[sessionId],
      completedAt: nowSeconds(),
      userId,
      authToken,
      error: null,
    };
    return { user, authToken };
  });

  sendJson(req, res, 200, {
    status: "authorized",
    profile: publicProfile(result.user),
    authToken: result.authToken,
    retryAfterSeconds: null,
    message: null,
  });
}

async function handleGetMe(req, res) {
  const { user } = await requireAuth(req);
  sendJson(req, res, 200, { profile: publicProfile(user) });
}

async function handleOptIn(req, res) {
  const { user } = await requireAuth(req);
  const { optedIn } = await readJson(req);
  const updated = await updateStore((store) => {
    const next = store.users[user.userId];
    next.optedIn = Boolean(optedIn);
    next.updatedAt = nowSeconds();
    return next;
  });
  sendJson(req, res, 200, { profile: publicProfile(updated) });
}

async function handleSnapshot(req, res) {
  const { user } = await requireAuth(req);
  const snapshot = normalizeSnapshot(await readJson(req));
  const saved = await updateStore((store) => {
    store.snapshots[snapshot.range][user.userId] = {
      ...snapshot,
      userId: user.userId,
      snapshotAt: nowSeconds(),
    };
    return store.snapshots[snapshot.range][user.userId];
  });
  sendJson(req, res, 200, { snapshot: saved });
}

async function handleLeaderboard(req, res, url) {
  const { user } = await requireAuth(req);
  const range = url.searchParams.get("range") || "today";
  assertRange(range);
  const store = await loadStore();
  const rows = Object.values(store.snapshots[range] || {})
    .map((snapshot) => {
      const rowUser = store.users[snapshot.userId];
      if (!rowUser?.optedIn) return null;
      return {
        ...snapshot,
        ...publicProfile(rowUser),
      };
    })
    .filter(Boolean)
    .sort((a, b) => {
      if (b.totalTokens !== a.totalTokens) return b.totalTokens - a.totalTokens;
      if (b.requestCount !== a.requestCount) return b.requestCount - a.requestCount;
      return a.githubLogin.localeCompare(b.githubLogin);
    })
    .slice(0, 100)
    .map((entry, index) => ({
      rank: index + 1,
      userId: entry.userId,
      githubLogin: entry.githubLogin,
      displayName: entry.displayName,
      avatarUrl: entry.avatarUrl,
      totalTokens: entry.totalTokens,
      inputTokens: entry.inputTokens,
      outputTokens: entry.outputTokens,
      cacheReadTokens: entry.cacheReadTokens,
      cacheCreationTokens: entry.cacheCreationTokens,
      requestCount: entry.requestCount,
      totalCostUsd: entry.totalCostUsd,
      isCurrentUser: entry.userId === user.userId,
    }));

  sendJson(req, res, 200, {
    range,
    updatedAt: rows.reduce((max, row) => Math.max(max, row.snapshotAt || 0), 0) || null,
    entries: rows,
  });
}

async function route(req, res) {
  setCors(res);
  if (req.method === "OPTIONS") {
    res.statusCode = 204;
    return res.end();
  }

  const url = new URL(req.url || "/", `http://${req.headers.host || "localhost"}`);
  if (req.method === "GET" && url.pathname === "/healthz") {
    return sendJson(req, res, 200, {
      ok: true,
      service: "token-use-backend",
      time: nowSeconds(),
    });
  }
  if (req.method === "POST" && url.pathname === "/api/github/device/start") {
    return handleStartGithubLogin(req, res);
  }
  if (req.method === "POST" && url.pathname === "/api/github/device/poll") {
    return handlePollGithubLogin(req, res);
  }
  if (req.method === "GET" && url.pathname === "/api/me") {
    return handleGetMe(req, res);
  }
  if (req.method === "POST" && url.pathname === "/api/me/opt-in") {
    return handleOptIn(req, res);
  }
  if (req.method === "POST" && url.pathname === "/api/leaderboard/snapshot") {
    return handleSnapshot(req, res);
  }
  if (req.method === "GET" && url.pathname === "/api/leaderboard") {
    return handleLeaderboard(req, res, url);
  }
  return sendError(req, res, 404, "Not found", "not_found");
}

const server = http.createServer((req, res) => {
  route(req, res).catch((error) => {
    console.error(error);
    sendError(
      req,
      res,
      error.status || 500,
      error.message || "Internal server error",
      error.status ? "request_failed" : "internal_error",
    );
  });
});

server.listen(PORT, HOST, () => {
  console.log(`Token Use backend listening on ${HOST}:${PORT}`);
});
