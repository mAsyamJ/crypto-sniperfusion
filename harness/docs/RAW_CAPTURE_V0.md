# Raw Pump.fun program-touch capture V0

Scope: `harness/observation_capture.py` makes **read-only** JSON-RPC calls against Mainnet or Devnet after matching the exact cluster genesis hash. It samples up to 100 recent program-touch signatures, requests their raw transaction objects, records receive wall/monotonic clocks, unavailable-transaction flags and an integrity digest, then writes a new immutable JSON report under the ignored `harness/runs/` directory.

It cannot sign, fund, simulate, select mints, broadcast, submit orders, infer profitable trades, reconstruct canonical transaction indices, or claim full cohort coverage. **Recent program touches are not the same thing as all token creation events.** Commitment can change, RPC methods are paginated and public history may be incomplete. Its schema explicitly marks `all_launches_captured=false` and `coverage=UNKNOWN_INCOMPLETE_PROGRAM_TOUCH_ONLY`.

## Run from repository root

```bash
# Defaults to public read-only mainnet RPC. No key.
python3 harness/observation_capture.py \
  --cluster mainnet --limit 20 \
  --out harness/runs/raw-pump-mainnet-sample-001.json

# Alternatively configure authorized read-only private RPC via environment:
export SNF_MAINNET_READ_RPC='YOUR_READ_ONLY_MAINNET_RPC'
python3 harness/observation_capture.py --cluster mainnet --limit 40 \
  --out harness/runs/raw-pump-mainnet-sample-002.json
```

Never put API tokens in a CLI argument or commit environment files. The tool deliberately does not print its endpoint. A 429, dropped transaction, missing confirmation or genesis mismatch must be recorded or treated as a blocker; no absent event can be interpreted as a coin that never traded.

Next: add verified transaction-index chronology, complete create-event denominator with slot coverage/backfill, immutable canonical event Parquet partitions, program/IDL/version provenance, point-in-time feature availability, labels physically isolated from decisions, and source completeness attestations. Those are required before connecting observations to the S00–S10 quantitative harness.

**Evidence class:** `READ_ONLY_PROGRAM_TOUCH_SAMPLE_INCOMPLETE`; not mainnet shadow validation or strategy expected value. 
