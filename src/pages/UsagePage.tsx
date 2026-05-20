import { useState } from "react";
import { RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { UsageDashboard } from "@/components/usage/UsageDashboard";
import {
  TOKEN_USE_USAGE_PRESETS,
  UsageRangeSwitch,
} from "@/components/usage/UsageRangeSwitch";
import type { UsageRangeSelection } from "@/types/usage";

const REFRESH_OPTIONS_MS = [0, 10000, 30000, 60000] as const;

export function UsagePage() {
  const [range, setRange] = useState<UsageRangeSelection>({
    preset: "today",
  });
  const [refreshIntervalMs, setRefreshIntervalMs] = useState(30000);

  const rotateRefreshInterval = () => {
    const currentIndex = REFRESH_OPTIONS_MS.indexOf(
      refreshIntervalMs as (typeof REFRESH_OPTIONS_MS)[number],
    );
    const nextIndex =
      currentIndex >= 0 ? (currentIndex + 1) % REFRESH_OPTIONS_MS.length : 2;
    setRefreshIntervalMs(REFRESH_OPTIONS_MS[nextIndex]);
  };

  return (
    <section className="space-y-4" aria-labelledby="usage-page-title">
      <div className="flex flex-col gap-3 border-b border-black/10 pb-4 dark:border-white/10 lg:flex-row lg:items-center lg:justify-between">
        <div className="max-w-2xl">
          <h1
            id="usage-page-title"
            className="text-[22px] font-semibold leading-7 tracking-normal"
          >
            消耗统计
          </h1>
          <p className="mt-1 text-[13px] leading-5 text-muted-foreground">
            汇总本机 token 消耗记录，不再暴露供应商切换和软件筛选。
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <UsageRangeSwitch value={range} onChange={setRange} />
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-8 gap-1.5 rounded-xl border-black/10 bg-background/70 px-2.5 text-xs shadow-sm dark:border-white/10"
            onClick={rotateRefreshInterval}
            title="刷新频率"
          >
            <RefreshCw className="h-4 w-4" />
            {refreshIntervalMs > 0 ? `${refreshIntervalMs / 1000}s` : "手动"}
          </Button>
        </div>
      </div>

      <UsageDashboard
        range={range}
        onRangeChange={setRange}
        refreshIntervalMs={refreshIntervalMs}
        allowedPresets={TOKEN_USE_USAGE_PRESETS}
        showAppFilter={false}
      />
    </section>
  );
}
