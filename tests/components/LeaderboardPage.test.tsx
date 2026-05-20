import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { LeaderboardPage } from "@/pages/LeaderboardPage";

const leaderboardState = vi.hoisted(() => ({
  useLeaderboardProfile: vi.fn(),
  useLeaderboardEntries: vi.fn(),
  useGithubLoginStatus: vi.fn(),
  useStartGithubLogin: vi.fn(),
  useLeaderboardOptIn: vi.fn(),
  useLeaderboardSignOut: vi.fn(),
}));

vi.mock("@/lib/query/leaderboard", () => ({
  useLeaderboardProfile: (...args: unknown[]) =>
    leaderboardState.useLeaderboardProfile(...args),
  useLeaderboardEntries: (...args: unknown[]) =>
    leaderboardState.useLeaderboardEntries(...args),
  useGithubLoginStatus: (...args: unknown[]) =>
    leaderboardState.useGithubLoginStatus(...args),
  leaderboardKeys: { all: ["leaderboard"] },
  useStartGithubLogin: (...args: unknown[]) =>
    leaderboardState.useStartGithubLogin(...args),
  useLeaderboardOptIn: (...args: unknown[]) =>
    leaderboardState.useLeaderboardOptIn(...args),
  useLeaderboardSignOut: (...args: unknown[]) =>
    leaderboardState.useLeaderboardSignOut(...args),
}));

describe("LeaderboardPage", () => {
  it("requires GitHub login before leaderboard rows are visible", async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });
    const mutateAsync = vi.fn().mockResolvedValue({
      sessionId: "mock-github-session",
      userCode: "ABCD-1234",
      verificationUri: "https://github.com/login/device",
      expiresAt: Math.floor(Date.now() / 1000) + 900,
      intervalSeconds: 5,
    });
    leaderboardState.useLeaderboardProfile.mockReturnValue({
      data: null,
      isLoading: false,
    });
    leaderboardState.useLeaderboardEntries.mockReturnValue({
      data: { entries: [] },
      isLoading: false,
    });
    leaderboardState.useGithubLoginStatus.mockReturnValue({
      data: null,
    });
    leaderboardState.useStartGithubLogin.mockReturnValue({ mutateAsync });
    leaderboardState.useLeaderboardOptIn.mockReturnValue({
      mutate: vi.fn(),
      isPending: false,
    });
    leaderboardState.useLeaderboardSignOut.mockReturnValue({
      mutate: vi.fn(),
      isPending: false,
    });

    render(
      <QueryClientProvider client={queryClient}>
        <LeaderboardPage />
      </QueryClientProvider>,
    );

    expect(screen.getByRole("heading", { name: "排行" })).toBeInTheDocument();
    expect(screen.getByText("使用 GitHub 登录后查看排行")).toBeInTheDocument();
    expect(screen.queryByRole("table")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /GitHub 登录/ }));
    await waitFor(() => expect(mutateAsync).toHaveBeenCalledTimes(1));
    expect(await screen.findByText("ABCD-1234")).toBeInTheDocument();
  });

  it("can manually refresh leaderboard entries", async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });
    const refetch = vi.fn();
    leaderboardState.useLeaderboardProfile.mockReturnValue({
      data: {
        userId: "u1",
        githubLogin: "alice",
        displayName: "Alice",
        optedIn: true,
      },
      isLoading: false,
    });
    leaderboardState.useLeaderboardEntries.mockReturnValue({
      data: {
        entries: [
          {
            rank: 1,
            userId: "u1",
            githubLogin: "alice",
            displayName: "Alice",
            totalTokens: 2100,
            inputTokens: 1200,
            outputTokens: 600,
            cacheReadTokens: 200,
            cacheCreationTokens: 100,
            requestCount: 2,
            totalCostUsd: "0.030000",
            isCurrentUser: true,
          },
        ],
      },
      isLoading: false,
      isFetching: false,
      refetch,
    });
    leaderboardState.useGithubLoginStatus.mockReturnValue({
      data: null,
    });
    leaderboardState.useStartGithubLogin.mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    });
    leaderboardState.useLeaderboardOptIn.mockReturnValue({
      mutate: vi.fn(),
      isPending: false,
    });
    leaderboardState.useLeaderboardSignOut.mockReturnValue({
      mutate: vi.fn(),
      isPending: false,
    });

    render(
      <QueryClientProvider client={queryClient}>
        <LeaderboardPage />
      </QueryClientProvider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "刷新榜单" }));

    expect(refetch).toHaveBeenCalledTimes(1);
  });
});
