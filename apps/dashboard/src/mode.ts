/** Presentation-level trading modes (operator DASH-03 contract).
 * The API derives mode ONLY from sealed proof states (LIVE_CONFIRMED ==
 * REAL_MONEY); the UI copies that classification and never rewrites
 * evidence labels. This selector is a READ-ONLY VIEW SWITCH. */
export type Mode = 'PAPER' | 'REAL_MONEY';

export const MODE_LABEL: Record<Mode, string> = {
  PAPER: 'Paper Trading',
  REAL_MONEY: 'Real Money',
};

/** Papers run under these simulation types, filtered by actual source
 * evidence (engine + completeness), never by UI guesswork. */
export const SIMULATION_TYPES = ['Paper', 'Replay', 'Fork', 'Shadow'] as const;
export type SimulationType = (typeof SIMULATION_TYPES)[number];

export function modeOfProofState(proofState: string): Mode {
  return proofState === 'LIVE_CONFIRMED' ? 'REAL_MONEY' : 'PAPER';
}

export function isMode(v: unknown): v is Mode {
  return v === 'PAPER' || v === 'REAL_MONEY';
}
