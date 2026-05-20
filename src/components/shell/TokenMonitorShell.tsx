import { useEffect, useState } from "react";
import { LeaderboardPage } from "@/pages/LeaderboardPage";
import { UsagePage } from "@/pages/UsagePage";
import {
  NavigationRail,
  type TokenMonitorView,
} from "@/components/shell/NavigationRail";

const VIEW_STORAGE_KEY = "token-use-current-view";
const VIEWS: TokenMonitorView[] = ["usage", "leaderboard"];

function getInitialView(): TokenMonitorView {
  if (typeof window === "undefined") return "usage";
  const saved = window.localStorage.getItem(VIEW_STORAGE_KEY);
  return VIEWS.includes(saved as TokenMonitorView)
    ? (saved as TokenMonitorView)
    : "usage";
}

export function TokenMonitorShell() {
  const [activeView, setActiveView] =
    useState<TokenMonitorView>(getInitialView);

  useEffect(() => {
    window.localStorage.setItem(VIEW_STORAGE_KEY, activeView);
  }, [activeView]);

  return (
    <div className="h-screen overflow-hidden bg-[#f5f5f7] text-foreground dark:bg-[#1f1f21]">
      <header
        className="fixed inset-x-0 top-0 z-30 h-11 border-b border-black/10 bg-background/75 backdrop-blur-xl dark:border-white/10"
        data-tauri-drag-region
      >
        <div className="h-full" />
      </header>

      <div className="grid h-full grid-cols-[72px_minmax(0,1fr)] pt-11">
        <aside className="min-h-0 border-r border-black/10 bg-background/70 backdrop-blur-xl dark:border-white/10">
          <NavigationRail activeView={activeView} onChange={setActiveView} />
        </aside>

        <main className="min-w-0 overflow-y-auto">
          <div className="max-w-[1180px] px-6 py-5 lg:px-8">
            {activeView === "usage" ? <UsagePage /> : <LeaderboardPage />}
          </div>
        </main>
      </div>
    </div>
  );
}
