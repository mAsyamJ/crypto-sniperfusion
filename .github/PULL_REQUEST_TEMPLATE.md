<!--
SniperFusion PR checklist — mirrors AGENTS.md §4 (engineering loop).
Incomplete checklists are reviewed as incomplete work, not as failures.
-->

## Scope

<!-- One sentence: what this PR changes and which milestone/PR-plan item it belongs to. -->

- Milestone / plan item:
- Baseline SHA reviewed:

## Engineering loop

- [ ] Acceptance criteria specified before implementation
- [ ] Failing test written first (or N/A: docs-only — justify below)
- [ ] `cargo fmt --all` clean
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` exit 0 (both feature sets)
- [ ] `cargo test --workspace --locked` green ×2 (default + all features)
- [ ] `python3 -m unittest discover -s harness/tests` green (harness changes)
- [ ] `python3 harness/agentctl.py lint-skills` green (skill changes)

## Evidence discipline

- [ ] Write-once receipt added: `docs/evidence/{ID}.md` (commands, exit codes, measurements, open defects) — or N/A justified:
- [ ] `docs/PROJECT_STATUS.md` updated (milestone table / session record)
- [ ] Result labeled with the lowest evidence class it supports (see `docs/GLOSSARY.md`); synthetic outputs remain `promotable=false`

## Safety and licensing

- [ ] No secrets, keys, endpoint URLs with tokens, or signing material in the diff (history-aware scan reasoning included)
- [ ] No new `.github/secret-allowlist.txt` entries without a justification comment
- [ ] Upstream-derived code follows ADR-0002 clean-room rules; attribution updated in `docs/UPSTREAM_MANIFEST.md` if pins/uses changed
- [ ] No funded keys, no mainnet broadcast, no `--submit`, no visibility/credential changes

## Documentation

- [ ] New/moved/retired documents indexed in `docs/DOC_INDEX.md`
- [ ] Strategy status changes recorded in `docs/STRATEGY_REGISTRY.md` (via H8 registry + gates only)
- [ ] Provider/data-availability facts recorded in `docs/RPC_PROVIDERS.md` (with receipt)

## Commit discipline

- [ ] Conventional commit message
- [ ] Local commit only — push/merge to main requires owner approval
