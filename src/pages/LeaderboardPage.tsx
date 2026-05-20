import { useEffect, useState } from "react";
import {
  Github,
  Loader2,
  LogOut,
  ShieldCheck,
  Timer,
  Trophy,
} from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { UsageRangeSwitch } from "@/components/usage/UsageRangeSwitch";
import {
  leaderboardKeys,
  useGithubLoginStatus,
  useLeaderboardEntries,
  useLeaderboardOptIn,
  useLeaderboardProfile,
  useLeaderboardSignOut,
  useStartGithubLogin,
} from "@/lib/query/leaderboard";
import type {
  GithubDeviceLoginStart,
  LeaderboardRange,
} from "@/types/leaderboard";
import type { UsageRangeSelection } from "@/types/usage";

const RANGE_TO_LEADERBOARD: Record<
  UsageRangeSelection["preset"],
  LeaderboardRange | null
> = {
  today: "today",
  "1d": null,
  "7d": "7d",
  "14d": null,
  "30d": "30d",
  custom: null,
};

function formatTokens(value: number) {
  return new Intl.NumberFormat("zh-CN").format(value);
}

function formatCost(value: string) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return "$0.0000";
  return `$${parsed.toFixed(4)}`;
}

export function LeaderboardPage() {
  const [range, setRange] = useState<UsageRangeSelection>({ preset: "today" });
  const [loginSession, setLoginSession] =
    useState<GithubDeviceLoginStart | null>(null);
  const [nowSeconds, setNowSeconds] = useState(() =>
    Math.floor(Date.now() / 1000),
  );
  const leaderboardRange = RANGE_TO_LEADERBOARD[range.preset] ?? "today";
  const queryClient = useQueryClient();
  const { data: profile, isLoading: isProfileLoading } =
    useLeaderboardProfile();
  const startLogin = useStartGithubLogin();
  const optIn = useLeaderboardOptIn();
  const signOut = useLeaderboardSignOut();
  const canLoadEntries = Boolean(profile?.optedIn);
  const { data: leaderboard, isLoading: isEntriesLoading } =
    useLeaderboardEntries(leaderboardRange, canLoadEntries);
  const pollIntervalMs = Math.max(5, loginSession?.intervalSeconds ?? 5) * 1000;
  const loginStatus = useGithubLoginStatus(
    loginSession?.sessionId,
    Boolean(loginSession),
    pollIntervalMs,
  );

  const entries = leaderboard?.entries ?? [];
  const loginExpiresIn = loginSession
    ? Math.max(0, loginSession.expiresAt - nowSeconds)
    : null;

  useEffect(() => {
    if (!loginSession) return;

    setNowSeconds(Math.floor(Date.now() / 1000));
    const timer = window.setInterval(() => {
      setNowSeconds(Math.floor(Date.now() / 1000));
    }, 1000);

    return () => window.clearInterval(timer);
  }, [loginSession]);

  useEffect(() => {
    const status = loginStatus.data?.status;
    if (status === "authorized" || status === "completed") {
      setLoginSession(null);
      void queryClient.invalidateQueries({ queryKey: leaderboardKeys.all });
    }

    if (status === "expired" || status === "denied") {
      void queryClient.invalidateQueries({ queryKey: leaderboardKeys.all });
    }
  }, [loginStatus.data?.status, queryClient]);

  const beginGithubLogin = async () => {
    const session = await startLogin.mutateAsync();
    setLoginSession(session);
  };

  return (
    <section className="space-y-4" aria-labelledby="leaderboard-page-title">
      <div className="flex flex-col gap-3 border-b border-black/10 pb-4 dark:border-white/10 lg:flex-row lg:items-center lg:justify-between">
        <div className="max-w-2xl">
          <h1
            id="leaderboard-page-title"
            className="text-[22px] font-semibold leading-7 tracking-normal"
          >
            排行
          </h1>
          <p className="mt-1 text-[13px] leading-5 text-muted-foreground">
            排行只按 token
            消耗量计算。登录后可查看榜单，是否加入排行由用户自行开启。
          </p>
        </div>

        <UsageRangeSwitch value={range} onChange={setRange} />
      </div>

      {isProfileLoading ? (
        <div className="flex min-h-[220px] items-center justify-center rounded-2xl border border-black/10 bg-background/80 shadow-[0_1px_2px_rgba(0,0,0,0.04)] dark:border-white/10">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      ) : !profile ? (
        <div className="rounded-2xl border border-black/10 bg-background/80 p-8 shadow-[0_1px_2px_rgba(0,0,0,0.04)] dark:border-white/10">
          <div className="mx-auto flex max-w-md flex-col items-center text-center">
            <div className="mb-5 flex h-12 w-12 items-center justify-center rounded-2xl bg-foreground text-background">
              <Github className="h-6 w-6" />
            </div>
            <h2 className="text-xl font-semibold">
              使用 GitHub 登录后查看排行
            </h2>
            <p className="mt-3 text-sm leading-6 text-muted-foreground">
              登录仅用于确认排行身份。未登录时不会展示榜单，也不会上传本地消耗统计。
            </p>
            <Button
              type="button"
              className="mt-6 h-9 gap-2 rounded-xl px-4"
              onClick={() => {
                void beginGithubLogin();
              }}
              disabled={startLogin.isPending}
            >
              {startLogin.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Github className="h-4 w-4" />
              )}
              GitHub 登录
            </Button>
            {loginSession && (
              <div className="mt-6 w-full rounded-2xl border border-black/10 bg-muted/35 p-4 text-left dark:border-white/10">
                <div className="flex items-center justify-between gap-3">
                  <div className="text-sm font-medium">GitHub 验证码</div>
                  <div className="flex items-center gap-1 text-xs text-muted-foreground">
                    <Timer className="h-3.5 w-3.5" />
                    {loginExpiresIn ?? 0}s
                  </div>
                </div>
                <div className="mt-3 rounded-xl bg-background px-4 py-3 text-center font-mono text-2xl font-semibold tracking-[0.18em] shadow-inner">
                  {loginSession.userCode}
                </div>
                <p className="mt-3 text-xs leading-5 text-muted-foreground">
                  GitHub 授权页面已打开。如果没有自动打开，请访问{" "}
                  {loginSession.verificationUri} 并输入验证码。
                </p>
                <div className="mt-3 flex items-center gap-2 text-xs text-muted-foreground">
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  {loginStatus.data?.message ?? "等待 GitHub 授权确认..."}
                </div>
              </div>
            )}
            {!loginSession && startLogin.error && (
              <p className="mt-4 text-sm text-destructive">
                {String(startLogin.error)}
              </p>
            )}
            {loginSession &&
              (loginStatus.data?.status === "expired" ||
                loginStatus.data?.status === "denied") && (
                <p className="mt-4 text-sm text-destructive">
                  {loginStatus.data.message ?? "GitHub 登录未完成，请重试。"}
                </p>
              )}
          </div>
        </div>
      ) : (
        <div className="space-y-4">
          <div className="flex flex-col gap-4 rounded-2xl border border-black/10 bg-background/80 p-4 shadow-[0_1px_2px_rgba(0,0,0,0.04)] dark:border-white/10 md:flex-row md:items-center md:justify-between">
            <div className="flex items-center gap-3">
              {profile.avatarUrl ? (
                <img
                  src={profile.avatarUrl}
                  alt=""
                  className="h-10 w-10 rounded-xl border border-black/10 object-cover dark:border-white/10"
                />
              ) : (
                <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-muted">
                  <ShieldCheck className="h-5 w-5 text-muted-foreground" />
                </div>
              )}
              <div>
                <div className="font-medium">{profile.displayName}</div>
                <div className="text-sm text-muted-foreground">
                  @{profile.githubLogin}
                </div>
              </div>
            </div>

            <div className="flex flex-wrap items-center gap-3">
              <label className="flex items-center gap-2 rounded-xl border border-black/10 bg-muted/35 px-3 py-2 text-sm dark:border-white/10">
                <Switch
                  checked={profile.optedIn}
                  onCheckedChange={(checked) => optIn.mutate(checked)}
                  disabled={optIn.isPending}
                />
                加入排行
              </label>
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="gap-2 rounded-xl border-black/10 bg-background/70 dark:border-white/10"
                onClick={() => signOut.mutate()}
                disabled={signOut.isPending}
              >
                <LogOut className="h-4 w-4" />
                退出
              </Button>
            </div>
          </div>

          {!profile.optedIn ? (
            <div className="rounded-2xl border border-black/10 bg-background/80 p-8 text-center shadow-[0_1px_2px_rgba(0,0,0,0.04)] dark:border-white/10">
              <Trophy className="mx-auto mb-4 h-8 w-8 text-muted-foreground" />
              <h2 className="text-lg font-semibold">
                开启后加入 token 消耗排行
              </h2>
              <p className="mt-2 text-sm text-muted-foreground">
                关闭时仅保留本地统计，不进入当天、7 天、30 天排行榜。
              </p>
            </div>
          ) : (
            <div className="overflow-hidden rounded-2xl border border-black/10 bg-background/80 shadow-[0_1px_2px_rgba(0,0,0,0.04)] dark:border-white/10">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-20 text-center">名次</TableHead>
                    <TableHead>用户</TableHead>
                    <TableHead className="text-right">Token</TableHead>
                    <TableHead className="text-right">请求</TableHead>
                    <TableHead className="text-right">成本</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {isEntriesLoading ? (
                    <TableRow>
                      <TableCell colSpan={5} className="h-32 text-center">
                        <Loader2 className="mx-auto h-5 w-5 animate-spin text-muted-foreground" />
                      </TableCell>
                    </TableRow>
                  ) : entries.length === 0 ? (
                    <TableRow>
                      <TableCell
                        colSpan={5}
                        className="h-32 text-center text-muted-foreground"
                      >
                        暂无排行数据
                      </TableCell>
                    </TableRow>
                  ) : (
                    entries.map((entry) => (
                      <TableRow
                        key={`${entry.rank}-${entry.userId}`}
                        className={
                          entry.isCurrentUser ? "bg-primary/5" : undefined
                        }
                      >
                        <TableCell className="text-center font-semibold">
                          {entry.rank}
                        </TableCell>
                        <TableCell>
                          <div className="flex items-center gap-3">
                            {entry.avatarUrl ? (
                              <img
                                src={entry.avatarUrl}
                                alt=""
                                className="h-8 w-8 rounded-md object-cover"
                              />
                            ) : (
                              <div className="h-8 w-8 rounded-md bg-muted" />
                            )}
                            <div>
                              <div className="font-medium">
                                {entry.displayName}
                              </div>
                              <div className="text-xs text-muted-foreground">
                                @{entry.githubLogin}
                              </div>
                            </div>
                          </div>
                        </TableCell>
                        <TableCell className="text-right font-medium">
                          {formatTokens(entry.totalTokens)}
                        </TableCell>
                        <TableCell className="text-right">
                          {formatTokens(entry.requestCount)}
                        </TableCell>
                        <TableCell className="text-right">
                          {formatCost(entry.totalCostUsd)}
                        </TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
            </div>
          )}
        </div>
      )}
    </section>
  );
}
