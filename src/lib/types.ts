export interface Totals {
  input: string;
  cached: string;
  output: string;
  reasoning: string;
  total: string;
}
export interface Session {
  id: string;
  source_id: string;
  project: string;
  project_id: string;
  models: string[];
  last_at: string;
  totals: Totals;
  uncertain: number;
}
export interface Group {
  name: string;
  totals: Totals;
}
export interface Event {
  epoch: number;
  session: string;
  at: string;
  model: string;
  turn: string | null;
  quality: string;
  delta: Totals | null;
}
export interface Turn {
  session: string;
  id: string;
  totals: Totals;
  uncertain: number;
}
export type Grain = "minute" | "hour" | "day";
export interface Candle {
  open: string;
  close: string;
  low: string;
  high: string;
  samples: number;
}
export interface ActivityData {
  days: { date: string; weekday: number; total: string; level: number }[];
  active_days: number;
  total: string;
  peak: string;
}
export interface Dashboard {
  estimate: Estimate;
  totals: Totals;
  sessions: Session[];
  events: Event[];
  timeline: Group[];
  candles: (Candle | null)[];
  activity: ActivityData;
  interval_seconds: number;
  requested_grain: Grain;
  models: Group[];
  projects: Group[];
  turns: Turn[];
  turns_truncated: boolean;
  carried_baseline: Totals;
  partial_histories: number;
  uncertain: number;
  snapshots: number;
  cache_percent: number | null;
  active_sessions: number;
  providers: {
    id: string;
    name: string;
    status: string;
    capabilities: Record<string, boolean>;
  }[];
}
export interface Window {
  used_percent: number;
  remaining_percent: number;
  duration_mins: number | null;
  resets_at: number | null;
}
export interface Quota {
  status: string;
  collected_at: string | null;
  message: string | null;
  data: {
    buckets: {
      id: string;
      name: string | null;
      plan: string | null;
      primary: Window | null;
      secondary: Window | null;
      credits: {
        balance: string | null;
        unlimited: boolean;
        hasCredits: boolean;
      } | null;
      spend_control_reached: boolean | null;
    }[];
    ordinary_usage_allowed: boolean | null;
    account: string | null;
  } | null;
}
export type CollectorStatus =
  "starting" | "synced" | "degraded" | "missing" | "error";
export interface Health {
  database_path?: string;
  last_error?: string | null;
  test_mode: boolean;
  collector: {
    status: CollectorStatus;
    last_attempt: string | null;
    last_success: string | null;
    next_retry: string | null;
    consecutive_failures: number;
    recoveries: number;
  };
  report: {
    discovered: number;
    changed: number;
    inserted: number;
    parse_errors: number;
    source_available: boolean;
    io_errors: number;
  };
  quota: Quota;
}
export interface Filter {
  session?: string;
  grain?: Grain;
  from?: string;
  to?: string;
  project?: string;
  model?: string;
}

export interface Fx {
  enabled: boolean;
  status: string;
  collected_at: string | null;
  source: string;
  rates: {
    currency: string;
    date: string;
    rate: string;
    providers: string[];
  }[];
  message: string | null;
}
export interface Estimate {
  usd: string | null;
  hkd: string | null;
  cny: string | null;
  covered_tokens: string;
  uncovered_tokens: string;
  catalog: {
    id: string;
    verified_at: string;
    scenario: string;
    prices: {
      model: string;
      source: string;
      input_nano_usd_per_token: number;
      cached_nano_usd_per_token: number;
      output_nano_usd_per_token: number;
    }[];
  };
  models: {
    model: string;
    usd: string | null;
    tokens: string;
    source: string | null;
  }[];
  fx: Fx;
}
