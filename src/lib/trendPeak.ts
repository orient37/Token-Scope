import type { Candle, Group } from "./types";
export type TrendMode = "line" | "bar" | "candlestick";
// Only locate an annotation among Rust aggregates; never recalculate usage.
// BigInt keeps values above JS's safe integer range distinct. First tie wins.
export function trendPeak(
  groups: Group[],
  candles: (Candle | null)[],
  mode: TrendMode,
) {
  let peak: { index: number; value: string; at: string } | null = null;
  for (let index = 0; index < groups.length; index++) {
    const value =
      mode === "candlestick"
        ? candles[index]?.high
        : groups[index].totals.total;
    if (value === undefined || BigInt(value) <= 0n) continue;
    if (!peak || BigInt(value) > BigInt(peak.value))
      peak = { index, value, at: groups[index].name };
  }
  return peak;
}
