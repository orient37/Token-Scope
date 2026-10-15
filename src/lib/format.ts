export function exact(value: string) {
  return BigInt(value).toLocaleString("zh-CN");
}
export function compact(value: string) {
  const n = BigInt(value);
  for (const [unit, div] of [
    ["B", 1000000000n],
    ["M", 1000000n],
    ["K", 1000n],
  ] as const) {
    if (n >= div) {
      const hundred = (n * 100n) / div;
      return (
        (hundred / 100n).toString() +
        "." +
        (hundred % 100n).toString().padStart(2, "0") +
        unit
      );
    }
  }
  return n.toString();
}
export function localTime(s: string) {
  return new Date(s).toLocaleString("zh-CN", { hour12: false });
}
export function resetLabel(at: number | null, now: number) {
  if (at === null) return "重置时间未知";
  const seconds = Math.ceil((at * 1000 - now) / 1000);
  if (seconds <= 0) return "已到重置时间 · 等待同步";
  const mins = Math.ceil(seconds / 60);
  return mins >= 1440
    ? Math.floor(mins / 1440) +
        "天 " +
        Math.floor((mins % 1440) / 60) +
        "小时后重置"
    : mins >= 60
      ? Math.floor(mins / 60) + "小时 " + (mins % 60) + "分钟后重置"
      : mins + "分钟后重置";
}
export function stale(at: string | null, now: number) {
  return !at || now - new Date(at).getTime() > 300000;
}
