/** Fetch layer: same-origin only (Vite proxy -> Rust API on 127.0.0.1).
 * Every run-scoped request carries the active mode so the API boundary
 * enforces isolation (T-MODE2). No other network calls exist in the app. */
import type { Envelope, RunEntry, RunDetail, PnlSummary, CalendarPayload, PortfolioPayload, TradeRow, StatusPayload, StrategyStanding } from './types';
import type { Mode } from '../mode';

const BASE = '/api/v1';

async function get<T>(path: string): Promise<Envelope<T>> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { Accept: 'application/json' },
  });
  if (!res.ok && res.status !== 200) {
    // Transport-level errors (404/400/403/405/501) surface as typed errors.
    let body: unknown = null;
    try { body = await res.json(); } catch { /* empty body */ }
    const reason =
      (body && typeof body === 'object' && 'error' in body && typeof (body as { error: unknown }).error === 'string')
        ? (body as { error: string }).error
        : `HTTP ${res.status}`;
    const err = new Error(reason) as Error & { status: number };
    err.status = res.status;
    throw err;
  }
  return (await res.json()) as Envelope<T>;
}

function withMode(params: URLSearchParams, mode: Mode): URLSearchParams {
  params.set('mode', mode);
  return params;
}

export const api = {
  status: () => get<StatusPayload>('/status'),
  runs: (mode: Mode) => get<RunEntry[]>(`/runs?${withMode(new URLSearchParams(), mode)}`),
  runDetail: (mode: Mode, id: string) => get<RunDetail>(`/runs/${id}?mode=${mode}`),
  pnlSummary: (mode: Mode, id: string) =>
    get<PnlSummary>(`/pnl/summary?${withMode(new URLSearchParams({ run: id }), mode)}`),
  calendar: (mode: Mode, id: string, strategy: string, tz = 'UTC') =>
    get<CalendarPayload>(`/pnl/calendar?${withMode(new URLSearchParams({ run: id, strategy, tz }), mode)}`),
  portfolio: (mode: Mode, id: string) =>
    get<PortfolioPayload>(`/portfolio?${withMode(new URLSearchParams({ run: id }), mode)}`),
  trades: (mode: Mode, id: string, cursor = 0, limit = 200) =>
    get<{ run_id: string; rows: TradeRow[]; next_cursor: number | null }>(
      `/trades?${withMode(new URLSearchParams({ run: id, cursor: String(cursor), limit: String(limit) }), mode)}`),
  strategies: (mode: Mode) => get<{ strategies: StrategyStanding[] }>(`/strategies?${withMode(new URLSearchParams(), mode)}`),
};
