/** Labeled all-runs overview: every run in the active scope side by side.
 * This is a SCAN VIEW, not an aggregation: each row is that run's own
 * sealed numbers; cross-run sums are prohibited (runs are independent
 * arms — REQUIREMENTS §6.4). Rows without an S00 baseline are observation
 * receipts and are labeled as such (their comparison views are voided by
 * design, SOUL Law 5). */
import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';
import { useMode } from '../state/ModeContext';
import { MODE_LABEL } from '../mode';
import { lamports, lamportsSigned, pnlClass, signGlyph } from '../fmt';
import { Card, CardTitle, Chip, ErrorState, LoadingState, Td, Th } from './ui';

export default function AllRunsCard({ onSelect }: { onSelect?: () => void }) {
  const { mode, runId, setRunId } = useMode();
  const q = useQuery({ queryKey: ['runs', mode], queryFn: () => api.runs(mode) });
  const runs = q.data?.payload ?? [];

  return (
    <Card>
      <CardTitle right={<Chip tone="accent">{MODE_LABEL[mode]}</Chip>}>
        All runs in scope ({runs.length})
      </CardTitle>
      <p className="mb-2 text-[11px] text-muted">
        Combined scan view: each row is one sealed run with its own values — runs are
        independent arms and are <span className="text-fg">never summed</span>. Rows lacking
        an S00 baseline are observation receipts (comparison views voided by design).
      </p>
      {q.isLoading && <LoadingState label="runs" />}
      {q.isError && <ErrorState message={(q.error as Error).message} />}
      {q.data && runs.length === 0 && (
        <div className="hatched rounded border border-border p-4 text-center text-muted">
          No runs in this scope{mode === 'REAL_MONEY' ? ' — LIVE TRADING DISABLED / NO VERIFIED REAL TRADES' : ''}.
        </div>
      )}
      {runs.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full min-w-[760px] text-[12px]" aria-label="All runs in scope">
            <thead>
              <tr>
                <Th>Run</Th>
                <Th>Engine</Th>
                <Th>Proof</Th>
                <Th>Verdict</Th>
                <Th numeric>Net PnL (lam)</Th>
                <Th numeric>Ending NAV (lam)</Th>
                <Th numeric>Deposits (lam)</Th>
                <Th numeric>Open</Th>
                <Th>Baseline</Th>
                <Th />
              </tr>
            </thead>
            <tbody>
              {runs.map((r) => (
                <tr key={r.id} className={r.id === runId ? 'bg-surface-2/60' : ''}>
                  <Td className="font-data">{r.id}</Td>
                  <Td className="font-data">{r.engine}</Td>
                  <Td><Chip tone="muted">{r.proof_state}</Chip></Td>
                  <Td>{r.verdict}</Td>
                  <Td numeric className={pnlClass(r.net_pnl_lamports)}>
                    {signGlyph(r.net_pnl_lamports)} {lamportsSigned(r.net_pnl_lamports)}
                  </Td>
                  <Td numeric>{lamports(r.ending_nav_lamports)}</Td>
                  <Td numeric>{lamports(r.external_deposits_lamports)}</Td>
                  <Td numeric>{r.open_positions}</Td>
                  <Td>
                    {r.has_s00_baseline
                      ? <Chip tone="ok">S00 present</Chip>
                      : <Chip tone="warn" title="observation receipt: comparison rows have no S00 baseline; PnL comparison views are voided for this run (SOUL Law 5)">no S00 · observation</Chip>}
                  </Td>
                  <Td>
                    <button
                      onClick={() => { setRunId(r.id); onSelect?.(); }}
                      className="cursor-pointer rounded border border-accent/60 px-2 py-0.5 text-[11px] font-semibold text-accent hover:bg-accent hover:text-onaccent"
                    >
                      {r.id === runId ? 'selected' : 'select'}
                    </button>
                  </Td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </Card>
  );
}
