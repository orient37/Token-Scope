import { expect, test } from "vitest";
import { distribution } from "./distribution";
const group = (name: string, total: string) => ({
  name,
  totals: { input: total, cached: "0", output: "0", reasoning: "0", total },
});
test("distribution preserves large count sorting and source data", () => {
  const source = [
    group("a", "9007199254740992"),
    group("b", "9007199254740993"),
  ];
  const result = distribution(source);
  expect(result.total).toBe("18014398509481985");
  expect(result.rows.map((r) => r.name)).toEqual(["b", "a"]);
  expect(source[0].name).toBe("a");
  expect(result.rows[0].width).toBe(100);
});
test("empty and zero distributions have no fabricated share", () => {
  expect(distribution([])).toEqual({ total: "0", rows: [] });
  expect(distribution([group("empty", "0")]).rows[0].share).toBe(0);
});
