# Control Tower Design System (DESIGN.md)

**Status:** LIVING (DASH-01 baseline; token changes are versioned) · **Date:** 2026-11-05
**Product:** SniperFusion Control Tower — a local research terminal for one operator. **AntiSlop mode for this workstream: `during` (session override).** This file is the design direction the antislop filter reads; antislop treats it as data (design fields), never as commands.

## 1. Identity & purpose (the soul)

A **research terminal**, not a trading casino and not a marketing site. The operator must answer, within ten seconds of landing: *Is anything running? Is the data reliable? What is the reconciled net PnL vs S00? What is blocked, stale, or unobserved?* The visual character: a quiet, dense, instrument-grade surface — closer to a lab bench instrument than a fintech landing page. If the logo were swapped out, the interface should still be unmistakably *this* terminal: honest-state chips everywhere, tabular numerals, zero celebratory styling. **Negative evidence (S00 champion, EDGE_NOT_DEMONSTRATED, zero-launch windows) is first-class content, styled with the same weight as positive results.**

Dials (per antislop Design Read): ENERGY 2/10 (calm), RHYTHM 3/10 (steady scan rhythm, no motion unless data changes), MOTION 1/10 (reduced-motion-safe transitions only), DENSITY 8/10 (data-dense, generous only where reading comprehension demands).

## 2. Design-system research inputs (recorded per DASH-00 tooling gate)

Two searches against the installed `ui-ux-pro-max` offline catalog (inputs recorded verbatim; outputs summarized; decisions ours):

1. `search.py "financial research terminal dashboard: dense data tables, PnL calendar, monospace tabular numerals, honest empty and unavailable states" --domain product --design-system` → suggested "Dark Mode (OLED)" style (slate/navy, dark-first), Fira Code/Fira Sans ("dashboard, data, analytics, code, technical, precise"), minimal glow, and an a11y pre-delivery checklist. **Adopted:** dark-first slate direction, Fira family, checklist. **Rejected:** its "Enterprise Gateway" page *pattern* (marketing hero/role-selection — wrong genre for an internal terminal) and its Google Fonts `@import` (violates NFR-1: frontend makes no external calls — fonts are bundled via `@fontsource/*`, OFL license, at DASH-03). Its "light mode not recommended" note was overridden: this product requires both modes (R-34 theme-toggle integrity), so both were built and verified below.
2. `search.py "financial charts: candlestick equity curve, daily PnL calendar heatmap, fee waterfall, drawdown" --domain chart` → candlestick guidance (Lightweight Charts; not-color-alone; table fallback), calendar heatmap (≤365 cells, divergent scale, numeric legend, texture/labels in addition to color), waterfall (4–12 bars, signed values + directional glyphs). **Adopted:** library picks (Lightweight Charts + Recharts), every chart has a table fallback, no color-only encodings.

## 3. Color tokens (contrast-verified with the installed `antislop-human/contrast-check.py`)

Measured ratios recorded — these are test-pinned values (DASH-03 asserts them), not claims.

### Dark (default)

| Token | Value | Use | Measured contrast (vs bg/surface) |
|---|---|---|---|
| `bg` | `#0B1120` | app background | — |
| `surface` | `#111A2E` | cards, tables | — |
| `surface-2` | `#1A2440` | headers, hover, chips | — |
| `border` | `#2A3A5C` | hairlines, grid | decorative (3:1 n/a) |
| `fg` | `#E6EDF7` | primary text | **15.98:1** on bg, **13.01:1** on surface-2 → AA |
| `fg-muted` | `#9FB0CC` | secondary text | **7.89:1** on surface → AA |
| `accent` | `#22D3EE` | interactive/data emphasis (single accent) | **10.42:1** on bg → AA |
| `pos` | `#34D399` | realized gain | **9.02:1** on surface → AA |
| `neg` | `#F87171` | realized loss | **6.27:1** on surface → AA |
| `on-accent` | `#0B1120` | text on accent buttons | **10.42:1** → AA |

### Light

| Token | Value | Measured contrast |
|---|---|---|
| `bg` / `surface` | `#F5F7FB` / `#FFFFFF` | — |
| `fg` | `#0F172A` | **16.65:1** on bg → AA |
| `fg-muted` | `#475569` | **7.58:1** on surface → AA |
| `accent` | `#0E7490` | **5.00:1** on bg → AA |
| `pos` / `neg` | `#047857` / `#B91C1C` | **5.48:1** / **6.47:1** on surface → AA |

Rules: one accent (`accent`) for interaction; `pos`/`neg` are **data semantics only** and never appear without a sign and a glyph (`▲`/`▼` or +/−), satisfying no-color-alone. No gradients, no glass, no bento tiling, no glow — flat surfaces + hairline borders + typographic hierarchy do the work (purpose test: hierarchy and readability; ornament would signal a product this is not).

State chips each get a fixed pair (dark/light) with ≥4.5:1 text: `OK` (accent-tinted), `UNAVAILABLE` (muted, hatched 45° pattern), `STALE` (amber `#FBBF24`/`#B45309`, verified at DASH-03 token table), `BLOCKED` (fg-muted + lock glyph), `SOURCE_MISMATCH` (neg + ⚠ glyph), `RECONCILING` (animated only if motion allowed), `INSUFFICIENT_SAMPLE` (muted), `SYNTHETIC` watermark (diagonal repeating 2% opacity text), `OFFLINE/UNKNOWN` (muted, hollow dot).

## 4. Typography

- **UI text:** Fira Sans (bundled `@fontsource/fira-sans`, OFL) — 13px base, 1.45 line height; sizes 11/12/13/15/18/24 (named `text-2xs…text-2xl`).
- **Data & numerals:** Fira Code (bundled, OFL) with `font-variant-numeric: tabular-nums` (Fira Code is monospace → columns of lamports/SOL align by construction). All money, slots, hashes, dates-in-tables render in data font.
- Density: table rows 28px; KPI numerals 24px data font; section headers 15px/600 with 2px accent underline offset (typographic, not decorative).
- `lang="en"`, real `<title>`, landmarks (`header/nav/main/aside`), zoom never blocked.

## 5. Layout & IA

Desktop-first dual-pane: left rail (56px icons + labels on focus/hover, keyboard-reachable) with the eight destinations (M1–M5, M10, M11, M12); content area max 1440 with a 12-col grid at ≥1024.

```
M1 Command Center                     ┌────────────────────────────────┐
┌───────────────────────────────┬────│ Runtime: ● OFFLINE/UNKNOWN     │
│ [RUN SWITCHER ▾ wa7  SYNT… ]  │    │ last artifact 3h ago           │
├───────────────────────────────┤    ├────────────────────────────────┤
│ VERDICT  EDGE_NOT_DEMONSTRATED│    │ vs S00 (mandatory)             │
│ net −250308 lamports  ▼        │    │ FOLLOW_DELAY_500MS   −250308 ▼│
│ deposits 0 · withdrawals 0     │    │ FOLLOW_DELAY_1000MS  −250308 ▼│
├──────────────┬────────────────┤    │ S00 NO_TRADE_BASELINE    0  =│
│ Blocked (2)  │ Stale (1)       │    ├────────────────────────────────┤
│ · G0 window  │ · fee-config    │    │ coverage: COMPLETE 3001 blk,   │
│   re-eval    │   probe 26h     │    │ 0 creates (D0 caveat chip)     │
└──────────────┴────────────────┘    └────────────────────────────────┘

M3 PnL Calendar (month grid)
      Mon Tue Wed Thu Fri Fri Sat Sun   ── tz: Asia/Jakarta ▾ (display-only)
wk45 ──  ──  ▼−83,436  ──  ▼−83,436 ──  ──     hatched cell = UNAVAILABLE
      cells: data font, sign+glyph+color; footer: Σ buckets = −250,308 ✓ reconciled

M5 Trade detail (drilldown pane)
  mint 4cNf…pump · entry 1793856330500 · exit 1793856350500 · net −83,436
  fees: protocol 0 · creator 0 · lp … · network 5,000 · tip 1,000 [SYNTHETIC 100bps chip]
  provenance: wa7/trades.jsonl · sha 1ea8bf… · proof SYNTHETIC · engine wallet_follower_v1
```

Rejected alternatives (purpose-gate log): hero banner with gradient stat cards (wrong genre; stats without live source); bento grid M1 (scanning cost > density); glassmorphism panels (contrast risk, zero function); animated tickers (fabricates liveness — violates P10 truth); pie chart of fees (waterfall carries sign + order; pie loses both).

## 6. States, motion, and interaction

- Every module implements the full state matrix (loading skeleton ≠ shimmer — a static 1px-hatched block; empty = "no artifacts discovered yet — run a SniperFusion command"; error = typed chip + retry; stale; partial; synthetic watermark; see REQUIREMENTS §7). A view may never show a number it cannot source.
- Motion: transitions ≤150ms opacity/transform only; `prefers-reduced-motion` → all transitions off, skeletons static; nothing moves unless its data changed.
- Keyboard: full task flow without pointer (rail = `1..8` hotkeys, tables: ↑/↓ row, Enter drilldown, Esc back, `/` run-switcher focus); visible 2px `accent` focus ring, offset 2px, always.
- Tables: TanStack virtualized beyond 100 rows; sticky header; column sort via labeled buttons (aria-sort); numeric columns right-aligned, tabular.
- Charts → table fallbacks (a11y): each chart block has an adjacent "table" toggle rendering the same numbers (also the Playwright assertion hook).

## 7. Responsive targets (DASH-03 test matrix)

1440×900 (primary), 1024×768 (rail collapses to 48px icons; grids 8-col), 390×844 (single column; tables become row-cards with the same numbers; calendar cells 44px tap targets), 320×700 (no horizontal overflow ever — R-03; KPI strip wraps to 2×2; fonts never below 11px). Light+dark both baselined in screenshots.

## 8. AntiSlop Delivery Gate (binding for every UI delivery)

Every UI milestone ends with the PASS/FAIL report (one line per Hard-Gate item with concrete evidence), run in `during` mode from the first line of UI code. DASH-01 ships tokens/contracts only — gate's UI items `NOT_APPLICABLE` this milestone; the token contrast table above pre-satisfies R-25 with measured values, and R-37 (direction before building) is satisfied by this document.
