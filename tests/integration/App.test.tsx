import { Suspense, type ComponentType } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";

const renderApp = (AppComponent: ComponentType) => {
  const client = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });
  return render(
    <QueryClientProvider client={client}>
      <Suspense fallback={<div data-testid="loading">loading</div>}>
        <AppComponent />
      </Suspense>
    </QueryClientProvider>,
  );
};

describe("App integration", () => {
  it("starts as the usage-only Token Use shell", async () => {
    const { default: App } = await import("@/App");
    renderApp(App);

    expect(
      screen.getByRole("navigation", { name: "主导航" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /统计/ })).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "消耗统计" }),
    ).toBeInTheDocument();
    expect(screen.queryByText("Provider")).not.toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText("明细记录")).toBeInTheDocument();
    });
  });

  it("keeps leaderboard on a separate GitHub-gated view", async () => {
    const { default: App } = await import("@/App");
    renderApp(App);

    fireEvent.click(screen.getByRole("button", { name: /排行/ }));

    expect(screen.getByRole("heading", { name: "排行" })).toBeInTheDocument();
    await waitFor(() => {
      expect(
        screen.getByText("使用 GitHub 登录后查看排行"),
      ).toBeInTheDocument();
    });
    expect(
      screen.queryByRole("heading", { name: "消耗统计" }),
    ).not.toBeInTheDocument();
  });
});
