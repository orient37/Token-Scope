import type { Group } from "./types";

// Only visualization ratios use Number; counts and ordering remain exact.
export function distribution(groups: Group[]) {
  const sorted = [...groups].sort((a, b) => {
    const x = BigInt(a.totals.total),
      y = BigInt(b.totals.total);
    return x === y ? a.name.localeCompare(b.name) : x > y ? -1 : 1;
  });
  const total = sorted.reduce((n, group) => n + BigInt(group.totals.total), 0n);
  const max = BigInt(sorted[0]?.totals.total ?? "0");
  return {
    total: total.toString(),
    rows: sorted.map((group) => {
      const value = BigInt(group.totals.total);
      return {
        ...group,
        share: total ? Number((value * 10000n) / total) / 100 : 0,
        width: max ? Number((value * 10000n) / max) / 100 : 0,
      };
    }),
  };
}
