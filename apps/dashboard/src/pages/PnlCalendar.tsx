/** Page 3 · PnL Calendar: month grid of daily realized PnL from the
 * producer-side calendar (PF1 extension). UNAVAILABLE cells are hatched,
 * never zero. Reconciliation footer is always visible. tz display-only. */
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';
import { useMode } from '../state/ModeContext';
import { lamportsSigned, pnlClass, signGlyph } from '../fmt';
import { Card, CardTitle, Chip, EmptyState, ErrorState, LiveDisabledBanner, LoadingState, PaperBanner, StateChip } from '../components/ui';

const TZ_OPTIONS = ['UTC', 'Asia/Jakarta'] as const;

function monthKey(day: string): string {
  return day.slice(0, 7);
}

export default function PnlCalendar() {
  const { mode, runId } = useMode();
  const [tz, setTz] = useState<string>('UTC');
  const [strategy, setStrategy] = useState<string>('');
  const [month, setMonth] = useState<string>('');

  const summary = useQuery({
    queryKey: ['summary', mode, runId],
    queryFn: () => api.pnlSummary(mode, runId!),
    enabled: !!runId,
  });
  const strategies = useMemo(
    () => (summary.data?.payload.comparison ?? []).map((c) => c.strategy),
    [summary.data],
  );
  const effectiveStrategy = strategy || strategies[0] || '';

  const cal = useQuery({
    queryKey: ['calendar', mode, runId, effectiveStrategy, tz],
    queryFn: () => api.calendar(mode, runId!, effectiveStrategy, tz),
    enabled: !!runId && !!effectiveStrategy,
  });

  const days = cal.data?.payload.days ?? [];
  const months = useMemo(() => [...new Set(days.map((d) => monthKey(d.day)))].sort(), [days]);
  const effectiveMonth = month || months[0] || '';
  const monthDays = days.filter((d) => monthKey(d.day) === effectiveMonth);

  // Month grid: weeks x Mon..Sun. Days without a bucket render UNAVAILABLE
  // (hatched) — never zero-filled (REQUIREMENTS §6.3 rule 5).
  const grid = useMemo(() => {
    if (!effectiveMonth) return [];
    const [y, m] = effectiveMonth.split('-').map(Number);
    const first = new Date(Date.UTC(y, m - 1, 1));
    const daysInMonth = new Date(Date.UTC(y, m, 0)).getUTCDate();
    const lead = (first.getUTCDay() + 6) % 7; // Monday-first
    const cells: Array<{ day: string | null; cell: (typeof days)[number] | null }> = [];
    for (let i = 0; i < lead; i++) cells.push({ day: null, cell: null });
    for (let d = 1; d <= daysInMonth; d++) {
      const day = `${effectiveMonth}-${String(d).padStart(2, '0')}`;
      cells.push({ day, cell: monthDays.find((x) => x.day === day) ?? null });
    }
    const weeks: typeof cells[] = [];
    for (let i = 0; i < cells.length; i += 7) weeks.push(cells.slice(i, i + 7));
    return weeks;
  }, [effectiveMonth, monthDays]);

  return (
    <div className="mx-auto max-w-6xl space-y-4">
      {mode === 'PAPER' ? <PaperBanner /> : <LiveDisabledBanner />}
      {!runId && <EmptyState what="selected run" hint="Pick a run; the calendar is per-run and per-strategy arm." />}

      {runId && (
        <Card>
          <CardTitle
            right={
              <div className="flex items-center gap-2">
                <label className="text-[11px] text-muted">
                  strategy{' '}
                  <select
                    value={effectiveStrategy}
                    onChange={(e) => setStrategy(e.target.value)}
                    className="cursor-pointer rounded border border-border bg-surface px-1.5 py-1 font-data text-[12px] text-fg"
                  >
                    {strategies.map((s) => <option key={s}>{s}</option>)}
                  </select>
                </label>
                <label className="text-[11px] text-muted">
                  tz{' '}
                  <select
                    value={tz}
                    onChange={(e) => setTz(e.target.value)}
                    className="cursor-pointer rounded border border-border bg-surface px-1.5 py-1 font-data text-[12px] text-fg"
                  >
                    {TZ_OPTIONS.map((z) => <option key={z}>{z}</option>)}
                  </select>
                </label>
                <Chip tone="accent">{cal.data?.payload.tz_label ?? ''}</Chip>
              </div>
            }
          >
            Daily realized PnL{effectiveStrategy ? ` · ${effectiveStrategy}` : ''}
          </CardTitle>

          {cal.isLoading && <LoadingState label="calendar" />}
          {cal.isError && <ErrorState message={(cal.error as Error).message} />}
          {cal.data && (
            <>
              <div className="mb-2 flex flex-wrap items-center gap-2 text-[11px] text-muted">
                <StateChip status={cal.data.data_status} reason={cal.data.data_status_reason} />
                <span>scope: <span className="text-fg">{cal.data.payload.scope}</span> ({cal.data.payload.scope_note})</span>
              </div>

              {cal.data.data_status !== 'OK' ? (
                <div className="hatched rounded border border-border p-6 text-center text-muted">
                  {cal.data.data_status_reason ?? 'calendar unavailable for this strategy/run'}
                </div>
              ) : (
                <>
                  <div className="mb-2 flex gap-1">
                    {months.map((m) => (
                      <button
                        key={m}
                        onClick={() => setMonth(m)}
                        className={`cursor-pointer rounded border px-2 py-1 font-data text-[11px] ${m === effectiveMonth ? 'border-accent/60 text-accent' : 'border-border text-muted hover:text-fg'}`}
                      >
                        {m}
                      </button>
                    ))}
                  </div>
                  <div className="overflow-x-auto">
                    <table aria-label="Daily realized PnL calendar" className="min-w-[560px] border-collapse text-[11px]">
                      <thead>
                        <tr>
                          {['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'].map((d) => (
                            <th key={d} scope="col" className="border border-border px-1 py-1 text-[10px] uppercase tracking-wider text-muted">{d}</th>
                          ))}
                        </tr>
                      </thead>
                      <tbody>
                        {grid.map((week, wi) => (
                          <tr key={wi}>
                            {week.map(({ day, cell }, di) => (
                              <td
                                key={di}
                                className={`h-11 w-20 border border-border p-0.5 align-top ${cell ? '' : 'hatched'}`}
                                title={day ? (cell ? `${day}: ${lamportsSigned(cell.realized_net_pnl_lamports)} lamports (${cell.closed_count} closed)` : `${day}: UNAVAILABLE (no closed positions bucketed)`) : ''}
                              >
                                {day && (
                                  <div className="flex h-full flex-col">
                                    <span className="text-[10px] text-muted">{Number(day.slice(8))}</span>
                                    {cell ? (
                                      <span className={`font-data text-[10px] leading-tight ${pnlClass(cell.realized_net_pnl_lamports)}`}>
                                        {signGlyph(cell.realized_net_pnl_lamports)}{lamportsSigned(cell.realized_net_pnl_lamports).replace(/^\+|^\u2212/, '')}
                                      </span>
                                    ) : (
                                      <span className="text-[10px] text-muted">n/a</span>
                                    )}
                                  </div>
                                )}
                              </td>
                            ))}
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>

                  <div className="mt-3 flex flex-wrap items-center gap-2 border-t border-border pt-2 text-[12px]">
                    <span className="text-muted">reconciliation:</span>
                    <span className={`font-data ${pnlClass(cal.data.payload.reconciled_total_lamports)}`}>
                      {signGlyph(cal.data.payload.reconciled_total_lamports)} {lamportsSigned(cal.data.payload.reconciled_total_lamports)}
                    </span>
                    <span className="text-muted">= Σ day buckets</span>
                    {cal.data.payload.comparison_row_net_pnl_lamports !== null && (
                      <>
                        <span className="text-muted">vs sealed comparison row</span>
                        <span className={`font-data ${pnlClass(cal.data.payload.comparison_row_net_pnl_lamports)}`}>
                          {lamportsSigned(cal.data.payload.comparison_row_net_pnl_lamports)}
                        </span>
                        {cal.data.payload.reconciles_to_comparison ? <Chip tone="ok">RECONCILED</Chip> : <Chip tone="neg">MISMATCH</Chip>}
                      </>
                    )}
                    {cal.data.payload.untimestamped_count > 0 && (
                      <Chip tone="warn" title="records lacking producer timestamps are excluded from buckets">
                        partial · {cal.data.payload.untimestamped_count} untimestamped
                      </Chip>
                    )}
                  </div>
                  <div className="mt-1 flex gap-4 text-[11px] text-muted">
                    <span>NAV series: {cal.data.payload.nav_series}</span>
                    <span>daily returns: {cal.data.payload.daily_returns}</span>
                  </div>
                </>
              )}
            </>
          )}
        </Card>
      )}
    </div>
  );
}
