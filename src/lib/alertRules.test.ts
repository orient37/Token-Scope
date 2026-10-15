import { expect, test } from "vitest";
import { normalizeThreshold } from "./alertRules";
test("threshold accepts exact integers and grouped pasted values", () => {
  expect(normalizeThreshold(" 1,250,000 ")).toBe("1250000");
  expect(normalizeThreshold("000123")).toBe("123");
  expect(normalizeThreshold("18446744073709551615")).toBe(
    "18446744073709551615",
  );
  for (const value of [
    "",
    "0",
    "-1",
    "1.5",
    "1e6",
    "12,34",
    "18446744073709551616",
  ])
    expect(normalizeThreshold(value)).toBeNull();
});
