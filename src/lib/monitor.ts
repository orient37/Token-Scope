import type { Health } from "./types";
export function collectorLabel(
  collector: Health["collector"] | undefined,
  now: number,
) {
  if (!collector) return "等待采集";
  if (
    collector.next_retry &&
    now > new Date(collector.next_retry).getTime() + 15000
  )
    return "采集状态已过期";
  return {
    starting: "正在连接数据源",
    synced: "本地采集已连接",
    missing: "等待数据源",
    degraded: "部分采集失败 · 将重试",
    error: "采集异常 · 将重试",
  }[collector.status];
}
export function intervalLabel(seconds: number) {
  if (seconds % 86400 === 0) return seconds / 86400 + " 天";
  if (seconds % 3600 === 0) return seconds / 3600 + " 小时";
  return seconds / 60 + " 分钟";
}
