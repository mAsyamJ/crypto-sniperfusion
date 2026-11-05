import { describe, expect, it } from 'vitest';
import { lamports, lamportsSigned, lamportsToSol, signGlyph } from '../fmt';
import { modeOfProofState } from '../mode';

describe('money formatting (display-only transforms of served strings)', () => {
  it('groups digits and forces a sign', () => {
    expect(lamportsSigned('-250308')).toBe('\u2212250,308');
    expect(lamportsSigned('0')).toBe('+0');
    expect(lamportsSigned('9007199254740993')).toBe('+9,007,199,254,740,993');
  });
  it('groups without a sign for totals', () => {
    expect(lamports('1000000000')).toBe('1,000,000,000');
  });
  it('converts to SOL at the fixed 1e9 unit fact (not a price)', () => {
    expect(lamportsToSol('1000000000')).toBe('1.0');
    expect(lamportsToSol('-250308')).toBe('-0.000250308');
  });
  it('sign glyphs never rely on color alone', () => {
    expect(signGlyph('5')).toBe('\u25B2');
    expect(signGlyph('-5')).toBe('\u25BC');
    expect(signGlyph('0')).toBe('\u2014');
  });
  it('preserves 64-bit precision through BigInt (no float drift)', () => {
    expect(lamports('9007199254740993')).toBe('9,007,199,254,740,993');
  });
});

describe('mode classification (mirrors the Rust TradeMode rule)', () => {
  it('maps LIVE_CONFIRMED to REAL_MONEY and nothing else', () => {
    expect(modeOfProofState('LIVE_CONFIRMED')).toBe('REAL_MONEY');
    expect(modeOfProofState('SYNTHETIC')).toBe('PAPER');
    expect(modeOfProofState('FORK_MECHANICS')).toBe('PAPER');
    expect(modeOfProofState('SHADOW_ESTIMATE')).toBe('PAPER');
  });
});
