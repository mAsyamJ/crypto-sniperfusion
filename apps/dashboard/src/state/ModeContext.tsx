/** Mode context: the READ-ONLY view switch. Mode + selected run persist in
 * the URL (?mode=PAPER&run=<id>) so scopes survive navigation and are
 * deep-linkable. Changing mode clears the whole query cache AND the run
 * selection (operator rule 4): nothing from the previous scope survives. */
import { createContext, useCallback, useContext, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useQueryClient } from '@tanstack/react-query';
import { isMode, type Mode } from '../mode';

interface ModeState {
  mode: Mode;
  runId: string | null;
  setMode: (m: Mode) => void;
  setRunId: (id: string | null) => void;
}

const Ctx = createContext<ModeState | null>(null);

export function ModeProvider({ children }: { children: React.ReactNode }) {
  const queryClient = useQueryClient();
  const [searchParams, setSearchParams] = useSearchParams();

  const urlMode = searchParams.get('mode');
  const [mode, setModeState] = useState<Mode>(isMode(urlMode) ? urlMode : 'PAPER');
  const [runId, setRunIdState] = useState<string | null>(searchParams.get('run'));

  const setMode = useCallback(
    (m: Mode) => {
      setModeState(m);
      queryClient.clear(); // drop every cached query from the other scope
      setRunIdState(null); // a run of one scope must never feed the other
      const next = new URLSearchParams(searchParams);
      next.set('mode', m);
      next.delete('run');
      setSearchParams(next, { replace: true });
    },
    [queryClient, searchParams, setSearchParams],
  );

  const setRunId = useCallback(
    (id: string | null) => {
      setRunIdState(id);
      const next = new URLSearchParams(searchParams);
      if (id) next.set('run', id);
      else next.delete('run');
      setSearchParams(next, { replace: true });
    },
    [searchParams, setSearchParams],
  );

  const value = useMemo(
    () => ({ mode, runId, setMode, setRunId }),
    [mode, runId, setMode, setRunId],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useMode(): ModeState {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error('useMode outside ModeProvider');
  return ctx;
}
