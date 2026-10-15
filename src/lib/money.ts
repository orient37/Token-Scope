export type Currency = "USD" | "HKD" | "CNY";
export function money(value: string | null, currency: Currency) {
  if (value === null) return "—";
  const [whole, fraction = ""] = value.split(".");
  const nanos = BigInt(whole) * 1000000000n + BigInt(fraction.padEnd(9, "0"));
  const cents = (nanos + 5000000n) / 10000000n;
  return (
    currency +
    " " +
    (cents / 100n).toLocaleString("zh-CN") +
    "." +
    (cents % 100n).toString().padStart(2, "0")
  );
}
export function coverage(covered: string, uncovered: string) {
  const c = BigInt(covered),
    u = BigInt(uncovered);
  if (c + u === 0n) return "—";
  const tenths = (c * 1000n) / (c + u);
  return (tenths / 10n).toString() + "." + (tenths % 10n).toString() + "%";
}
