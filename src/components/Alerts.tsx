import { useEffect, useState, useRef } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Bell, Check, LoaderCircle } from "lucide-react";
import { Button } from "./ui/button";
import { localTime } from "../lib/format";
import { normalizeThreshold } from "../lib/alertRules";
interface Inbox {
  enabled: boolean;
  usage_threshold: string;
  unread: number;
  total: number;
  page: number;
  pages: number;
  page_size: number;
  as_of: string | null;
  items: {
    id: string;
    kind: string;
    severity: string;
    title: string;
    detail: string;
    created_at: string;
    updated_at: string;
    acknowledged_at: string | null;
  }[];
}
export function Alerts({ settings = false }: { settings?: boolean }) {
  const [inbox, setInbox] = useState<Inbox | null>(null),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [threshold, setThreshold] = useState(""),
    [enabled, setEnabled] = useState(false);
  const [query, setQuery] = useState({
    page: 0,
    kind: "",
    severity: "",
    state: "all",
    from: "",
    to: "",
    before: null as string | null,
  });
  const [loading, setLoading] = useState(false);
  const savedRules = useRef("");
  const dirty = useRef(false);
  const saving = useRef(false);
  const revision = useRef(0);
  const [saveMessage, setSaveMessage] = useState("");
  const [readError, setReadError] = useState("");
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    setInbox(null);
    let running = false,
      queued = false;
    const refresh = () => {
      queued = true;
      if (running || disposed) return;
      running = true;
      setLoading(true);
      void (async () => {
        try {
          while (queued && !disposed) {
            queued = false;
            try {
              const requestRevision = revision.current;
              const d = await invoke<Inbox>("read_alerts", {
                filter: {
                  ...query,
                  before: query.before,
                  kind: query.kind || null,
                  severity: query.severity || null,
                  from: query.from ? new Date(query.from).toISOString() : null,
                  to: query.to ? new Date(query.to).toISOString() : null,
                },
              });
              if (disposed) return;
              if (requestRevision !== revision.current || saving.current)
                continue;
              setInbox(d);
              setReadError("");
              const key = d.enabled + ":" + d.usage_threshold;
              if (!dirty.current && savedRules.current !== key) {
                savedRules.current = key;
                setThreshold(d.usage_threshold);
                setEnabled(d.enabled);
              }
            } catch {
              if (!disposed)
                setReadError("预警数据不可用，请检查本地数据库或时间筛选");
            }
          }
        } finally {
          running = false;
          if (!disposed) setLoading(false);
        }
      })();
    };
    refresh();
    const pending = listen("alerts-updated", refresh);
    return () => {
      disposed = true;
      void pending.then((fn) => fn());
    };
  }, [query]);
  const action = async (command: string, args: Record<string, unknown>) => {
    if (saving.current) return;
    saving.current = true;
    revision.current++;
    setBusy(true);
    setError("");
    setSaveMessage("");
    try {
      await invoke(command, args);
      if (command === "set_alert_rules") {
        const confirmed = await invoke<Inbox>("read_alerts");
        if (
          confirmed.usage_threshold !== args.threshold ||
          confirmed.enabled !== args.enabled
        )
          throw new Error("Rules readback differs");
        savedRules.current =
          confirmed.enabled + ":" + confirmed.usage_threshold;
        dirty.current = false;
        setInbox(confirmed);
        setThreshold(confirmed.usage_threshold);
        setEnabled(confirmed.enabled);
        setSaveMessage(
          "已保存并生效 · 阈值 " +
            BigInt(confirmed.usage_threshold).toLocaleString() +
            " Token",
        );
      } else {
        setQuery((previous) => ({ ...previous }));
      }
    } catch {
      setError(
        command === "set_alert_rules"
          ? "保存失败：检查数据库；阈值须为 1 至 18446744073709551615 的整数"
          : "预警状态保存失败，请重试",
      );
    } finally {
      revision.current++;
      saving.current = false;
      setBusy(false);
    }
  };
  return (
    <section className="panel explanation alert-panel">
      <div className="panel-head">
        <div>
          <h2>{settings ? "监控规则" : "本地预警历史"}</h2>
          <p>
            {settings
              ? "规则只使用已确认用量与新鲜服务端额度"
              : "保留触发依据 · 分页查询全部历史 · 已读状态跨重启保存"}
          </p>
        </div>
        <Bell size={20} />
      </div>
      {(error || readError) && (
        <p role="alert" className="warning">
          {error || readError}
        </p>
      )}
      {!settings && (
        <div className="alert-filters">
          <select
            aria-label="预警类型"
            value={query.kind}
            onChange={(e) =>
              setQuery({
                ...query,
                kind: e.target.value,
                page: 0,
                before: null,
              })
            }
          >
            <option value="">全部类型</option>
            <option value="usage_high">高用量</option>
            <option value="quota_risk">额度风险</option>
          </select>
          <select
            aria-label="预警级别"
            value={query.severity}
            onChange={(e) =>
              setQuery({
                ...query,
                severity: e.target.value,
                page: 0,
                before: null,
              })
            }
          >
            <option value="">全部级别</option>
            <option value="warning">提示</option>
            <option value="critical">严重</option>
          </select>
          <select
            aria-label="预警已读状态"
            value={query.state}
            onChange={(e) =>
              setQuery({
                ...query,
                state: e.target.value,
                page: 0,
                before: null,
              })
            }
          >
            <option value="all">全部状态</option>
            <option value="unread">未读</option>
            <option value="read">已读</option>
          </select>
          <label>
            首次触发从
            <input
              type="datetime-local"
              aria-label="预警开始时间"
              value={query.from}
              onChange={(e) =>
                setQuery({
                  ...query,
                  from: e.target.value,
                  page: 0,
                  before: null,
                })
              }
            />
          </label>
          <label>
            至（不含）
            <input
              type="datetime-local"
              aria-label="预警结束时间"
              value={query.to}
              onChange={(e) =>
                setQuery({
                  ...query,
                  to: e.target.value,
                  page: 0,
                  before: null,
                })
              }
            />
          </label>
          <Button
            variant="outline"
            disabled={loading}
            onClick={() => setQuery({ ...query, page: 0, before: null })}
          >
            刷新预警列表
          </Button>
        </div>
      )}
      {!isTauri() ? (
        <div className="empty">未连接桌面预警服务</div>
      ) : !inbox ? (
        <div className="empty">
          {readError ? "预警读取失败" : "正在读取预警"}
        </div>
      ) : settings ? (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            const normalized = normalizeThreshold(threshold);
            if (!normalized) {
              setError(
                "保存失败：阈值须为 1 至 18446744073709551615 的整数，可使用千位逗号",
              );
              return;
            }
            void action("set_alert_rules", { enabled, threshold: normalized });
          }}
        >
          <label className="rule-toggle">
            <input
              type="checkbox"
              checked={enabled}
              disabled={busy}
              onChange={(e) => {
                dirty.current = true;
                setSaveMessage("有未保存的修改");
                setEnabled(e.target.checked);
              }}
            />
            启用本地监控规则
          </label>
          <label className="rule-threshold">
            每会话五分钟 Token 阈值
            <input
              aria-label="五分钟 Token 阈值"
              inputMode="numeric"
              value={threshold}
              disabled={busy}
              onChange={(e) => {
                dirty.current = true;
                setSaveMessage("有未保存的修改");
                setError("");
                setThreshold(e.target.value);
              }}
              required
            />
          </label>
          <p>
            默认 1,000,000 Token；使用 UTC
            对齐的固定五分钟窗口，只统计已经确认的输入 +
            输出，缓存/推理不重复相加。阈值提示不能证明消耗异常。
          </p>
          <p>
            额度达到已使用 80% 提示、95%
            严重提示；只接受采集不足五分钟且尚未重置的服务端窗口。关闭规则保留历史记录。
          </p>
          <p role="status" className="rule-save-status">
            {saveMessage || "修改后点击保存监控规则；已有预警保留在历史中。"}
          </p>
          <Button disabled={busy} aria-busy={busy} type="submit">
            {busy && <LoaderCircle size={14} aria-hidden="true" />}
            {busy ? "保存中" : "保存监控规则"}
          </Button>
        </form>
      ) : (
        <>
          <p>
            {inbox.enabled ? "监控规则已启用" : "监控规则已关闭，仅显示历史"} ·{" "}
            {inbox.unread} 条全局未读；历史提示不代表当前额度状态。
          </p>
          <div className="alert-pagination">
            <span>
              {inbox.total} 条匹配 · 第 {inbox.page + 1} / {inbox.pages} 页 ·
              每页 {inbox.page_size} 条
            </span>
            <div>
              <Button
                variant="outline"
                disabled={loading || inbox.page === 0}
                onClick={() =>
                  setQuery({
                    ...query,
                    page: inbox.page - 1,
                    before: inbox.as_of,
                  })
                }
              >
                上一页
              </Button>
              <Button
                variant="outline"
                disabled={loading || inbox.page + 1 >= inbox.pages}
                onClick={() =>
                  setQuery({
                    ...query,
                    page: inbox.page + 1,
                    before: inbox.as_of,
                  })
                }
              >
                下一页
              </Button>
            </div>
          </div>
          <p className="muted">
            列表固定到 {inbox.as_of ? localTime(inbox.as_of) : "当前查询时刻"}{" "}
            的首次触发记录；首次打开跟随新预警，翻页后固定范围，刷新纳入新预警。已读与等级显示最新状态。
          </p>
          {inbox.items.length === 0 ? (
            <div className="empty">
              <Check size={24} />
              <strong>当前筛选没有预警记录</strong>
              <span>达到已配置阈值或有效额度风险条件时才会创建提示。</span>
            </div>
          ) : (
            inbox.items.map((a) => (
              <article className={"alert-item " + a.severity} key={a.id}>
                <div className="between">
                  <h3>{a.title}</h3>
                  <span className="tag">
                    {a.severity === "critical" ? "严重" : "提示"}
                  </span>
                </div>
                <p>{a.detail}</p>
                <div className="between muted">
                  <small>
                    首次 {localTime(a.created_at)} · 更新{" "}
                    {localTime(a.updated_at)}
                  </small>
                  <Button
                    variant="outline"
                    disabled={busy || loading || !!a.acknowledged_at}
                    onClick={() =>
                      void action("acknowledge_alert", { id: a.id })
                    }
                  >
                    {a.acknowledged_at ? "已读" : "标为已读"}
                  </Button>
                </div>
              </article>
            ))
          )}
        </>
      )}
    </section>
  );
}
