import { invoke } from "@tauri-apps/api/core";
import type {
  GithubDeviceLoginStart,
  GithubDeviceLoginStatus,
  LeaderboardEntriesResult,
  LeaderboardProfile,
  LeaderboardRange,
} from "@/types/leaderboard";

export const leaderboardApi = {
  getProfile: async (): Promise<LeaderboardProfile | null> => {
    return invoke("get_leaderboard_profile");
  },

  startGithubLogin: async (): Promise<GithubDeviceLoginStart> => {
    return invoke("start_github_leaderboard_login");
  },

  pollGithubLogin: async (
    sessionId: string,
  ): Promise<GithubDeviceLoginStatus> => {
    return invoke("poll_github_leaderboard_login", { sessionId });
  },

  signOut: async (): Promise<void> => {
    return invoke("sign_out_leaderboard");
  },

  setOptIn: async (optedIn: boolean): Promise<LeaderboardProfile> => {
    return invoke("set_leaderboard_opt_in", { optedIn });
  },

  getEntries: async (
    range: LeaderboardRange,
  ): Promise<LeaderboardEntriesResult> => {
    return invoke("get_leaderboard_entries", { range });
  },
};
