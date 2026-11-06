/** Page 2 · PnL Analytics: mode-scoped KPI strip, fee waterfall (Recharts),
 * strategy comparison table (TanStack Table with sorting). */
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
  type SortingState,
} from '@tanstack/react-table';
import { Bar, BarChart, CartesianGrid, Cell, ReferenceLine, XAxis, YAxis } from 'recharts';
import { api } from '../api/client';
import type { ComparisonRow } from '../api/types';
import { useMode } from '../state/ModeContext';
import { lamports, lamportsSigned, msToStamp, pnlClass, signGlyph } from '../fmt';
import { Card, CardTitle, Chip, EmptyState, ErrorState, KpiCard, LiveDisabledBanner, LoadingState, PaperBanner, StateChip, UnavailablePanel } from '../components/ui';
import AllRunsCard from '../components/AllRunsCard';

const col = createColumnHelper<ComparisonRow>();

export default function PnlAnalytics() {
  const { mode, runId } = useMode();
  const q = useQuery({
    queryKey: ['summary', mode, runId],
    queryFn: () => api.pnlSummary(mode, runId!),
    enabled: !!runId,
  });
  const [sorting, setSorting] = useState<SortingState>([]);

  const table = useReactTable({
    data: q.data?.payload?.comparison ?? [],
    columns: [
      col.accessor('strategy', { header: 'Strategy', cell: (i) => <span className="font-data">{i.getValue()}</span> }),
      col.accessor((r) => BigInt(r.net_pnl_lamports), {
        id: 'net',
        header: 'Net PnL (lamports)',
        cell: (i) => {
          const raw = i.row.original.net_pnl_lamports;
          return (
            <span className={`font-data ${pnlClass(raw)}`}>
              {signGlyph(raw)} {lamportsSigned(raw)}
            </span>
          );
        },
      }),
      col.accessor((r) => BigInt(r.incremental_net_edge_vs_no_trade), {
        id: 'edge',
        header: 'vs S00',
        cell: (i) => {
          const raw = i.row.original.incremental_net_edge_vs_no_trade;
          return <span className={`font-data ${pnlClass(raw)}`}>{signGlyph(raw)} {lamportsSigned(raw)}</span>;
        },
      }),
      col.accessor('status', { header: 'Status', cell: (i) => <Chip tone="muted">{i.getValue()}</Chip> }),
    ],
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  const fees = useMemo(
    () =>
      Object.entries(q.data?.payload?.fees_by_kind_lamports ?? {})
        .filter(([, v]) => BigInt(v) !== 0n)
        .map(([kind, v]) => ({ kind, lamports: Number(BigInt(v)), display: lamports(v) }))
        .sort((a, b) => b.lamports - a.lamports),
    [q.data],
  );

  return (
    <div className="mx-auto max-w-6xl space-y-4">
      {mode === 'PAPER' ? <PaperBanner /> : <LiveDisabledBanner />}
      {!runId && (
        <>
          <EmptyState what="selected run" hint="PnL views are strictly per-run. Pick a run below or in the header." />
          <AllRunsCard />
        </>
      )}

      {q.isLoading && runId && <LoadingState label="PnL summary" />}
      {q.isError && runId && <ErrorState message={(q.error as Error).message} />}
      {mode === 'REAL_MONEY' && runId && q.data?.data_status !== 'OK' && !q.isLoading && !q.isError && (
        <UnavailablePanel what="Real-money PnL" reason="No LIVE_CONFIRMED receipts exist; nothing may be displayed here." />
      )}

      {q.data?.data_status === 'SOURCE_MISMATCH' && (
        <>
          <Card>
            <CardTitle right={<StateChip status={q.data.data_status} reason={q.data.data_status_reason} />}>
              Comparison views voided for this run
            </CardTitle>
            <div className="rounded border border-neg/50 bg-neg/10 p-3 text-[12px]">
              <p className="font-semibold text-neg">{q.data.data_status_reason}</p>
              <p className="mt-2 text-muted">
                This is an honest fail-closed state (SOUL Law 5: a missing S00 baseline voids
                comparison views), not a loading failure. The run's own sealed scalars and trade
                records are still served on <span className="text-fg">Portfolio</span> and{' '}
                <span className="text-fg">Execution</span>, and its row is labeled in the
                all-runs table below.
              </p>
            </div>
          </Card>
          <AllRunsCard />
        </>
      )}

      {q.data?.data_status === 'OK' && q.data.payload && (
        <>
          <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
            <KpiCard
              label="Net PnL"
              tone={BigInt(q.data.payload.net_pnl_lamports) < 0n ? 'neg' : BigInt(q.data.payload.net_pnl_lamports) > 0n ? 'pos' : 'muted'}
              value={`${signGlyph(q.data.payload.net_pnl_lamports)} ${lamportsSigned(q.data.payload.net_pnl_lamports)}`}
              sub="lamports · cashflow-normalized (PF1)"
            />
            <KpiCard label="Realized" value={lamportsSigned(q.data.payload.realized_pnl_lamports)} sub={`scope ${q.data.payload.proof_state}`} tone={BigInt(q.data.payload.realized_pnl_lamports) < 0n ? 'neg' : 'muted'} />
            <KpiCard label="Unrealized" value={lamportsSigned(q.data.payload.unrealized_pnl_lamports)} sub="open lots at cost, not exit value" tone="muted" />
            <KpiCard label="External deposits" value={lamports(q.data.payload.external_deposits_lamports)} sub="deposits are not PnL" tone="muted" />
          </div>

          <Card>
            <CardTitle right={<StateChip status={q.data.data_status} />}>Fee waterfall</CardTitle>
            {fees.length === 0 ? (
              <UnavailablePanel what="Fees" reason="This run's sealed receipt records zero fees by kind (fees map empty)." />
            ) : (
              <div className="grid gap-4 lg:grid-cols-2">
                <BarChart data={fees} width={480} height={240} margin={{ top: 8, right: 8, bottom: 24, left: 8 }}>
                  <CartesianGrid stroke="var(--border)" strokeDasharray="2 4" />
                  <XAxis dataKey="kind" tick={{ fill: 'var(--fg-muted)', fontSize: 11 }} interval={0} angle={-30} textAnchor="end" />
                  <YAxis tick={{ fill: 'var(--fg-muted)', fontSize: 11 }} />
                  <ReferenceLine y={0} stroke="var(--border)" />
                  <Bar dataKey="lamports" radius={[2, 2, 0, 0]}>
                    {fees.map((f) => (
                      <Cell key={f.kind} fill="var(--accent)" />
                    ))}
                  </Bar>
                </BarChart>
                <table className="w-full text-[12px]">
                  <thead>
                    <tr>
                      <th scope="col" className="border-b border-border px-2 py-1.5 text-left text-[11px] uppercase tracking-wider text-muted">Fee kind</th>
                      <th scope="col" className="border-b border-border px-2 py-1.5 text-right text-[11px] uppercase tracking-wider text-muted">Lamports</th>
                    </tr>
                  </thead>
                  <tbody>
                    {fees.map((f) => (
                      <tr key={f.kind}>
                        <td className="border-b border-border/60 px-2 py-1.5 font-data">{f.kind}</td>
                        <td className="border-b border-border/60 px-2 py-1.5 text-right font-data">{f.display}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </Card>

          <Card>
            <CardTitle>Strategy comparison (S00 mandatory)</CardTitle>
            <table className="w-full text-[12px]">
              <thead>
                {table.getHeaderGroups().map((hg) => (
                  <tr key={hg.id}>
                    {hg.headers.map((h) => (
                      <th
                        key={h.id}
                        scope="col"
                        aria-sort={h.column.getIsSorted() === 'asc' ? 'ascending' : h.column.getIsSorted() === 'desc' ? 'descending' : 'none'}
                        className="cursor-pointer select-none border-b border-border px-2 py-1.5 text-left text-[11px] uppercase tracking-wider text-muted"
                        onClick={h.column.getToggleSortingHandler()}
                      >
                        {flexRender(h.column.columnDef.header, h.getContext())}
                        {{ asc: ' ↑', desc: ' ↓' }[h.column.getIsSorted() as string] ?? ''}
                      </th>
                    ))}
                  </tr>
                ))}
              </thead>
              <tbody>
                {table.getRowModel().rows.map((row) => (
                  <tr key={row.id} className={row.original.strategy === 'S00' ? 'bg-surface-2/60' : ''}>
                    {row.getVisibleCells().map((cell) => (
                      <td key={cell.id} className="border-b border-border/60 px-2 py-1.5">
                        {flexRender(cell.column.columnDef.cell, cell.getContext())}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
            <p className="mt-2 text-[11px] text-muted">
              S00 NO_TRADE baseline pinned; a missing baseline voids the view (server-enforced).
            </p>
          </Card>

          <Card>
            <CardTitle>Run metadata</CardTitle>
            <div className="grid gap-1 text-[12px] text-muted sm:grid-cols-2">
              <div>engine <span className="font-data text-fg">{q.data.payload.engine}</span></div>
              <div>currency <span className="font-data text-fg">{q.data.payload.currency}</span></div>
              <div>as_of <span className="font-data text-fg">{msToStamp(Date.parse(q.data.as_of))}</span> (projection build)</div>
              <div>unexitable exits <span className="font-data text-fg">{q.data.payload.unexitable_exits}</span></div>
            </div>
          </Card>
        </>
      )}
    </div>
  );
}
