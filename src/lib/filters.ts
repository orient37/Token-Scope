import type { Filter, Grain } from "./types";
export function usageFilter(
  period: string,
  project: string,
  model: string,
  grain: Grain,
  now = new Date(),
): Filter {
  const filter: Filter = { grain };
  if (period !== "all") {
    const start = new Date(now);
    if (period === "today") start.setHours(0, 0, 0, 0);
    else start.setTime(start.getTime() - Number(period) * 86400000);
    filter.from = start.toISOString();
  }
  if (project) filter.project = project;
  if (model) filter.model = model;
  return filter;
}
