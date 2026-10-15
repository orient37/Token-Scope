import { describe, expect, it } from "vitest";
import { trendPeak } from "./trendPeak";
import type { Group, Candle } from "./types";
const groups = (...values: string[]): Group[] =>
  values.map((total, i) => ({
    name: `2026-10-11T00:0${i}:00Z`,
    totals: { input: total, cached: "0", output: "0", reasoning: "0", total },
  }));
const candle = (high: string): Candle => ({
  open: "1",
  close: "1",
  low: "1",
  high,
  samples: 2,
});
describe("trend peak annotation", () => {
  it("locates the exact total in line and bar, preserving first ties", () => {
    for (const mode of ["line", "bar"] as const) {
      expect(
        trendPeak(
          groups(
            "4",
            "9007199254740992",
            "9007199254740993",
            "9007199254740993",
          ),
          [],
          mode,
        ),
      ).toEqual({
        index: 2,
        value: "9007199254740993",
        at: "2026-10-11T00:02:00Z",
      });
    }
  });
  it("marks highest individual increment in K line rather than bucket total", () => {
    expect(
      trendPeak(
        groups("1000", "100", "5000"),
        [candle("10"), candle("80"), null],
        "candlestick",
      )?.index,
    ).toBe(1);
  });
  it("does not create peaks for empty, zero or absent candles", () => {
    expect(trendPeak([], [], "line")).toBeNull();
    expect(trendPeak(groups("0"), [], "bar")).toBeNull();
    expect(trendPeak(groups("100"), [], "candlestick")).toBeNull();
  });
  it("updates when the displayed filtered aggregates change", () => {
    expect(trendPeak(groups("9", "2"), [], "line")?.index).toBe(0);
    expect(trendPeak(groups("1", "2"), [], "line")?.index).toBe(1);
  });
});
