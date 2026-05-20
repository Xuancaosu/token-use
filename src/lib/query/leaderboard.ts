import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { leaderboardApi } from "@/lib/api/leaderboard";
import type { LeaderboardRange } from "@/types/leaderboard";

export const leaderboardKeys = {
  all: ["leaderboard"] as const,
  profile: () => [...leaderboardKeys.all, "profile"] as const,
  entries: (range: LeaderboardRange) =>
    [...leaderboardKeys.all, "entries", range] as const,
  githubLogin: (sessionId: string) =>
    [...leaderboardKeys.all, "github-login", sessionId] as const,
};

export function useLeaderboardProfile() {
  return useQuery({
    queryKey: leaderboardKeys.profile(),
    queryFn: leaderboardApi.getProfile,
  });
}

export function useLeaderboardEntries(range: LeaderboardRange, enabled = true) {
  return useQuery({
    queryKey: leaderboardKeys.entries(range),
    queryFn: () => leaderboardApi.getEntries(range),
    enabled,
    refetchInterval: enabled ? 60000 : false,
  });
}

export function useGithubLoginStatus(
  sessionId: string | undefined,
  enabled: boolean,
  intervalMs: number,
) {
  return useQuery({
    queryKey: leaderboardKeys.githubLogin(sessionId ?? ""),
    queryFn: () => leaderboardApi.pollGithubLogin(sessionId ?? ""),
    enabled: enabled && Boolean(sessionId),
    refetchInterval: (query) => {
      const status = query.state.data?.status;
      return status === "authorized" ||
        status === "completed" ||
        status === "expired" ||
        status === "denied"
        ? false
        : intervalMs;
    },
  });
}

export function useStartGithubLogin() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: leaderboardApi.startGithubLogin,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: leaderboardKeys.all });
    },
  });
}

export function useLeaderboardOptIn() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: leaderboardApi.setOptIn,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: leaderboardKeys.all });
    },
  });
}

export function useLeaderboardSignOut() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: leaderboardApi.signOut,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: leaderboardKeys.all });
    },
  });
}
