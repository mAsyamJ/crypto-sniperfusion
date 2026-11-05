/** Money formatting: the API serves lamports as decimal STRINGS; the UI
 * formats for display ONLY (grouping, SOL at the fixed 1e9 lamports/SOL
 * unit fact). No arithmetic on money happens in JS beyond this unit
 * presentation; every displayed value is copied from a served string. */

const GROUP = /\B(?=(\d{3})+(?!\d))/g;

function groupDigits(intPart: string): string {
  const neg = intPart.startsWith('-');
  const digits = neg ? intPart.slice(1) : intPart;
  return (neg ? '-' : '') + digits.replace(GROUP, ',');
}

/** "−250,308" style with an explicit sign glyph. */
export function lamportsSigned(raw: string): string {
  const n = BigInt(raw);
  const abs = n < 0n ? -n : n;
  const sign = n < 0n ? '\u2212' : '+';
  return sign + groupDigits(abs.toString());
}

/** Plain grouped form, no forced sign (totals, basis). */
export function lamports(raw: string): string {
  return groupDigits(raw);
}

/** SOL display at the fixed unit fact 1e9 lamports/SOL (not a price). */
export function lamportsToSol(raw: string): string {
  const n = BigInt(raw);
  const neg = n < 0n;
  const magnitude = neg ? -n : n;
  const abs = magnitude.toString().padStart(10, '0');
  const sol = `${abs.slice(0, -9)}.${abs.slice(-9).replace(/0+$/, '')}`;
  return (neg ? '-' : '') + (sol.endsWith('.') ? sol + '0' : sol);
}

/** Sign glyph for not-color-alone semantics (DESIGN.md §3). */
export function signGlyph(raw: string): '\u25B2' | '\u25BC' | '\u2014' {
  const n = BigInt(raw);
  if (n > 0n) return '\u25B2';
  if (n < 0n) return '\u25BC';
  return '\u2014';
}

export function pnlClass(raw: string): string {
  const n = BigInt(raw);
  return n > 0n ? 'text-pos' : n < 0n ? 'text-neg' : 'text-muted';
}

/** ISO day to a readable label (no locale drift in tests). */
export function dayLabel(day: string): string {
  return day;
}

/** ms unix to a short human stamp (UTC, display-only). */
export function msToStamp(ms: number): string {
  const d = new Date(ms);
  const p = (x: number) => String(x).padStart(2, '0');
  return `${d.getUTCFullYear()}-${p(d.getUTCMonth() + 1)}-${p(d.getUTCDate())} ${p(d.getUTCHours())}:${p(d.getUTCMinutes())}Z`;
}
