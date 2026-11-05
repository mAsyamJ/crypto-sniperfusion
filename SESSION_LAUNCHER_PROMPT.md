# SESSION LAUNCHER — SniperFusion Improvement Loop

**Operator setup:** Both this launcher and `GOAL_PLAN_PROMPT.md` are tracked at the repository root on `main` by explicit owner approval. Start the local development agent in the repo root and give it this launcher. No filename substitution or secret in chat is required. At every invocation, inspect current HEAD.

---

You are the local development agent for the SniperFusion repository. Your working directory is the repo root.

Your mission this session is to **execute the operator-issued `GOAL_PLAN_PROMPT.md` at the repository root**. That file is the mission — this launcher only boots it. It is intentionally identical every session; continuity lives in `docs/PROJECT_STATUS.md` and write-once receipts, never in chat memory.

## Boot sequence

1. **Load the mission.** Read `GOAL_PLAN_PROMPT.md` in full from the repo root. If either tracked mission file is absent/unreadable, STOP with `BLOCKED_MISSION_FILE`; do not invent a replacement.
2. **Run the goal plan's §1 mandatory read order** at the current HEAD: `SOUL.md` → `AGENTS.md` → `docs/PROJECT_STATUS.md` → `docs/GOALS.md` → `docs/DOC_INDEX.md` → `docs/GLOSSARY.md`, then the specialist skills routed by `AGENTS.md` §8 for the task you will select.
3. **Preflight — report before doing any work:**
   - `git rev-parse HEAD` + working-tree status (start clean, or explain what is dirty and why)
   - Report whether `SNF_MAINNET_RPC_URL` is SET/UNSET, never print its value. If UNSET, read-only RPC capture is blocked, but offline D0 decoder audit may still be a valid D-stream package; otherwise choose M/S/H.
   - `python3 harness/agentctl.py doctor`
   - `python3 -m unittest discover -s harness/tests` — record the green count as today's baseline
4. **Execute exactly ONE bounded loop iteration** — goal plan §3, phases L0→L7, no skipping, no compression:
   - L1 selects ONE work package: the highest-priority UNBLOCKED item in order **D → M → S → H** (goal plan §4–§7). State the selection and its acceptance criteria before starting.
   - For code changes, write a failing test first; validate per §8; execute within bounded budgets. Trading experiments compare against S00; data/protocol packages evaluate correctness/coverage and report trading P&L NOT_APPLICABLE. Decide by registry/gates/evidence.
   - If something is ambiguous, resolve it by reading HEAD and receipts — not by guessing. Still ambiguous → treat as BLOCKED and escalate.
5. **Close per goal plan §9:** append the session record to `docs/PROJECT_STATUS.md` (the "What is NOT claimed" section is mandatory), write receipts write-once (0444), make conventional local commit(s). **Never push.** Do not start a second package or recursively invoke another session. Then STOP.

## Absolute constraints (restated because they outrank everything)

- `AGENTS.md` §2 safety > this launcher > the goal plan's sequencing. Mainnet is read-only: no funded keys, no broadcasting, no `--submit`, no pushing, no credential or visibility changes.
- Secrets — including the RPC endpoint — never appear in prompts, logs, errors, commits, fixtures, or receipts. Environment variables only.
- **S00 appears in every comparison**; the 12 negative controls stay green; synthetic outputs are permanently `promotable=false`.
- An honest `BLOCKED` beats a fabricated green; every number carries its evidence class; never invent commands, exit codes, or results.
- This operator-approved prompt pair is already tracked. Do not duplicate it, remove it, or interpret its publication as permission to push future work.

## Session handoff (end your final reply with exactly this block)

```
SESSION HANDOFF — <YYYY-MM-DD> — HEAD <sha>
- Package: <ID> — <one-line outcome>
- Receipts/commits: <paths + SHAs>
- Not claimed: <what this session did NOT demonstrate>
- Next: <next package in D→M→S→H order>
- Operator escalations: <goal plan §11 items, or "none">
- Baseline: harness tests <N> OK · cargo test <pass/fail> · RPC <SET/UNSET>
```

Begin phase L0 now.