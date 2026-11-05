/** Page 1 · Command Center: runtime truth, mode banner, run verdict vs
 * S00, freshness, blocked/stale roll-up. Answers the north-star questions
 * in one screen (DESIGN.md §1). */
import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';
import { useMode } from '../state/ModeContext';
import { MODE_LABEL } from '../mode';
import { lamportsSigned, pnlClass, signGlyph } from '../fmt';
import { Card, CardTitle, Chip, EmptyState, ErrorState, KpiCard, LiveDisabledBanner, LoadingState, PaperBanner, StateChip } from '../components/ui';

export default function CommandCenter() {
  const { mode, runId } = useMode();
  const status = useQuery({ queryKey: ['status'], queryFn: api.status });
  const runs = useQuery({ queryKey: ['runs', mode], queryFn: () => api.runs(mode) });
  const summary = useQuery({
    queryKey: ['summary', mode, runId],
    queryFn: () => api.pnlSummary(mode, runId!),
    enabled: !!runId,
  });

  return (
    <div className="mx-auto max-w-6xl space-y-4">
      {mode === 'PAPER' ? <PaperBanner /> : <LiveDisabledBanner />}

      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <Card>
          <div className="text-[11px] uppercase tracking-wider text-muted">Runtime</div>
          <div className="mt-1 flex items-center gap-2">
            <span aria-hidden="true" className="inline-block h-2.5 w-2.5 rounded-full border-2 border-muted" />
            <span className="font-semibold" data-testid="runtime-state">
              {status.data?.payload.runtime_state ?? (status.isLoading ? '…' : 'UNKNOWN')}
            </span>
          </div>
          <div className="mt-1 text-[11px] text-muted">
            {status.data?.payload.runtime_note ?? 'no heartbeat producer exists (DASH-04)'}
          </div>
        </Card>
        <KpiCard
          label={`${MODE_LABEL[mode]} runs`}
          value={status.isLoading ? '…' : String(mode === 'REAL_MONEY' ? (runs.data?.payload.length ?? 0) : runs.data?.payload.length ?? 0)}
          sub={`scope ${mode} · historical receipts never prove liveness`}
        />
        <KpiCard
          label="Coverage windows"
          value={status.data ? String(status.data.payload.coverage_windows) : '…'}
          sub="block-verified G0 captures (all scopes)"
        />
        <Card>
          <div className="text-[11px] uppercase tracking-wider text-muted">Trading</div>
          <div className="mt-1"><Chip tone="muted">LIVE TRADING DISABLED</Chip></div>
          <div className="mt-1 text-[11px] text-muted">read-only mirror · no signing, no broadcasting</div>
        </Card>
      </div>

      <Card>
        <CardTitle right={runs.data ? <Chip tone="accent">{mode}</Chip> : undefined}>
          Verdict vs S00 NO_TRADE {runId ? `· run ${runId}` : ''}
        </CardTitle>
        {!runId && (runs.data?.payload.length ?? 0) > 0 && (
          <EmptyState what="selected run" hint="Pick a run in the header selector to scope every page." />
        )}
        {(runs.data?.payload.length ?? 0) === 0 && !runs.isLoading && (
          mode === 'REAL_MONEY' ? (
            <div className="hatched rounded border border-border p-4 text-center text-muted" role="note">
              No LIVE_CONFIRMED runs exist. Real Money stays in this disabled state until the
              SniperFusion execution system independently confirms funded activity.
            </div>
          ) : (
            <EmptyState what="paper runs" hint="Run any SniperFusion research/paper command and rebuild the projection." />
          )
        )}
        {runs.isLoading && <LoadingState label="runs" />}
        {runs.isError && <ErrorState message={(runs.error as Error).message} />}
        {summary.data && (
          <div className="space-y-3">
            <div className="flex flex-wrap items-center gap-2">
              <StateChip status={summary.data.data_status} reason={summary.data.data_status_reason} />
              <Chip tone="muted" title="sealed on the artifact">verdict: {summary.data.payload.verdict}</Chip>
              <Chip tone="muted">proof: {summary.data.payload.proof_state}</Chip>
              <Chip tone="muted">{summary.data.payload.engine}</Chip>
            </div>
            <table className="w-full text-[12px]">
              <thead>
                <tr>
                  <th scope="col" className="border-b border-border px-2 py-1.5 text-left text-[11px] uppercase tracking-wider text-muted">Strategy</th>
                  <th scope="col" className="border-b border-border px-2 py-1.5 text-right text-[11px] uppercase tracking-wider text-muted">Net PnL (lamports)</th>
                  <th scope="col" className="border-b border-border px-2 py-1.5 text-right text-[11px] uppercase tracking-wider text-muted">vs S00</th>
                  <th scope="col" className="border-b border-border px-2 py-1.5 text-left text-[11px] uppercase tracking-wider text-muted">Status</th>
                </tr>
              </thead>
              <tbody>
                {summary.data.payload.comparison.map((c) => (
                  <tr key={c.strategy} className={c.strategy === 'S00' ? 'bg-surface-2/60' : ''}>
                    <td className="border-b border-border/60 px-2 py-1.5 font-data">{c.strategy}</td>
                    <td className={`border-b border-border/60 px-2 py-1.5 text-right font-data ${pnlClass(c.net_pnl_lamports)}`}>
                      {signGlyph(c.net_pnl_lamports)} {lamportsSigned(c.net_pnl_lamports)}
                    </td>
                    <td className={`border-b border-border/60 px-2 py-1.5 text-right font-data ${pnlClass(c.incremental_net_edge_vs_no_trade)}`}>
                      {signGlyph(c.incremental_net_edge_vs_no_trade)} {lamportsSigned(c.incremental_net_edge_vs_no_trade)}
                    </td>
                    <td className="border-b border-border/60 px-2 py-1.5"><Chip tone="muted">{c.status}</Chip></td>
                  </tr>
                ))}
              </tbody>
            </table>
            {summary.data.payload.deposits_are_not_pnl && (
              <p className="text-[11px] text-muted">deposits are not PnL · external flows are shown separately on Portfolio</p>
            )}
          </div>
        )}
        {summary.isError && <ErrorState message={(summary.error as Error).message} />}
      </Card>

      <Card>
        <CardTitle>Freshness &amp; integrity</CardTitle>
        <div className="grid gap-2 text-[12px] text-muted sm:grid-cols-2">
          <div>Projection built at: <span className="font-data text-fg">{status.data?.as_of ?? '…'}</span> (projection time, not wall clock)</div>
          <div>Quarantined artifacts: <span className="font-data text-fg">{status.data?.payload.quarantined_artifacts ?? '…'}</span> (hash-mismatch, never served)</div>
          <div>S00 NO_TRADE is the standing champion until a challenger passes gates G-A..G-G (SOUL Law 5)</div>
          <div>Every number on this page is copied from a hash-sealed SniperFusion receipt</div>
        </div>
      </Card>
    </div>
  );
}
