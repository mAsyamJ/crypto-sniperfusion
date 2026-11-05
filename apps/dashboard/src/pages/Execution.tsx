/** Page 5 · Execution: mode-scoped trade history with record kinds,
 * timestamps where the producer emitted them, refusals/failed landings
 * visible (fees and failures are the strategy's shadow), and run-detail
 * evidence (limitations verbatim, oracle audit chip, artifact hash). */
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';
import { useMode } from '../state/ModeContext';
import { lamportsSigned, msToStamp, pnlClass, signGlyph } from '../fmt';
import { Card, CardTitle, Chip, EmptyState, ErrorState, LiveDisabledBanner, LoadingState, PaperBanner, StateChip, Td, Th } from '../components/ui';

const KIND_FILTERS = ['ALL', 'FOLLOWER_ENTRY', 'BUY_FILL', 'SELL_FILL', 'REJECTED', 'FAILED_LANDING', 'EXIT_BLOCKED', 'OBSERVED_TRADE'] as const;

export default function Execution() {
  const { mode, runId } = useMode();
  const [kindFilter, setKindFilter] = useState<string>('ALL');
  const q = useQuery({
    queryKey: ['trades', mode, runId],
    queryFn: () => api.trades(mode, runId!),
    enabled: !!runId,
  });
  const detail = useQuery({
    queryKey: ['runDetail', mode, runId],
    queryFn: () => api.runDetail(mode, runId!),
    enabled: !!runId,
  });

  const rows = useMemo(
    () => (q.data?.payload.rows ?? []).filter((r) => kindFilter === 'ALL' || r.kind === kindFilter),
    [q.data, kindFilter],
  );

  return (
    <div className="mx-auto max-w-6xl space-y-4">
      {mode === 'PAPER' ? <PaperBanner /> : <LiveDisabledBanner />}
      {!runId && <EmptyState what="selected run" hint="Pick a run to inspect its execution records." />}

      {runId && detail.data?.data_status === 'OK' && (
        <Card>
          <CardTitle right={<StateChip status={detail.data.data_status} />}>Run evidence · {runId}</CardTitle>
          <div className="flex flex-wrap gap-2 text-[11px]">
            <Chip tone="accent">{detail.data.payload.mode}</Chip>
            <Chip tone="muted">proof: {detail.data.payload.proof_state}</Chip>
            <Chip tone="muted">{detail.data.payload.evidence_class}</Chip>
            <Chip tone="muted">completeness: {detail.data.payload.completeness}</Chip>
            <Chip tone={detail.data.payload.oracle_audit === 'PASS' ? 'ok' : detail.data.payload.oracle_audit === 'FAIL' ? 'neg' : 'hatched'}>
              oracle audit: {detail.data.payload.oracle_audit}
            </Chip>
            <Chip tone="muted" title={detail.data.payload.dataset_sha256}>
              dataset sha: {detail.data.payload.dataset_sha_is_hash ? detail.data.payload.dataset_sha256.slice(0, 12) + '…' : 'label'}
            </Chip>
          </div>
          {detail.data.payload.limitations.length > 0 && (
            <ul className="mt-2 list-inside list-disc space-y-0.5 text-[11px] text-muted">
              {detail.data.payload.limitations.map((l, i) => <li key={i}>{l}</li>)}
            </ul>
          )}
          <div className="mt-2 font-data text-[10px] break-all text-muted">artifact sha256: {detail.data.payload.artifact_sha256}</div>
        </Card>
      )}

      {runId && (
        <Card>
          <CardTitle
            right={
              <label className="text-[11px] text-muted">
                kind{' '}
                <select
                  value={kindFilter}
                  onChange={(e) => setKindFilter(e.target.value)}
                  className="cursor-pointer rounded border border-border bg-surface px-1.5 py-1 font-data text-[12px] text-fg"
                >
                  {KIND_FILTERS.map((k) => <option key={k}>{k}</option>)}
                </select>
              </label>
            }
          >
            Trade &amp; order records {rows.length > 0 ? `(${rows.length})` : ''}
          </CardTitle>
          {q.isLoading && <LoadingState label="trades" />}
          {q.isError && <ErrorState message={(q.error as Error).message} />}
          {q.data?.data_status === 'OK' && rows.length === 0 && (
            <EmptyState what="records" hint="No records match this filter in the selected run (empty bundles are common for wallet-pnl engines)." />
          )}
          {q.data?.data_status === 'OK' && rows.length > 0 && (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[720px] text-[12px]">
                <thead>
                  <tr>
                    <Th>#</Th>
                    <Th>Kind</Th>
                    <Th>Strategy</Th>
                    <Th>Mint</Th>
                    <Th numeric>Slot</Th>
                    <Th>Time (UTC)</Th>
                    <Th numeric>Net PnL (lam)</Th>
                    <Th>State</Th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((r) => (
                    <tr key={r.ord}>
                      <Td numeric>{r.ord}</Td>
                      <Td><Chip tone={r.kind === 'REJECTED' || r.kind === 'FAILED_LANDING' || r.kind === 'EXIT_BLOCKED' ? 'warn' : 'muted'}>{r.kind}</Chip></Td>
                      <Td className="font-data">{r.strategy}</Td>
                      <Td className="font-data">{r.mint}</Td>
                      <Td numeric>{r.slot ?? 'n/a'}</Td>
                      <Td className="font-data">{r.at_ms ? msToStamp(r.at_ms) : 'no timestamp'}</Td>
                      <Td numeric className={r.net_pnl_lamports ? pnlClass(r.net_pnl_lamports) : ''}>
                        {r.net_pnl_lamports ? `${signGlyph(r.net_pnl_lamports)} ${lamportsSigned(r.net_pnl_lamports)}` : 'n/a'}
                      </Td>
                      <Td>{r.open ? <Chip tone="warn">open</Chip> : <Chip tone="muted">closed</Chip>}</Td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
          <p className="mt-2 text-[11px] text-muted">
            Rejections and failed landings are first-class records (SOUL Law 3); research fills carry slots only,
            follower entries carry producer millisecond timestamps.
          </p>
        </Card>
      )}
    </div>
  );
}
