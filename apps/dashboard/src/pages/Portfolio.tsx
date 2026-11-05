/** Page 4 · Portfolio: mode-scoped balances/positions with honest
 * at-risk framing; NAV series / marks / USD are UNAVAILABLE by
 * construction until producers exist. */
import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';
import { useMode } from '../state/ModeContext';
import { lamports, lamportsToSol, msToStamp } from '../fmt';
import { Card, CardTitle, Chip, EmptyState, ErrorState, KpiCard, LiveDisabledBanner, LoadingState, PaperBanner, StateChip, UnavailablePanel } from '../components/ui';

export default function Portfolio() {
  const { mode, runId } = useMode();
  const q = useQuery({
    queryKey: ['portfolio', mode, runId],
    queryFn: () => api.portfolio(mode, runId!),
    enabled: !!runId,
  });

  return (
    <div className="mx-auto max-w-6xl space-y-4">
      {mode === 'PAPER' ? <PaperBanner /> : <LiveDisabledBanner />}
      {!runId && <EmptyState what="selected run" hint="Pick a run; portfolio views are per-run and never summed across runs." />}

      {runId && q.isLoading && <LoadingState label="portfolio" />}
      {runId && q.isError && <ErrorState message={(q.error as Error).message} />}

      {q.data?.data_status === 'OK' && (
        <>
          <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
            <KpiCard label={`Ending NAV (${mode.toLowerCase()} scope)`} value={`${lamports(q.data.payload.ending_nav_lamports)} lam`} sub={`${lamportsToSol(q.data.payload.ending_nav_lamports)} SOL (unit display, 1e9)`} />
            <KpiCard label="Open positions" value={String(q.data.payload.open_positions)} sub={`cost basis ${lamports(q.data.payload.open_inventory_cost_basis_lamports)} lam`} tone="muted" />
            <KpiCard label="Unknown at risk" value={lamports(q.data.payload.unknown_at_risk_lamports)} sub="unresolved holds, excluded from NAV" tone="muted" />
            <KpiCard label="Recoverable rent" value={lamports(q.data.payload.recoverable_rent_lamports)} sub="closed accounts, claimable" tone="muted" />
          </div>

          <Card>
            <CardTitle right={<StateChip status={q.data.data_status} />}>Valuation availability</CardTitle>
            <div className="grid gap-2 sm:grid-cols-3">
              <div className="rounded border border-border p-2 text-[12px]">
                <div className="text-muted">NAV series</div>
                <UnavailablePanel what="NAV series" reason="no NAV/mark producer exists (NEEDS_NEW_TELEMETRY); only run-level scalars are emitted" />
              </div>
              <div className="rounded border border-border p-2 text-[12px]">
                <div className="text-muted">Mark model</div>
                <UnavailablePanel what="Mark model" reason="exit-mark valuation lives inside PF1 only; no series is published" />
              </div>
              <div className="rounded border border-border p-2 text-[12px]">
                <div className="text-muted">USD value</div>
                <UnavailablePanel what="USD" reason="no FX/price source exists in SniperFusion; USD display is prohibited by construction" />
              </div>
            </div>
          </Card>

          <Card>
            <CardTitle>Open positions {q.data.payload.open_entries.length > 0 ? `(${q.data.payload.open_entries.length})` : ''}</CardTitle>
            {q.data.payload.open_entries.length === 0 ? (
              <EmptyState what="open positions" hint="No open positions recorded in this run's sealed trade records." />
            ) : (
              <table className="w-full text-[12px]">
                <thead>
                  <tr>
                    <th scope="col" className="border-b border-border px-2 py-1.5 text-left text-[11px] uppercase tracking-wider text-muted">Strategy</th>
                    <th scope="col" className="border-b border-border px-2 py-1.5 text-left text-[11px] uppercase tracking-wider text-muted">Mint</th>
                    <th scope="col" className="border-b border-border px-2 py-1.5 text-right text-[11px] uppercase tracking-wider text-muted">Entry (lam)</th>
                    <th scope="col" className="border-b border-border px-2 py-1.5 text-right text-[11px] uppercase tracking-wider text-muted">Tokens</th>
                    <th scope="col" className="border-b border-border px-2 py-1.5 text-left text-[11px] uppercase tracking-wider text-muted">Entered</th>
                  </tr>
                </thead>
                <tbody>
                  {q.data.payload.open_entries.map((e, i) => (
                    <tr key={i}>
                      <td className="border-b border-border/60 px-2 py-1.5 font-data">{e.strategy}</td>
                      <td className="border-b border-border/60 px-2 py-1.5 font-data">{e.mint}</td>
                      <td className="border-b border-border/60 px-2 py-1.5 text-right font-data">{e.entry_effective_lamports ? lamports(e.entry_effective_lamports) : 'n/a'}</td>
                      <td className="border-b border-border/60 px-2 py-1.5 text-right font-data">{e.tokens ?? 'n/a'}</td>
                      <td className="border-b border-border/60 px-2 py-1.5 font-data">{e.entry_at_ms ? msToStamp(e.entry_at_ms) : 'n/a'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
            <p className="mt-2 text-[11px] text-muted">
              {q.data.payload.open_inventory_note} · unexitable exits: {q.data.payload.unexitable_exits}
            </p>
          </Card>

          <Card>
            <CardTitle>Scope discipline</CardTitle>
            <ul className="list-inside list-disc space-y-1 text-[12px] text-muted">
              <li>This portfolio is the selected run only ({runId}); runs are independent arms and are never added together.</li>
              <li>Deposits/withdrawals are flows, not PnL: they appear as their own lines in the PnL view.</li>
              {mode === 'PAPER' && <li><Chip tone="warn">PAPER</Chip> simulated capital; the same run can never appear under Real Money (its proof state is sealed {`SYNTHETIC`}).</li>}
              {mode === 'REAL_MONEY' && <li>Real-money scope requires LIVE_CONFIRMED proof on every value shown.</li>}
            </ul>
          </Card>
        </>
      )}
    </div>
  );
}
