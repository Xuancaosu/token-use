import { cn } from "@/lib/utils";
import type { UsageRangePreset, UsageRangeSelection } from "@/types/usage";

export interface UsageRangeOption {
  preset: Extract<UsageRangePreset, "today" | "7d" | "30d">;
  label: string;
}

const DEFAULT_OPTIONS: UsageRangeOption[] = [
  { preset: "today", label: "今日" },
  { preset: "7d", label: "7 天" },
  { preset: "30d", label: "30 天" },
];

interface UsageRangeSwitchProps {
  value: UsageRangeSelection;
  onChange: (value: UsageRangeSelection) => void;
  options?: UsageRangeOption[];
  className?: string;
}

export function UsageRangeSwitch({
  value,
  onChange,
  options = DEFAULT_OPTIONS,
  className,
}: UsageRangeSwitchProps) {
  return (
    <div
      className={cn(
        "inline-grid grid-flow-col gap-0.5 rounded-xl border border-black/10 bg-muted/55 p-0.5 shadow-inner dark:border-white/10",
        className,
      )}
      aria-label="统计周期"
    >
      {options.map((option) => {
        const isActive = value.preset === option.preset;
        return (
          <button
            key={option.preset}
            type="button"
            onClick={() => onChange({ preset: option.preset })}
            className={cn(
              "h-7 rounded-[10px] px-3 text-xs font-medium transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              isActive
                ? "bg-background text-foreground shadow-sm ring-1 ring-black/5 dark:ring-white/10"
                : "text-muted-foreground hover:bg-background/45 hover:text-foreground",
            )}
            aria-pressed={isActive}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}

export const TOKEN_USE_USAGE_PRESETS = DEFAULT_OPTIONS.map(
  (option) => option.preset,
);
