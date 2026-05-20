import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Activity, BarChart3, Clock3, ListFilter } from "lucide-react";
import { UsageHero } from "./UsageHero";
import { UsageTrendChart } from "./UsageTrendChart";
import { RequestLogTable } from "./RequestLogTable";
import { ProviderStatsTable } from "./ProviderStatsTable";
import { ModelStatsTable } from "./ModelStatsTable";
import type {
  AppTypeFilter,
  UsageRangePreset,
  UsageRangeSelection,
} from "@/types/usage";
import { getUsageRangePresetLabel, resolveUsageRange } from "@/lib/usageRange";
import { getLocaleFromLanguage } from "./format";

interface UsageDashboardProps {
  range?: UsageRangeSelection;
  onRangeChange?: (range: UsageRangeSelection) => void;
  refreshIntervalMs?: number;
  allowedPresets?: UsageRangePreset[];
  showAppFilter?: boolean;
}

const DEFAULT_RANGE: UsageRangeSelection = { preset: "today" };
const DEFAULT_ALLOWED_PRESETS: UsageRangePreset[] = ["today", "7d", "30d"];

export function UsageDashboard({
  range,
  onRangeChange,
  refreshIntervalMs = 30000,
  allowedPresets = DEFAULT_ALLOWED_PRESETS,
  showAppFilter = false,
}: UsageDashboardProps) {
  const { t, i18n } = useTranslation();
  const [internalRange, setInternalRange] =
    useState<UsageRangeSelection>(DEFAULT_RANGE);
  const activeRange = range ?? internalRange;
  const setRange = onRangeChange ?? setInternalRange;
  const appType: AppTypeFilter = "all";

  useEffect(() => {
    if (!allowedPresets.includes(activeRange.preset)) {
      setRange({ preset: allowedPresets[0] ?? "today" });
    }
  }, [activeRange.preset, allowedPresets, setRange]);

  const language = i18n.resolvedLanguage || i18n.language || "en";
  const locale = getLocaleFromLanguage(language);
  const resolvedRange = useMemo(
    () => resolveUsageRange(activeRange),
    [activeRange],
  );
  const rangeLabel = useMemo(() => {
    if (activeRange.preset !== "custom") {
      return getUsageRangePresetLabel(activeRange.preset, t);
    }

    return `${new Date(resolvedRange.startDate * 1000).toLocaleString(locale)} - ${new Date(
      resolvedRange.endDate * 1000,
    ).toLocaleString(locale)}`;
  }, [
    activeRange.preset,
    locale,
    resolvedRange.endDate,
    resolvedRange.startDate,
    t,
  ]);

  const sectionClass =
    "rounded-2xl border border-black/10 bg-background/80 p-4 shadow-[0_1px_2px_rgba(0,0,0,0.04)] dark:border-white/10 dark:bg-background/70";

  return (
    <div className="space-y-4" data-app-filter-visible={showAppFilter}>
      <UsageHero
        range={activeRange}
        appType={undefined}
        refreshIntervalMs={refreshIntervalMs}
      />

      <div className={sectionClass}>
        <div className="mb-3 flex items-center justify-between gap-3">
          <div className="flex items-center gap-2">
            <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-blue-500/10 text-blue-600 dark:text-blue-400">
              <Clock3 className="h-3.5 w-3.5" />
            </span>
            <div>
              <h2 className="text-sm font-semibold">趋势</h2>
              <p className="text-xs text-muted-foreground">{rangeLabel}</p>
            </div>
          </div>
        </div>
        <UsageTrendChart
          range={activeRange}
          rangeLabel={rangeLabel}
          appType={appType}
          refreshIntervalMs={refreshIntervalMs}
        />
      </div>

      <div className="grid gap-4 xl:grid-cols-2">
        <section className={sectionClass} aria-labelledby="model-stats-title">
          <div className="mb-3 flex items-center gap-2">
            <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-600 dark:text-emerald-400">
              <BarChart3 className="h-3.5 w-3.5" />
            </span>
            <div>
              <h2 id="model-stats-title" className="text-sm font-semibold">
                模型消耗
              </h2>
              <p className="text-xs text-muted-foreground">
                按模型聚合 token 与成本
              </p>
            </div>
          </div>
          <ModelStatsTable
            range={activeRange}
            appType={appType}
            refreshIntervalMs={refreshIntervalMs}
          />
        </section>

        <section
          className={sectionClass}
          aria-labelledby="provider-stats-title"
        >
          <div className="mb-3 flex items-center gap-2">
            <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-violet-500/10 text-violet-600 dark:text-violet-400">
              <Activity className="h-3.5 w-3.5" />
            </span>
            <div>
              <h2 id="provider-stats-title" className="text-sm font-semibold">
                来源统计
              </h2>
              <p className="text-xs text-muted-foreground">
                按采集来源保留原始归因
              </p>
            </div>
          </div>
          <ProviderStatsTable
            range={activeRange}
            appType={appType}
            refreshIntervalMs={refreshIntervalMs}
          />
        </section>
      </div>

      <section className={sectionClass} aria-labelledby="request-log-title">
        <div className="mb-3 flex items-center gap-2">
          <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-orange-500/10 text-orange-600 dark:text-orange-400">
            <ListFilter className="h-3.5 w-3.5" />
          </span>
          <div>
            <h2 id="request-log-title" className="text-sm font-semibold">
              明细记录
            </h2>
            <p className="text-xs text-muted-foreground">
              保留请求级 token、缓存与成本明细
            </p>
          </div>
        </div>
        <RequestLogTable
          range={activeRange}
          rangeLabel={rangeLabel}
          appType={appType}
          refreshIntervalMs={refreshIntervalMs}
          onRangeChange={setRange}
          showAppTypeFilter={showAppFilter}
          showRangePicker={showAppFilter}
        />
      </section>
    </div>
  );
}
