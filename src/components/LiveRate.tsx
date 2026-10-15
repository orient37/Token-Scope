import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Health } from "../lib/types";
import { usageFilter } from "../lib/filters";
import { exact, localTime } from "../lib/format";
interface Rate {
  tokens_per_second: string | null;
  tokens_per_minute: string | null;
  collected_at: string;
  window_start: string;
  unknown_events: number;
  confirmed_events: number;
}
function formattedRate(value: string) {
  const [whole, fraction] = value.split(".");
  return fraction === undefined ? exact(whole) : `${exact(whole)}.${fraction}`;
}

export function LiveRate({
  period,
  project,
  model,
  health,
}: {
  period: string;
  project: string;
  model: string;
  health: Health | null;
}) {
  const [rate, setRate] = useState<Rate | null>(null);
  const [failed, setFailed] = useState(false);
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    let disposed = false,
      busy = false,
      queued = false;
    setRate(null);
    setFailed(false);
    const refresh = async () => {
      if (disposed || !isTauri()) return;
      queued = true;
      if (busy) return;
      busy = true;
      while (queued && !disposed) {
        queued = false;
        try {
          const value = await invoke<Rate>("read_output_rate", {
            filter: usageFilter(period, project, model, "minute"),
          });
          if (!disposed) {
            setNow(Date.now());
            setRate(value);
            setFailed(false);
          }
        } catch {
          if (!disposed) setFailed(true);
        }
      }
      busy = false;
    };
    const wake = () => void refresh();
    const visibility = () => {
      if (document.visibilityState === "visible") wake();
    };
    const unlisten = isTauri()
      ? listen("usage-updated", wake).catch(() => () => {})
      : null;
    // Polling also ages events out when no new usage event arrives.
    void refresh();
    const timer = setInterval(() => void refresh(), 3000);
    const clock = setInterval(() => setNow(Date.now()), 1000);
    window.addEventListener("focus", wake);
    document.addEventListener("visibilitychange", visibility);
    return () => {
      disposed = true;
      clearInterval(timer);
      clearInterval(clock);
      window.removeEventListener("focus", wake);
      document.removeEventListener("visibilitychange", visibility);
      void unlisten?.then((stop) => stop()).catch(() => {});
    };
  }, [period, project, model]);
  const connected =
    health?.collector.status === "synced" &&
    !!health.collector.last_success &&
    now - Date.parse(health.collector.last_success) <= 15000;
  const available =
    connected &&
    !failed &&
    !!rate &&
    (rate.unknown_events === 0 || rate.confirmed_events > 0) &&
    !!rate.tokens_per_second &&
    !!rate.tokens_per_minute &&
    now - Date.parse(rate.collected_at) <= 10000;
  const description = `近 60 秒已确认可见输出吞吐率：输出 Token − 推理输出 Token，排除输入与缓存。按计量事件时间归入窗口，当前筛选下多会话合计；每秒值为窗口总量 ÷ 60，每分钟值为窗口总量。包含工具等待和空闲时间，不是模型解码速度。日志在响应结束后更新计数，页面每 3 秒刷新。${rate ? `窗口：${localTime(rate.window_start)} — ${localTime(rate.collected_at)}。${rate.unknown_events} 条未知计量未纳入。` : ""}`;
  return (
    <div className="live-rate" aria-label="可见输出吞吐率" title={description}>
      <div className="live-rate-values">
        <span>
          <strong>
            {available && rate?.tokens_per_second
              ? formattedRate(rate.tokens_per_second)
              : "—"}
          </strong>{" "}
          tokens/s
        </span>
        <span>
          <strong>
            {available && rate?.tokens_per_minute
              ? formattedRate(rate.tokens_per_minute)
              : "—"}
          </strong>{" "}
          tokens/m
        </span>
      </div>
      <small>
        {available
          ? `可见输出吞吐率 · 近 60 秒${rate?.unknown_events ? " · 仅已确认部分" : ""}`
          : failed
            ? "输出吞吐率读取失败"
            : !connected
              ? "输出吞吐率未知 · 采集未就绪 / 已过期"
              : !rate
                ? "正在读取输出吞吐率"
                : now - Date.parse(rate.collected_at) > 10000
                  ? "输出吞吐率未知 · 读数已过期"
                  : "输出吞吐率未知 · 窗口内计量未确认"}
      </small>
      <small>每 3 秒刷新 · 含工具等待 / 空闲 · 非模型解码速度</small>
    </div>
  );
}
