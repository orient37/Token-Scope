import { UsageDistribution } from "../components/UsageDistribution";
import { ProjectBubbles } from "../components/ProjectBubbles";
import { usageFilter } from "../lib/filters";
import { ExportUsage } from "../components/ExportUsage";
import { AccountAvatar } from "../components/AccountAvatar";
import { LiveRate } from "../components/LiveRate";
import { useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  ArrowUpRight,
  ArrowDownLeft,
  BarChart3,
  Clock3,
  Database,
  Folder,
  Layers3,
  LayoutDashboard,
  RefreshCw,
  Settings2,
  ShieldCheck,
  Sparkles,
  X,
  Sun,
  Moon,
  Bell,
} from "lucide-react";
import type {
  Dashboard,
  Grain,
  Filter,
  Health,
  Quota,
  Session,
  Window,
} from "../lib/types";
import { compact, exact, localTime, resetLabel, stale } from "../lib/format";
import { collectorLabel, intervalLabel } from "../lib/monitor";
import { Alerts } from "../components/Alerts";
import { Cost } from "../components/Cost";
import type { Currency } from "../lib/money";
import { ActivityHeatmap } from "../components/ActivityHeatmap";
import { ProviderMark } from "../components/ProviderMark";
import { Trend } from "../components/Trend";
import { Button } from "../components/ui/button";
import { useUI, type Page } from "./store";
const nav: { page: Page; label: string; icon: typeof Activity }[] = [
  { page: "overview", label: "总览", icon: LayoutDashboard },
  { page: "sessions", label: "会话", icon: Layers3 },
  { page: "timeline", label: "时间线", icon: Activity },
  { page: "alerts", label: "预警", icon: Bell },
  { page: "settings", label: "设置与数据源", icon: Settings2 },
];
const titles: Record<Page, string> = {
  overview: "用量总览",
  sessions: "会话与轮次",
  timeline: "消耗时间线",
  alerts: "消耗与额度预警",
  settings: "设置与数据源",
};
function Metric({
  label,
  value,
  note,
  icon: Icon,
}: {
  label: string;
  value: string;
  note: string;
  icon: typeof Activity | typeof ProviderMark;
}) {
  return (
    <div className="metric">
      <div className="metric-label">
        {label}
        <Icon size={17} />
      </div>
      <strong title={exact(value)}>{compact(value)}</strong>
      <small>{note}</small>
    </div>
  );
}
function QuotaWindow({
  w,
  label,
  now,
  isStale,
}: {
  w: Window | null;
  label: string;
  now: number;
  isStale: boolean;
}) {
  if (!w)
    return (
      <div className="quota-window">
        <span>{label}</span>
        <p className="muted">该窗口不可用</p>
      </div>
    );
  const resetPassed = w.resets_at !== null && w.resets_at * 1000 <= now;
  const usable = !isStale && !resetPassed;
  return (
    <div className="quota-window">
      <div className="between">
        <span>
          {w.duration_mins === null
            ? label
            : w.duration_mins >= 1440
              ? w.duration_mins / 1440 + " 天窗口"
              : w.duration_mins / 60 + " 小时窗口"}
        </span>
        <strong className={usable && w.used_percent >= 80 ? "warning" : ""}>
          {usable ? w.remaining_percent.toFixed(0) + "% 剩余" : "待同步"}
        </strong>
      </div>
      <div className="meter">
        <i
          className={w.used_percent >= 80 ? "warn" : ""}
          style={{ width: Math.min(100, w.used_percent) + "%" }}
        />
      </div>
      <div className="between muted">
        <small>快照已使用 {w.used_percent}%</small>
        <small>{resetLabel(w.resets_at, now)}</small>
      </div>
    </div>
  );
}
export function App() {
  const { page, setPage } = useUI();
  const [theme, setTheme] = useState<"light" | "dark">(() => {
    try {
      return localStorage.getItem("tokenscope-theme") === "dark"
        ? "dark"
        : "light";
    } catch {
      return "light";
    }
  });
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem("tokenscope-theme", theme);
    } catch {
      /* storage is optional */
    }
  }, [theme]);
  useEffect(() => {
    const navigate = (e: KeyboardEvent) => {
      if (document.querySelector("dialog[open]")) return;
      if (!e.altKey || e.ctrlKey || e.metaKey) return;
      const item = nav[Number(e.key) - 1];
      if (item) {
        e.preventDefault();
        setPage(item.page);
        setSelected(null);
      }
    };
    window.addEventListener("keydown", navigate);
    return () => window.removeEventListener("keydown", navigate);
  }, [setPage]);
  const [data, setData] = useState<Dashboard | null>(null);
  const [health, setHealth] = useState<Health | null>(null);
  const [quota, setQuota] = useState<Quota | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [tick, setTick] = useState(0);
  const [unreadAlerts, setUnreadAlerts] = useState<number | null>(null);
  const queryRefresh = useRef<(() => void) | null>(null);
  const detailRefresh = useRef<(() => void) | null>(null);
  const [period, setPeriod] = useState("all");
  const [grain, setGrain] = useState<Grain>("day");
  const [project, setProject] = useState("");
  const [model, setModel] = useState("");
  const [catalog, setCatalog] = useState<Dashboard | null>(null);
  const [selected, setSelected] = useState<Session | null>(null);
  const [currency, setCurrency] = useState<Currency>("USD");
  const [fxBusy, setFxBusy] = useState(false);
  const [fxEnabledPending, setFxEnabledPending] = useState<boolean | null>(
    null,
  );
  const toggleFx = async (enabled: boolean) => {
    setFxEnabledPending(enabled);
    try {
      await invoke("set_fx_enabled", { enabled });
      setTick((t) => t + 1);
    } catch {
      setFxEnabledPending(null);
      setError("汇率设置保存失败");
    }
  };
  const fxBusyRef = useRef(false);
  const refreshFx = async () => {
    if (!desktop || fxBusyRef.current) return;
    fxBusyRef.current = true;
    setFxBusy(true);
    try {
      await invoke("refresh_fx");
      setTick((t) => t + 1);
    } catch {
      setError("参考汇率同步失败");
    } finally {
      fxBusyRef.current = false;
      setFxBusy(false);
    }
  };
  const [detail, setDetail] = useState<Dashboard | null>(null);
  const [now, setNow] = useState(Date.now());
  const [busy, setBusy] = useState(false);
  const busyRef = useRef(false);
  const desktop = isTauri();
  const refreshQuota = async () => {
    if (!desktop || busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    try {
      const q = await invoke<Quota>("refresh_quota");
      setQuota(q);
    } catch {
      setQuota({
        status: "unavailable",
        collected_at: null,
        data: null,
        message: "额度服务请求失败",
      });
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  };
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 30000);
    if (!desktop) return () => clearInterval(t);
    let disposed = false;
    const pending = [
      listen("usage-updated", () => setTick((t) => t + 1)),
      listen("alerts-updated", () => setTick((t) => t + 1)),
      listen("ingest-health", () => {
        void invoke<Health>("read_health")
          .then((h) => {
            if (!disposed) setHealth(h);
          })
          .catch(() => setError("采集健康状态读取失败"));
      }),
    ];
    void refreshQuota();
    const q = setInterval(() => void refreshQuota(), 60000);
    return () => {
      disposed = true;
      clearInterval(t);
      clearInterval(q);
      for (const p of pending) void p.then((fn) => fn());
    };
  }, []);
  useEffect(() => {
    let cancelled = false;
    if (!desktop) {
      setLoading(false);
      return;
    }
    setLoading(true);
    let running = false;
    let queued = false;
    const refresh = () => {
      queued = true;
      if (running || cancelled) return;
      running = true;
      void (async () => {
        try {
          while (queued && !cancelled) {
            queued = false;
            const currentFilter = usageFilter(period, project, model, grain);
            try {
              const [d, h] = await Promise.all([
                invoke<Dashboard>("read_dashboard", { filter: currentFilter }),
                invoke<Health>("read_health"),
              ]);
              if (cancelled) return;
              setData(d);
              setFxEnabledPending((p) =>
                p === d.estimate.fx.enabled ? null : p,
              );
              setSelected((prev) =>
                prev
                  ? (d.sessions.find((s) => s.id === prev.id) ?? null)
                  : null,
              );
              setHealth(h);
              setError("");
              if (!project && !model) setCatalog(d);
            } catch {
              if (!cancelled)
                setError("本地数据读取失败，请检查应用数据目录或筛选条件");
            } finally {
              if (!cancelled) setLoading(false);
            }
          }
        } finally {
          running = false;
        }
      })();
    };
    queryRefresh.current = refresh;
    refresh();
    return () => {
      cancelled = true;
      if (queryRefresh.current === refresh) queryRefresh.current = null;
    };
  }, [period, grain, project, model, desktop]);
  useEffect(() => {
    if (tick > 0) {
      queryRefresh.current?.();
      detailRefresh.current?.();
    }
  }, [tick]);
  useEffect(() => {
    setDetail(null);
    if (!desktop || !selected?.id) return;
    let cancelled = false;
    const filter: Filter = { session: selected.id, grain };
    if (period !== "all") {
      const start = new Date();
      if (period === "today") start.setHours(0, 0, 0, 0);
      else start.setTime(start.getTime() - Number(period) * 86400000);
      filter.from = start.toISOString();
    }
    if (project) filter.project = project;
    if (model) filter.model = model;
    let running = false;
    let queued = false;
    const refresh = () => {
      queued = true;
      if (running || cancelled) return;
      running = true;
      void (async () => {
        try {
          while (queued && !cancelled) {
            queued = false;
            const currentFilter = { ...filter };
            if (period !== "all") {
              const start = new Date();
              if (period === "today") start.setHours(0, 0, 0, 0);
              else start.setTime(start.getTime() - Number(period) * 86400000);
              currentFilter.from = start.toISOString();
            }
            try {
              const d = await invoke<Dashboard>("read_dashboard", {
                filter: currentFilter,
              });
              if (!cancelled) setDetail(d);
            } catch {
              if (!cancelled) setError("会话轮次读取失败");
            }
          }
        } finally {
          running = false;
        }
      })();
    };
    detailRefresh.current = refresh;
    refresh();
    return () => {
      cancelled = true;
      if (detailRefresh.current === refresh) detailRefresh.current = null;
    };
  }, [selected?.id, period, grain, project, model, desktop]);
  useEffect(() => {
    if (!desktop) return;
    let disposed = false;
    void invoke<{ unread: number }>("read_alerts")
      .then((d) => {
        if (!disposed) setUnreadAlerts(d.unread);
      })
      .catch(() => {
        if (!disposed) setUnreadAlerts(null);
      });
    return () => {
      disposed = true;
    };
  }, [tick, desktop]);
  const qStale = stale(quota?.collected_at ?? null, now);
  const options = catalog ?? data;
  const sessions = data?.sessions ?? [];
  const totals =
    data?.snapshots || health?.report.source_available
      ? data?.totals
      : undefined;
  const showQuota = () => (
    <section
      className="panel quota-panel"
      id="quota-panel"
      aria-label="套餐额度面板"
    >
      <div className="panel-head">
        <div>
          <h2>Codex 套餐额度</h2>
          <p>服务端快照 · 与本地 Token 统计分开计量</p>
        </div>
        <Button
          variant="outline"
          onClick={() => void refreshQuota()}
          disabled={busy || !desktop}
          aria-busy={busy}
        >
          <RefreshCw size={14} className={busy ? "spin" : ""} />
          {busy ? "同步中" : "同步额度"}
        </Button>
      </div>
      {!quota || quota.status !== "synced" ? (
        <div className="empty">
          <ShieldCheck size={28} />
          <strong>{busy ? "正在读取官方额度接口" : "额度不可用"}</strong>
          <span>{quota?.message ?? "尚未同步"}</span>
        </div>
      ) : (
        <>
          {qStale && <div className="notice">快照已过期，等待重新同步</div>}
          {quota.data?.ordinary_usage_allowed === false && (
            <div className="notice">服务端报告：当前账户不允许普通套餐用量</div>
          )}
          {quota.data?.buckets.length === 0 && (
            <div className="empty">服务端没有返回可用限额池</div>
          )}
          {quota.data?.buckets.map((b) => (
            <div className="quota-pool" key={b.id}>
              <div className="between">
                <h3>{b.name ?? b.id}</h3>
                <span className="tag">{b.plan ?? "套餐未知"}</span>
              </div>
              {b.spend_control_reached === true && (
                <p className="warning">服务端报告已达支出限制</p>
              )}
              <QuotaWindow
                w={b.primary}
                label="主窗口"
                now={now}
                isStale={qStale}
              />
              <QuotaWindow
                w={b.secondary}
                label="次窗口"
                now={now}
                isStale={qStale}
              />
              {b.credits && (
                <small className="muted">
                  Credits：
                  {b.credits.unlimited
                    ? "不限量"
                    : (b.credits.balance ?? "未知")}
                  （保留服务端单位）
                </small>
              )}
            </div>
          ))}
          <div className="panel-foot">
            上次同步{" "}
            {quota.collected_at ? localTime(quota.collected_at) : "未知"} ·
            每分钟刷新
          </div>
        </>
      )}
      <details className="quota-help">
        <summary>额度口径与状态说明</summary>
        <p>
          额度来自服务端，可能跨客户端与设备共享。本地 Token
          不等于账户全部用量，也不能换算为套餐剩余。
        </p>
        <p>
          窗口长度与重置时间按接口返回展示；快照过期、重置或请求失败时显示待同步
          / 不可用。
        </p>
        <p>达到 80% 已使用时显示醒目提示；额度风险预警记录可在预警页查看。</p>
      </details>
    </section>
  );
  return (
    <div className="app">
      <a className="skip-link" href="#main-content">
        跳到主内容
      </a>
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-icon">
            <img
              src={
                new URL("../../src-tauri/icons/source.svg", import.meta.url)
                  .href
              }
              alt=""
              aria-hidden="true"
              width={34}
              height={34}
            />
          </div>
          <span>
            Token<span className="brand-accent">Scope</span>
            <small>本地用量工作台</small>
          </span>
        </div>
        <div className="workspace">
          <ProviderMark provider="codex" size={17} />
          <div>
            个人工作空间<small>本地优先 · OpenAI Codex</small>
          </div>
          <span className="dot" />
        </div>
        <div className="nav-label">工作台</div>
        <nav aria-label="主导航">
          {nav.map((n) => (
            <button
              key={n.page}
              className={page === n.page ? "nav-item active" : "nav-item"}
              onClick={() => {
                setPage(n.page);
                setSelected(null);
              }}
              title={n.label + " · Alt+" + (nav.indexOf(n) + 1)}
              aria-label={n.label}
              aria-current={page === n.page ? "page" : undefined}
            >
              <n.icon size={18} />
              {n.label}
              {n.page === "alerts" &&
                unreadAlerts !== null &&
                unreadAlerts > 0 && (
                  <span className="alert-badge" aria-hidden="true">
                    {unreadAlerts}
                  </span>
                )}
              {n.page === "overview" && <span className="nav-key">01</span>}
            </button>
          ))}
        </nav>
        <div className="nav-label future-label">即将接入</div>
        <div className="future">
          Claude Code <span>规划中</span>
        </div>
        <div className="future">DeepSeek / GLM / Kimi</div>
        <div className="sidebar-bottom">
          <ShieldCheck size={17} />
          <div>
            数据留在你的设备<small>不采集对话正文或凭据</small>
          </div>
        </div>
        <button
          className="ui-button ui-button-outline theme-toggle"
          aria-label={theme === "light" ? "切换深色主题" : "切换浅色主题"}
          onClick={() => setTheme(theme === "light" ? "dark" : "light")}
        >
          {theme === "light" ? <Moon size={16} /> : <Sun size={16} />}
          <span>{theme === "light" ? "深色外观" : "浅色外观"}</span>
        </button>
      </aside>
      <main id="main-content" tabIndex={-1}>
        <header>
          <div className="breadcrumb">
            工作台 <span>/</span> {titles[page]}
          </div>
          <div className="header-right">
            <span className="live-pill">
              <i
                className={
                  health?.collector.status === "synced" &&
                  collectorLabel(health.collector, now) === "本地采集已连接"
                    ? "dot"
                    : "dot muted-dot"
                }
              />
              {desktop
                ? collectorLabel(health?.collector, now)
                : "浏览器预览 · 未连接"}
            </span>
            <AccountAvatar />
          </div>
        </header>
        <div className="content">
          <div className="title-row">
            <div>
              <div className="eyebrow">YOUR TOKENS, IN FOCUS</div>
              <h1>{titles[page]}</h1>
              <p>
                {page === "overview"
                  ? "核心用量、活跃日历与账户额度，一览工作负载。"
                  : page === "timeline"
                    ? "按时间查看消耗分布，并追溯具体计量事件。"
                    : "从每一次调用，了解你的 AI 工作流。"}
              </p>
            </div>
            <Button
              variant="outline"
              onClick={() => {
                void invoke("retry_ingest").catch(() =>
                  setError("采集服务重试失败"),
                );
                setTick((t) => t + 1);
              }}
              disabled={!desktop || loading}
              aria-busy={loading}
            >
              <RefreshCw size={15} className={loading ? "spin" : ""} />
              {loading ? "读取中" : "刷新并采集"}
            </Button>
          </div>
          {health?.test_mode && (
            <div className="notice" role="status">
              开发验证模式 · 合成 Fixture
              与真实账号数据隔离；这些指标不代表真实消耗。
            </div>
          )}
          {!desktop && (
            <div className="notice">
              当前为界面预览，未连接桌面后端。运行 npm run desktop
              读取本机真实数据。
            </div>
          )}
          {error && (
            <div role="alert" className="notice danger">
              {error}
            </div>
          )}
          {health &&
            (health.collector.status !== "synced" ||
              collectorLabel(health.collector, now) === "采集状态已过期" ||
              health.report.io_errors > 0 ||
              health.report.parse_errors > 0) && (
              <div role="status" className="notice">
                {health.collector.status === "missing"
                  ? "未发现 Codex 会话目录。"
                  : collectorLabel(health.collector, now)}
                {health.collector.status === "error"
                  ? " 数据库或采集服务异常；历史数据保留。"
                  : null}
                {health.collector.next_retry &&
                health.collector.status !== "synced"
                  ? " 下次重试：" +
                    localTime(health.collector.next_retry) +
                    "。"
                  : null}
                {health.collector.last_success
                  ? " 上次完整采集：" +
                    localTime(health.collector.last_success) +
                    "。"
                  : null}
                {health.report.io_errors > 0
                  ? " 部分文件读取失败：" + health.report.io_errors
                  : null}
                {health.report.parse_errors > 0
                  ? " 已跳过无效行：" + health.report.parse_errors
                  : null}
              </div>
            )}
          {data && data.partial_histories > 0 && (
            <div className="notice" role="status">
              已凭响应证据确认本次消耗。另有{" "}
              {exact(data.carried_baseline.total)} Token
              历史累计缺少时间/模型归属，单独保留，未计入当前统计或成本。
            </div>
          )}
          {data && data.uncertain > 0 && (
            <div className="notice">
              {data.uncertain}{" "}
              条快照无法确认增量，未计入总量。当前统计可能不完整。
            </div>
          )}
          {page !== "settings" && page !== "alerts" && (
            <div className="filters">
              <div className="filter-heading">
                <span className="dot" />
                OpenAI Codex <span className="muted">/ 本地会话</span>
              </div>
              <select
                aria-label="时间范围"
                value={period}
                onChange={(e) => setPeriod(e.target.value)}
              >
                <option value="all">全部时间</option>
                <option value="today">今天（本地时间）</option>
                <option value="7">最近 7 天</option>
                <option value="30">最近 30 天</option>
              </select>
              {page === "timeline" && (
                <select
                  aria-label="趋势粒度"
                  value={grain}
                  onChange={(e) => setGrain(e.target.value as Grain)}
                >
                  <option value="minute">按分钟</option>
                  <option value="hour">按小时</option>
                  <option value="day">按天</option>
                </select>
              )}
              <select
                aria-label="项目筛选"
                value={project}
                onChange={(e) => setProject(e.target.value)}
              >
                <option value="">全部项目</option>
                {Array.from(
                  new Map(
                    options?.sessions.map((s) => [s.project_id, s.project]),
                  ).entries(),
                ).map(([id, name]) => (
                  <option key={id} value={id}>
                    {name}
                  </option>
                ))}
              </select>
              <select
                aria-label="模型筛选"
                value={model}
                onChange={(e) => setModel(e.target.value)}
              >
                <option value="">全部模型</option>
                {options?.models.map((m) => (
                  <option key={m.name}>{m.name}</option>
                ))}
              </select>
            </div>
          )}
          {page === "timeline" && (
            <ExportUsage
              filter={() => usageFilter(period, project, model, grain)}
              projectLabel={
                options?.sessions.find((s) => s.project_id === project)?.project
              }
            />
          )}
          {page === "overview" && (
            <>
              <div className="metrics">
                {totals ? (
                  <>
                    <Metric
                      label="已确认 Token"
                      value={totals.total}
                      note="输入 + 输出，子集不重复相加"
                      icon={Sparkles}
                    />
                    <Metric
                      label="输入 Token"
                      value={totals.input}
                      note={"其中缓存 " + compact(totals.cached)}
                      icon={ArrowUpRight}
                    />
                    <Metric
                      label="输出 Token"
                      value={totals.output}
                      note={"其中推理 " + compact(totals.reasoning)}
                      icon={ArrowDownLeft}
                    />
                    <div className="metric">
                      <div className="metric-label">
                        缓存命中率
                        <Database size={17} />
                      </div>
                      <strong>
                        {data?.cache_percent === null
                          ? "—"
                          : data?.cache_percent.toFixed(1) + "%"}
                      </strong>
                      <small>
                        {data?.active_sessions} 个会话近 15 分钟有用量
                      </small>
                    </div>
                  </>
                ) : (
                  [
                    "已确认 Token",
                    "输入 Token",
                    "输出 Token",
                    "缓存命中率",
                  ].map((t) => (
                    <div className="metric" key={t}>
                      <div className="metric-label">{t}</div>
                      <strong>—</strong>
                      <small>{loading ? "正在读取" : "尚无已连接的数据"}</small>
                    </div>
                  ))
                )}
              </div>
              <section className="provider-strip" aria-label="当前数据供应商">
                <ProviderMark label />
                <div>
                  <strong>Codex</strong>
                  <span>
                    本地会话用量 · {data?.active_sessions ?? 0} 个近 15
                    分钟活跃会话
                  </span>
                </div>
                <LiveRate
                  period={period}
                  project={project}
                  model={model}
                  health={health}
                />
                <span className="provider-caption">仅展示实际接入的数据源</span>
              </section>
              <ActivityHeatmap data={data?.activity} />
              <section className="overview-drilldown" aria-label="时间线入口">
                <BarChart3 size={22} />
                <div>
                  <strong>深入查看消耗时间线</strong>
                  <p>
                    折线、柱状、K 线与计量事件集中在时间线，沿用当前时间 / 项目
                    / 模型筛选。
                  </p>
                </div>
                <Button
                  variant="outline"
                  onClick={() => {
                    setPage("timeline");
                    setSelected(null);
                  }}
                >
                  查看消耗时间线 <ArrowUpRight size={15} />
                </Button>
              </section>
              <div className="overview-grid overview-account">
                {showQuota()}
                <Cost
                  estimate={data?.estimate}
                  currency={currency}
                  setCurrency={setCurrency}
                  refresh={() => void refreshFx()}
                  busy={fxBusy}
                />
              </div>
            </>
          )}
          {(page === "overview" || page === "sessions") && (
            <section className="panel session-panel">
              <div className="panel-head">
                <div>
                  <h2>
                    {page === "overview" ? "最近会话" : "会话历史"}{" "}
                    <span className="count">{sessions.length}</span>
                  </h2>
                  <p>按最近用量排序 · 点击查看轮次与计量事件</p>
                </div>
                <Folder size={18} className="muted" />
              </div>
              {sessions.length ? (
                <div className="table-scroll">
                  <table>
                    <thead>
                      <tr>
                        <th>项目 / 会话</th>
                        <th>模型</th>
                        <th>输入</th>
                        <th>缓存输入</th>
                        <th>输出</th>
                        <th>总 Token</th>
                        <th>最近用量</th>
                      </tr>
                    </thead>
                    <tbody>
                      {(page === "overview"
                        ? sessions.slice(0, 6)
                        : sessions
                      ).map((s) => (
                        <tr key={s.id}>
                          <td>
                            <button
                              className="session-link"
                              onClick={() => setSelected(s)}
                            >
                              <span className="session-icon">
                                <ProviderMark provider="codex" />
                              </span>
                              <span>
                                {s.project}
                                <small>
                                  {s.source_id.slice(0, 18)}
                                  {s.uncertain > 0 ? " · 存在不确定数据" : ""}
                                </small>
                              </span>
                            </button>
                          </td>
                          <td>
                            <span className="model-tag">
                              {s.models.map((m) => (
                                <span className="model-label" key={m}>
                                  <ProviderMark model={m} />
                                  {m}
                                </span>
                              ))}
                            </span>
                          </td>
                          {[
                            s.totals.input,
                            s.totals.cached,
                            s.totals.output,
                            s.totals.total,
                          ].map((v, i) => (
                            <td
                              key={i}
                              title={exact(v)}
                              className={i === 3 ? "number accent" : "number"}
                            >
                              {compact(v)}
                            </td>
                          ))}
                          <td className="muted">{localTime(s.last_at)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <div className="empty">
                  <Layers3 size={30} />
                  <strong>
                    {loading ? "正在加载会话" : "没有可显示的会话"}
                  </strong>
                  <span>开始使用 Codex 后，此处会自动出现真实用量。</span>
                </div>
              )}
            </section>
          )}
          {page === "timeline" && (
            <>
              <section className="panel">
                <div className="panel-head">
                  <div>
                    <h2>消耗趋势与分布</h2>
                    <p>
                      {intervalLabel(data?.interval_seconds ?? 86400)} / 点 ·
                      超长范围自动合并，最多 720 点 · 本地时间显示
                    </p>
                  </div>
                </div>
                <Trend
                  groups={data?.timeline ?? []}
                  interval={data?.interval_seconds}
                  candles={data?.candles}
                />
              </section>
              <section className="panel session-panel timeline-events">
                <div className="panel-head">
                  <h2>
                    计量事件 <span className="count">最近 500 条</span>
                  </h2>
                </div>
                <div className="table-scroll">
                  <table>
                    <thead>
                      <tr>
                        <th>时间（本地）</th>
                        <th>模型</th>
                        <th>增量 Token</th>
                        <th>轮次关联</th>
                        <th>计数区段</th>
                        <th>质量</th>
                      </tr>
                    </thead>
                    <tbody>
                      {data?.events.map((e, i) => (
                        <tr key={i}>
                          <td>{localTime(e.at)}</td>
                          <td>
                            <span className="model-label">
                              <ProviderMark model={e.model} />
                              {e.model}
                            </span>
                          </td>
                          <td className="number">
                            {e.delta ? exact(e.delta.total) : "未知"}
                          </td>
                          <td>{e.turn?.slice(0, 16) ?? "未关联"}</td>
                          <td>{e.epoch + 1}</td>
                          <td>
                            {e.quality === "verified_reset"
                              ? "重置已核实"
                              : e.quality}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                  {!data?.events.length && (
                    <div className="empty">暂无计量事件</div>
                  )}
                </div>
              </section>
            </>
          )}
          {page === "alerts" && <Alerts />}
          {page === "settings" && (
            <>
              <Alerts settings />
              <section className="panel">
                <div className="panel-head">
                  <div>
                    <h2>Provider 能力</h2>
                    <p>独立适配接口 · 仅 Codex 已实现</p>
                  </div>
                </div>
                {(
                  data?.providers ?? [
                    {
                      id: "codex",
                      name: "OpenAI Codex",
                      status: "implemented",
                    },
                    {
                      id: "claude-code",
                      name: "Claude Code",
                      status: "unsupported",
                    },
                    { id: "deepseek", name: "DeepSeek", status: "unsupported" },
                    { id: "glm", name: "GLM / 智谱", status: "unsupported" },
                    { id: "kimi", name: "Kimi", status: "unsupported" },
                  ]
                ).map((p) => (
                  <div className="provider" key={p.id}>
                    <ProviderMark provider={p.id} size={19} />
                    <span>{p.name}</span>
                    <span
                      className={p.status === "implemented" ? "tag" : "muted"}
                    >
                      {p.status === "implemented"
                        ? "日志增量采集 + 官方额度接口"
                        : "未支持"}
                    </span>
                  </div>
                ))}
              </section>
              <section className="panel explanation">
                <h2>数据与隐私</h2>
                <p>
                  只读 CODEX_HOME（默认 ~/.codex）中的 sessions 与
                  archived_sessions。每 3 秒检查追加数据，SQLite
                  保存脱敏计量字段与事务游标。
                </p>
                <p>
                  不保存提示词、回复、代码和认证文件。项目路径及账户身份用哈希保存，仅显示项目目录名。
                </p>
                <p>
                  额度通过本机 Codex App Server 读取；需要 Codex 已登录。可用
                  TOKENSCOPE_CODEX_BIN 指定 codex.exe 路径。
                </p>
                <p>
                  计数重置仅在逐响应证据严格核对通过时恢复；缺少证据时标记不确定并排除后续增量。轮次依据日志
                  turn_id 关联，不代表每个用户问题的精确账单。
                </p>
              </section>
              <section className="panel explanation">
                <h2>采集健康状态</h2>
                <p>数据库位置：{health?.database_path ?? "尚未读取"}</p>
                {health?.last_error && (
                  <p role="alert">失败原因：{health.last_error}</p>
                )}
                <p>
                  {collectorLabel(health?.collector, now)} · 自动恢复{" "}
                  {health?.collector.recoveries ?? 0} 次
                </p>
                <p>
                  上次尝试：
                  {health?.collector.last_attempt
                    ? localTime(health.collector.last_attempt)
                    : "尚未采集"}
                  ；上次完整采集：
                  {health?.collector.last_success
                    ? localTime(health.collector.last_success)
                    : "尚未成功"}
                </p>
                <p>
                  正常每 3
                  秒检查日志追加。源目录缺失、文件读取失败或数据库异常时，重试间隔逐步增加至
                  30
                  秒；“刷新并采集”可立即触发下一次检查。不会删除历史数据或改写
                  Codex 日志。
                </p>
              </section>
              <section className="panel explanation">
                <h2>价格与每日参考汇率</h2>
                <p>
                  价格版本：{data?.estimate.catalog.id ?? "尚未读取"}。已核实
                  GPT-6.1 Sol、GPT-6 Luna、GPT-5.6 Luna
                  标准短上下文公开价；其他模型明确显示单价未知。
                </p>
                <label>
                  <input
                    type="checkbox"
                    aria-label="每日自动更新参考汇率"
                    checked={
                      fxEnabledPending ?? data?.estimate.fx.enabled ?? false
                    }
                    disabled={!desktop || fxEnabledPending !== null}
                    onChange={(e) => void toggleFx(e.target.checked)}
                  />{" "}
                  每日自动更新参考汇率
                </label>
                <p>
                  启用后请求 Frankfurter 公共汇率接口，仅发送 USD/HKD/CNY
                  币种参数，不发送项目、Token、账号或凭据。每小时检查本地缓存，24
                  小时到期后更新；观测日期超过 7
                  天或同步失败时暂停换算。参考汇率不保证实际结算汇率。
                </p>
                <p>
                  价格来源：developers.openai.com
                  官方模型页；汇率来源：frankfurter.dev，可在总览查看数据观测日期与采集时间。
                </p>
              </section>
            </>
          )}
          {page === "overview" && (
            <div className="bottom-grid">
              <UsageDistribution groups={data?.models} model />
              <ProjectBubbles groups={data?.projects} />
            </div>
          )}
          <footer>
            <span>
              <ShieldCheck size={13} />
              本地存储 · 不上传个人用量数据
            </span>
            <span>TokenScope v0.5.16 · 真实数据优先</span>
          </footer>
        </div>
      </main>
      {selected && (
        <div className="modal-backdrop" onClick={() => setSelected(null)}>
          <section
            role="dialog"
            aria-modal="true"
            aria-labelledby="detail-title"
            className="detail panel"
            onClick={(e) => e.stopPropagation()}
            onKeyDown={(e) => {
              if (e.key === "Escape") setSelected(null);
              if (e.key === "Tab") {
                const nodes = Array.from(
                  e.currentTarget.querySelectorAll<HTMLElement>(
                    'button, [href], select, input, [tabindex="0"]',
                  ),
                );
                const first = nodes[0],
                  last = nodes[nodes.length - 1];
                if (e.shiftKey && document.activeElement === first) {
                  e.preventDefault();
                  last?.focus();
                } else if (!e.shiftKey && document.activeElement === last) {
                  e.preventDefault();
                  first?.focus();
                }
              }
            }}
          >
            <div className="panel-head">
              <div>
                <h2 id="detail-title">{selected.project} · 会话详情</h2>
                <p>{selected.source_id}</p>
              </div>
              <Button
                autoFocus
                variant="outline"
                aria-label="关闭会话详情"
                onClick={() => setSelected(null)}
              >
                <X size={18} />
              </Button>
            </div>
            <div className="detail-body">
              <div className="metrics detail-metrics">
                {(["input", "cached", "output", "reasoning"] as const).map(
                  (key, i) => (
                    <Metric
                      key={key}
                      label={["输入", "缓存输入", "输出", "推理输出"][i]}
                      value={selected.totals[key]}
                      note="当前筛选范围"
                      icon={Clock3}
                    />
                  ),
                )}
              </div>
              <h3>轮次增量（日志关联）</h3>
              <p className="muted">
                内部调用依据 turn_id 关联；缺失时保留为未关联。
                {detail?.turns_truncated
                  ? " 当前显示本会话最近 500 个轮次，汇总包含全部轮次。"
                  : ""}
                {!detail ? " 正在读取轮次…" : ""}
              </p>
              <div className="table-scroll">
                <table>
                  <thead>
                    <tr>
                      <th>轮次</th>
                      <th>输入</th>
                      <th>输出</th>
                      <th>总 Token</th>
                      <th>不确定快照</th>
                    </tr>
                  </thead>
                  <tbody>
                    {detail?.turns
                      .filter((t) => t.session === selected.id)
                      .map((t) => (
                        <tr key={t.id}>
                          <td title={t.id}>{t.id.slice(0, 20)}</td>
                          <td>{exact(t.totals.input)}</td>
                          <td>{exact(t.totals.output)}</td>
                          <td>{exact(t.totals.total)}</td>
                          <td>{t.uncertain}</td>
                        </tr>
                      ))}
                  </tbody>
                </table>
              </div>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}
