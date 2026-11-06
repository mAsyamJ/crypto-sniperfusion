/** App layout: header (mode selector, run selector, theme toggle), left
 * rail navigation with keyboard hotkeys 1..5, main content area. */
import { NavLink, Outlet, useLocation } from 'react-router-dom';
import { Activity, CalendarDays, CandlestickChart, LayoutGrid, Moon, Sun, Wallet, ListOrdered } from 'lucide-react';
import { clsx } from 'clsx';
import { useMode } from '../state/ModeContext';
import { useTheme } from '../state/ThemeContext';
import { MODE_LABEL } from '../mode';
import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';

const NAV = [
  { to: '/', label: 'Command Center', icon: LayoutGrid },
  { to: '/pnl', label: 'PnL Analytics', icon: CandlestickChart },
  { to: '/calendar', label: 'PnL Calendar', icon: CalendarDays },
  { to: '/portfolio', label: 'Portfolio', icon: Wallet },
  { to: '/execution', label: 'Execution', icon: ListOrdered },
];

function ModeSelector() {
  const { mode, setMode } = useMode();
  return (
    <div
      role="radiogroup"
      aria-label="Trading mode (read-only view switch)"
      className="flex overflow-hidden rounded border border-accent/60"
    >
      {(['PAPER', 'REAL_MONEY'] as const).map((m) => (
        <button
          key={m}
          role="radio"
          aria-checked={mode === m}
          data-testid={`mode-${m}`}
          onClick={() => setMode(m)}
          className={clsx(
            'cursor-pointer px-3 py-1.5 text-[12px] font-semibold tracking-wide',
            mode === m ? 'bg-accent text-onaccent' : 'bg-surface text-muted hover:text-fg',
          )}
        >
          {MODE_LABEL[m]}
        </button>
      ))}
    </div>
  );
}

function RunSelector() {
  const { mode, runId, setRunId } = useMode();
  const { data, isLoading } = useQuery({
    queryKey: ['runs', mode],
    queryFn: () => api.runs(mode),
  });
  const runs = data?.payload ?? [];
  return (
    <label className="flex items-center gap-2 text-[12px] text-muted">
      <span className="hidden sm:inline">Run</span>
      <select
        data-testid="run-selector"
        value={runId ?? ''}
        onChange={(e) => setRunId(e.target.value || null)}
        className="max-w-[220px] cursor-pointer rounded border border-border bg-surface px-2 py-1.5 font-data text-[12px] text-fg"
      >
        <option value="">{isLoading ? 'loading…' : runs.length === 0 ? 'no runs in scope' : 'select a run'}</option>
        {runs.map((r) => (
          <option key={r.id} value={r.id}>
            {r.id} · {r.engine} · {r.proof_state}
          </option>
        ))}
      </select>
    </label>
  );
}

export default function Layout() {
  const { mode } = useMode();
  const { theme, toggle } = useTheme();
  const location = useLocation();
  return (
    <div className="flex h-screen flex-col bg-bg text-fg">
      <header className="flex flex-wrap items-center gap-3 border-b border-border bg-surface px-4 py-2.5">
        <div className="flex items-center gap-2 font-semibold">
          <Activity aria-hidden="true" className="text-accent" size={18} />
          <span className="text-[15px]">SniperFusion Control Tower</span>
        </div>
        <ModeSelector />
        <RunSelector />
        <div className="ml-auto flex items-center gap-2">
          <span data-testid="scope-chip" className="rounded border border-border px-2 py-1 text-[11px] uppercase tracking-wider text-muted">
            scope: {mode}
          </span>
          <button
            onClick={toggle}
            aria-label={`Switch to ${theme === 'dark' ? 'light' : 'dark'} theme`}
            className="cursor-pointer rounded border border-border bg-surface p-1.5 text-muted hover:text-fg"
          >
            {theme === 'dark' ? <Sun size={14} /> : <Moon size={14} />}
          </button>
        </div>
      </header>
      <div className="flex min-h-0 flex-1">
        <nav aria-label="Pages" className="flex w-14 flex-col gap-1 border-r border-border bg-surface p-2 lg:w-48">
          {NAV.map(({ to, label, icon: Icon }) => (
            <NavLink
              key={to}
              to={{ pathname: to, search: location.search }}
              end={to === '/'}
              aria-label={label}
              className={({ isActive }) =>
                clsx(
                  'flex items-center gap-2 rounded px-2 py-2 text-[12px] font-medium',
                  isActive ? 'bg-surface-2 text-accent' : 'text-muted hover:bg-surface-2 hover:text-fg',
                )
              }
            >
              <Icon size={16} aria-hidden="true" />
              <span className="hidden lg:inline">{label}</span>
            </NavLink>
          ))}
        </nav>
        <main tabIndex={0} role="region" aria-label="Main content" className="min-w-0 flex-1 overflow-y-auto p-4">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
