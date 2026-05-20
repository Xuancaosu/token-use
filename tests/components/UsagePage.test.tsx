import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { UsagePage } from "@/pages/UsagePage";

vi.mock("@/components/usage/UsageDashboard", () => ({
  UsageDashboard: ({ allowedPresets, showAppFilter }: any) => (
    <div data-testid="usage-dashboard">
      <span data-testid="allowed-presets">{allowedPresets.join(",")}</span>
      <span data-testid="show-app-filter">{String(showAppFilter)}</span>
    </div>
  ),
}));

describe("UsagePage", () => {
  it("shows usage statistics separately from leaderboard and removes software filters", () => {
    render(<UsagePage />);

    expect(screen.getByRole("heading", { name: "消耗统计" })).toBeInTheDocument();
    expect(screen.getByText("今日")).toBeInTheDocument();
    expect(screen.getByText("7 天")).toBeInTheDocument();
    expect(screen.getByText("30 天")).toBeInTheDocument();
    expect(screen.getByTestId("allowed-presets")).toHaveTextContent(
      "today,7d,30d",
    );
    expect(screen.getByTestId("show-app-filter")).toHaveTextContent("false");
    expect(screen.queryByText(/Claude|Codex|Gemini/)).not.toBeInTheDocument();
    expect(screen.queryByText(/榜单|排行/)).not.toBeInTheDocument();
  });
});
