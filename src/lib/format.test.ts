import { describe, it, expect } from "vitest";
import { exact, compact, resetLabel, stale } from "./format";
describe("accurate display", () => {
  it("retains integers above JS precision", () =>
    expect(exact("9007199254740993").replaceAll(",", "")).toBe(
      "9007199254740993",
    ));
  it("compacts without Number conversion", () => {
    expect(compact("1234567")).toBe("1.23M");
    expect(compact("0")).toBe("0");
  });
  it("does not invent reset usage", () =>
    expect(resetLabel(1, 2000)).toContain("等待同步"));
  it("missing timestamps and stale snapshots", () => {
    expect(stale(null, 0)).toBe(true);
    expect(
      stale("2026-10-09T00:00:00Z", Date.parse("2026-10-09T00:06:00Z")),
    ).toBe(true);
  });
});
