# Operator Checklist — enabling model-backed Harness²

1. Provision the worker VM/container exactly per `runner.spec.md` (all 11
   controls verified, evidenced by the audit commands you ran).
2. Install harness² inside the worker only (`scripts/install-harness2.sh`).
3. Provide ONLY `HARNESS2_CODEX_*` credentials inside the worker's env
   (never SNF_* / wallet material — the scrubber drops them anyway).
4. Run `python3 harness/harness2ctl.py doctor` inside the worker and
   confirm every check.
5. Create the marker INSIDE THE WORKER:
   `touch integrations/harness2/worker/.operator-enabled`
6. Record the enablement (date, worker identity, auditor) in
   `docs/evidence/harness2-model-enablement.md` on the DEV repo via a
   normal reviewed commit.
7. Bound every model-backed run: explicit `--k`, wall-clock and cost
   budget noted before launch; results are research-harness evidence
   only and NEVER trading-strategy evidence.
