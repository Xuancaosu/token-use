import type { LucideIcon } from "lucide-react";
import { BarChart3, Trophy } from "lucide-react";
import { cn } from "@/lib/utils";

export type TokenMonitorView = "usage" | "leaderboard";

interface NavigationItem {
  id: TokenMonitorView;
  label: string;
  icon: LucideIcon;
}

const NAV_ITEMS: NavigationItem[] = [
  {
    id: "usage",
    label: "统计",
    icon: BarChart3,
  },
  {
    id: "leaderboard",
    label: "排行",
    icon: Trophy,
  },
];

interface NavigationRailProps {
  activeView: TokenMonitorView;
  onChange: (view: TokenMonitorView) => void;
}

export function NavigationRail({ activeView, onChange }: NavigationRailProps) {
  return (
    <nav
      className="flex h-full flex-col items-center gap-2 px-2 py-4"
      aria-label="主导航"
    >
      {NAV_ITEMS.map((item) => {
        const Icon = item.icon;
        const active = activeView === item.id;
        return (
          <button
            key={item.id}
            type="button"
            onClick={() => onChange(item.id)}
            className={cn(
              "group flex h-[54px] w-[54px] flex-col items-center justify-center gap-1 rounded-xl text-center transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
              active
                ? "bg-foreground/10 text-foreground shadow-inner dark:bg-white/10"
                : "text-muted-foreground hover:bg-foreground/5 hover:text-foreground",
            )}
            aria-current={active ? "page" : undefined}
            title={item.label}
          >
            <span className="flex h-5 w-5 items-center justify-center">
              <Icon className="h-[18px] w-[18px]" />
            </span>
            <span className="text-[10px] font-medium leading-none">
              {item.label}
            </span>
          </button>
        );
      })}
    </nav>
  );
}
