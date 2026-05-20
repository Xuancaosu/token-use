import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TokenMonitorShell } from "@/components/shell/TokenMonitorShell";

vi.mock("@/pages/UsagePage", () => ({
  UsagePage: () => <div data-testid="usage-page">usage page</div>,
}));

vi.mock("@/pages/LeaderboardPage", () => ({
  LeaderboardPage: () => (
    <div data-testid="leaderboard-page">leaderboard page</div>
  ),
}));

describe("TokenMonitorShell", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("renders a usage-first shell and separates leaderboard navigation", () => {
    render(<TokenMonitorShell />);

    expect(
      screen.getByRole("navigation", { name: "主导航" }),
    ).toBeInTheDocument();
    expect(screen.queryByText("Token Use")).not.toBeInTheDocument();
    expect(screen.getByTestId("usage-page")).toBeInTheDocument();
    expect(screen.queryByTestId("leaderboard-page")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /排行/ }));

    expect(screen.getByTestId("leaderboard-page")).toBeInTheDocument();
    expect(screen.queryByTestId("usage-page")).not.toBeInTheDocument();
  });
});
