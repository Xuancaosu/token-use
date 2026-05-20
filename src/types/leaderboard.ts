export type LeaderboardRange = "today" | "7d" | "30d";

export interface LeaderboardProfile {
  userId: string;
  githubLogin: string;
  displayName: string;
  avatarUrl?: string;
  optedIn: boolean;
  joinedAt?: number;
}

export interface LeaderboardEntry {
  rank: number;
  userId: string;
  githubLogin: string;
  displayName: string;
  avatarUrl?: string;
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheCreationTokens: number;
  requestCount: number;
  totalCostUsd: string;
  isCurrentUser?: boolean;
}

export interface LeaderboardEntriesResult {
  range: LeaderboardRange;
  updatedAt?: number;
  entries: LeaderboardEntry[];
}

export interface GithubDeviceLoginStart {
  sessionId: string;
  userCode: string;
  verificationUri: string;
  expiresAt: number;
  intervalSeconds: number;
}

export type GithubDeviceLoginStatusValue =
  | "pending"
  | "authorized"
  | "completed"
  | "expired"
  | "denied";

export interface GithubDeviceLoginStatus {
  status: GithubDeviceLoginStatusValue;
  profile?: LeaderboardProfile | null;
  retryAfterSeconds?: number | null;
  message?: string | null;
}
