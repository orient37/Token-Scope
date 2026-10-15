import { expect, it } from "vitest";
import { money, coverage } from "./money";
it("rounds once without losing integer precision", () => {
  expect(money("9007199254740993.125000000", "USD")).toBe(
    "USD 9,007,199,254,740,993.13",
  );
  expect(money(null, "HKD")).toBe("—");
  expect(money("0.004999999", "CNY")).toBe("CNY 0.00");
});
it("reports exact partial coverage", () => {
  expect(coverage("1", "2")).toBe("33.3%");
  expect(coverage("0", "0")).toBe("—");
});
