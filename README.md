# Solana Sniper Fusion

Rust-native Solana memecoin sniper under milestone-gated engineering. Baseline behavior: [chainstacklabs/pumpfun-bonkfun-bot](https://github.com/chainstacklabs/pumpfun-bonkfun-bot) (Apache-2.0, pinned at `upstream/chainstack`), used strictly as an offline differential-test oracle. Protocol truth: official pump.fun docs/IDLs (`upstream/pump-docs`) + on-chain fixtures. Selected designs from three additional reference repos under clean-room rules (see `docs/ADRS/0002-clean-room-licensing.md`).

**Status: M0 complete (discovery, baseline, dependency selection, protocol verification). Live trading is disabled by default and additionally gated four ways. No funded keys exist in this repository.**

## Safety posture

- Default runtime modes: `observe`, `replay`, `paper` — no broadcaster, no signer, structurally.
- `LIVE_TRADING_ENABLED=false` is the default **and not the only safeguard**: a live-capable build requires a non-default cargo feature, an operator authorization token, validated budget config, and a healthy kill switch.
- Every money-moving order must carry a typed `RiskAuthorization` (strategy/mint/venue/limits/fees/slippage/validity) — strategies and emergency paths cannot bypass it.
- No mainnet broadcast, funded wallets, paid infrastructure, or code push happens without explicit owner authorization.

## Layout

```
crates/            # Rust workspace (from M1: sniper-core, sniper-protocols, sniper-runtime, sniper-app)
docs/              # manifest, capability matrix, architecture, threat model, ADRs, compatibility, evidence/
fixtures/          # sanitized recorded chain examples (M2+)
config/            # observe/simulation configs (M1+)
upstream/          # pinned reference repos (gitignored, read-only)
```

## Documentation index

| Doc | Purpose |
|---|---|
| `docs/UPSTREAM_MANIFEST.md` | Pinned SHAs, licenses, provenance, security findings |
| `docs/CAPABILITY_MATRIX.md` | Per-repo verified/stub/outdated + Chainstack parity rows |
| `docs/ARCHITECTURE.md` | Four-crate design, event model, lifecycle, priorities |
| `docs/THREAT_MODEL.md` | Adversaries, threats T1–T16, invariants |
| `docs/SOLANA_COMPATIBILITY.md` | SDK generation spike evidence + decision |
| `docs/PROTOCOL_COMPATIBILITY.md` | Pump.fun V2/V3 + PumpSwap V1/V2 instruction matrix |
| `docs/ADRS/` | 0001 rust-first · 0002 clean-room · 0003 SDK generation |
| `docs/TEST_PLAN.md` | Layers L1–L9, per-milestone gates |
| `docs/PROJECT_STATUS.md` | Living status + next actions |
| `docs/evidence/M*.md` | Per-milestone verbatim evidence receipts |

## Reproducing the M0 baseline

```bash
cd upstream/chainstack && uv sync --frozen && uv run python tests/regression/run_all.py
# → 38/38 passed (see docs/evidence/M0.md)
```

License notice: this repository's own code is proprietary to the operator until otherwise licensed. Chainstack behavioral reference is Apache-2.0 (attribution in `docs/UPSTREAM_MANIFEST.md`).
