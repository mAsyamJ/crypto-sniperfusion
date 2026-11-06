/** API DTOs — mirrors of the Rust envelope (crates/sniper-dashboard-api).
 * Money is decimal strings end-to-end. */

export interface Envelope<T> {
  as_of: string;
  data_status: string;
  data_status_reason: string | null;
  payload: T;
}

export interface RunEntry {
  id: string;
  mode: 'PAPER' | 'REAL_MONEY';
  engine: string;
  evidence_class: string;
  proof_state: string;
  completeness: string;
  verdict: string;
  /** Per-run scalars (all-runs overview); rows are never summed. */
  net_pnl_lamports: string;
  ending_nav_lamports: string;
  external_deposits_lamports: string;
  open_positions: number;
  /** False = observation receipt without an S00 row: comparison views are
   * voided by design (SOUL Law 5) and must be labeled, not hidden. */
  has_s00_baseline: boolean;
}

export interface RunDetail extends RunEntry {
  artifact_sha256: string;
  dataset_sha256: string;
  dataset_sha_is_hash: boolean;
  limitations: string[];
  oracle_audit: 'PASS' | 'FAIL' | 'UNAVAILABLE';
}

export interface ComparisonRow {
  strategy: string;
  net_pnl_lamports: string;
  incremental_net_edge_vs_no_trade: string;
  max_drawdown_lamports: string;
  open_positions: number;
  status: string;
}

export interface PnlSummary {
  run_id: string;
  engine: string;
  proof_state: string;
  currency: string;
  starting_nav_lamports: string;
  external_deposits_lamports: string;
  external_withdrawals_lamports: string;
  ending_nav_lamports: string;
  realized_pnl_lamports: string;
  unrealized_pnl_lamports: string;
  net_pnl_lamports: string;
  max_drawdown_lamports: string;
  recoverable_rent_lamports: string;
  open_inventory_cost_basis_lamports: string;
  open_positions: number;
  unexitable_exits: number;
  unknown_at_risk_lamports: string;
  deposits_are_not_pnl: boolean;
  fees_by_kind_lamports: Record<string, string>;
  comparison: ComparisonRow[];
  verdict: string;
}

export interface CalendarDay {
  day: string;
  realized_net_pnl_lamports: string;
  closed_count: number;
}

export interface CalendarPayload {
  run_id: string;
  strategy: string;
  tz: string;
  tz_label: string;
  scope: string;
  scope_note: string;
  days: CalendarDay[];
  closed_timestamped: number;
  untimestamped_count: number;
  open_positions: number;
  reconciled_total_lamports: string;
  closed_total_lamports: string;
  comparison_row_net_pnl_lamports: string | null;
  reconciles_to_comparison: boolean | null;
  nav_series: string;
  daily_returns: string;
}

export interface PortfolioPayload {
  run_id: string;
  ending_nav_lamports: string;
  open_positions: number;
  open_inventory_cost_basis_lamports: string;
  open_inventory_note: string;
  recoverable_rent_lamports: string;
  unknown_at_risk_lamports: string;
  unexitable_exits: number;
  open_entries: Array<{
    strategy: string;
    mint: string;
    entry_at_ms: number | null;
    entry_effective_lamports: string | null;
    tokens: number | null;
  }>;
  nav_series: string;
  mark_model: string;
  usd_value: string;
}

export interface TradeRow {
  ord: number;
  strategy: string;
  kind: string;
  mint: string;
  slot: number | null;
  at_ms: number | null;
  net_pnl_lamports: string | null;
  open: boolean;
}

export interface StatusPayload {
  runtime_state: string;
  runtime_note: string;
  runs: number;
  quarantined_artifacts: number;
  coverage_windows: number;
}

export interface StrategyStanding {
  strategy: string;
  latest_net_pnl_lamports: string;
  latest_run_id: string;
  latest_status: string;
  appearances: number;
}
