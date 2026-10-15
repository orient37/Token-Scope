import { describe, it, expect } from "vitest";
import { usageFilter } from "./filters";
describe("shared dashboard/export filters", () => {
  it("uses identical local midnight and rolling cutoffs", () => {
    const now = new Date("2026-10-09T12:34:56Z");
    const midnight = new Date(now);
    midnight.setHours(0, 0, 0, 0);
    expect(usageFilter("today", "P", "M", "minute", now)).toEqual({
      grain: "minute",
      project: "P",
      model: "M",
      from: midnight.toISOString(),
    });
    expect(usageFilter("7", "", "", "day", now).from).toBe(
      new Date(now.getTime() - 7 * 86400000).toISOString(),
    );
    expect(usageFilter("all", "", "", "hour", now)).toEqual({ grain: "hour" });
  });
});
