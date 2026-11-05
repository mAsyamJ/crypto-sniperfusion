/** Shared presentational primitives (shadcn-style local components styled
 * with DESIGN.md tokens; Radix powers Tabs and Tooltip). Every state chip
 * carries text, not color alone (R-39). */
import { clsx } from 'clsx';
import type { ReactNode } from 'react';

export function Card({ className, children }: { className?: string; children: ReactNode }) {
  return (
    <section className={clsx('rounded border border-border bg-surface p-4', className)}>
      {children}
    </section>
  );
}

export function CardTitle({ children, right }: { children: ReactNode; right?: ReactNode }) {
  return (
    <div className="mb-3 flex items-baseline justify-between gap-2">
      <h2 className="text-[15px] font-semibold text-fg">
        <span className="mr-2 inline-block h-[2px] w-4 translate-y-[-4px] bg-accent" aria-hidden="true" />
        {children}
      </h2>
      {right}
    </div>
  );
}

export type ChipTone = 'ok' | 'muted' | 'warn' | 'neg' | 'accent' | 'hatched';

const TONE: Record<ChipTone, string> = {
  ok: 'border-accent/60 text-accent',
  muted: 'border-border text-muted',
  warn: 'border-[#fbbf24]/60 text-[#fbbf24] dark:text-[#fbbf24]',
  neg: 'border-neg/60 text-neg',
  accent: 'border-accent/60 text-accent',
  hatched: 'border-border text-muted hatched',
};

export function Chip({ tone, children, title }: { tone: ChipTone; children: ReactNode; title?: string }) {
  return (
    <span
      title={title}
      className={clsx(
        'inline-flex items-center gap-1 rounded-sm border px-1.5 py-0.5 text-[11px] font-medium uppercase tracking-wide',
        TONE[tone],
      )}
    >
      {children}
    </span>
  );
}

/** Map API data_status to a chip tone + label. */
export function statusTone(status: string): ChipTone {
  switch (status) {
    case 'OK': return 'ok';
    case 'UNAVAILABLE': return 'hatched';
    case 'STALE': return 'warn';
    case 'SOURCE_MISMATCH': return 'neg';
    case 'BLOCKED':
    case 'INSUFFICIENT_SAMPLE':
    case 'RECONCILING': return 'muted';
    default: return 'muted';
  }
}

export function StateChip({ status, reason }: { status: string; reason?: string | null }) {
  return <Chip tone={statusTone(status)} title={reason ?? undefined}>{status.replaceAll('_', ' ')}</Chip>;
}

export function KpiCard({ label, value, sub, tone }: { label: string; value: ReactNode; sub?: ReactNode; tone?: 'pos' | 'neg' | 'muted' }) {
  const toneClass = tone === 'pos' ? 'text-pos' : tone === 'neg' ? 'text-neg' : 'text-fg';
  return (
    <Card className="min-w-0">
      <div className="text-[11px] uppercase tracking-wider text-muted">{label}</div>
      <div className={clsx('font-data mt-1 truncate text-xl', toneClass)}>{value}</div>
      {sub ? <div className="mt-1 text-[11px] text-muted">{sub}</div> : null}
    </Card>
  );
}

export function UnavailablePanel({ what, reason }: { what: string; reason: string }) {
  return (
    <div role="note" className="hatched flex min-h-[96px] items-center justify-center rounded border border-border p-4 text-center">
      <div>
        <div className="font-semibold uppercase tracking-wider text-muted">{what}: UNAVAILABLE</div>
        <div className="mt-1 max-w-md text-[12px] text-muted">{reason}</div>
      </div>
    </div>
  );
}

export function EmptyState({ what, hint }: { what: string; hint: string }) {
  return (
    <div className="flex min-h-[96px] items-center justify-center rounded border border-dashed border-border p-4 text-center">
      <div>
        <div className="font-semibold text-muted">No {what} yet</div>
        <div className="mt-1 max-w-md text-[12px] text-muted">{hint}</div>
      </div>
    </div>
  );
}

export function ErrorState({ message }: { message: string }) {
  return (
    <div role="alert" className="rounded border border-neg/60 bg-neg/10 p-3 text-[12px] text-fg">
      <span className="font-semibold text-neg">ERROR </span>
      {message}
    </div>
  );
}

export function LoadingState({ label }: { label: string }) {
  return (
    <div aria-busy="true" aria-label={`Loading ${label}`} className="hatched min-h-[64px] rounded border border-border opacity-70" />
  );
}

/** PAPER / SIMULATED banner — permanent on every paper-scope page. */
export function PaperBanner() {
  return (
    <div role="note" className="mb-3 rounded border border-[#fbbf24]/50 bg-[#fbbf24]/10 px-3 py-1.5 text-[12px] font-semibold tracking-wide text-[#fbbf24]">
      PAPER / SIMULATED &middot; NOT REAL MONEY
    </div>
  );
}

/** Real-money disabled banner (the honest default today). */
export function LiveDisabledBanner() {
  return (
    <div role="note" className="mb-3 rounded border border-border bg-surface-2 px-3 py-1.5 text-[12px] font-semibold tracking-wide text-muted">
      LIVE TRADING DISABLED / NO VERIFIED REAL TRADES
    </div>
  );
}

export function Th({ children, numeric }: { children: ReactNode; numeric?: boolean }) {
  return (
    <th
      scope="col"
      className={clsx(
        'sticky top-0 z-10 border-b border-border bg-surface px-2 py-1.5 text-left text-[11px] font-semibold uppercase tracking-wider text-muted',
        numeric && 'text-right',
      )}
    >
      {children}
    </th>
  );
}

export function Td({ children, numeric, className }: { children: ReactNode; numeric?: boolean; className?: string }) {
  return (
    <td className={clsx('border-b border-border/60 px-2 py-1.5', numeric && 'text-right font-data', className)}>
      {children}
    </td>
  );
}
