# DASH-00 — Tooling Audit (skill provisioning receipt)

- **Workstream:** SniperFusion Control Tower dashboard (historical master plan: `docs/history/dashboard/SNIPERFUSION_DASHBOARD_GOAL_PLAN_PROMPT.md` Phase 0; no longer an active launcher)
- **Session date (UTC):** 2026-11-05
- **Base:** `main` @ `54bf3d28d56c2c0c20b00a2e7deda3081a41eee0` (merge containing `origin/main` @ `36f7b019`, the PR #12 dashboard mission files)
- **Working branch:** `feat/dashboard-control-tower`
- **Harness:** ZCode CLI agent; skill installs performed via the Vercel `skills` CLI (project scope)
- **Status vocabulary:** this file records what WAS done and verified. Deferred items are marked `DEFERRED` with their owning milestone.

## 1. Preflight (truthful baseline)

### 1.1 Environment

| Item | Value |
|---|---|
| Node.js | v22.23.2 (nvm, sole installed version) — satisfies the `skills` CLI requirement of Node ≥ 22.20 |
| npm / npx | 10.9.8 |
| rustc / cargo | 1.99.1 / 1.99.1 (`rust-toolchain.toml` pin honored) |
| Python | 3.12.3 |
| git | 2.43.0 |
| rtk | 0.48.0 (`/home/ubuntu/.local/bin/rtk`) |
| RAM at preflight | 7.6 Gi total, ~4.4 Gi available (builds capped `CARGO_BUILD_JOBS=2` per `.cargo/config.toml`) |
| Git sync | `git fetch origin` run; `git merge-base --is-ancestor origin/main HEAD` → true (HEAD already contains origin/main; local ahead by the 3 D0 commits `f36c1b1`, `e671510`, merge `54bf3d2`). No merge needed. |
| Dirty tree | 2 untracked, unrelated, preserved untouched: `deep-research-report (3).md`, `deep-research-report (3).md:Zone.Identifier` |

### 1.2 Baseline commands and exit codes (run BEFORE any tooling change)

| Command | Exit | Result |
|---|---|---|
| `python3 harness/agentctl.py doctor` | 0 | 19 skills discovered; `live_trading_enabled=false`; `mainnet_broadcast_possible_from_this_tool=false`; `alpha: NOT_DEMONSTRATED` |
| `python3 harness/agentctl.py lint-skills` | 0 | `{"validated": 19, "status": "PASS"}` |
| `python3 -m unittest discover -s harness/tests` | 0 | 132 tests, OK |
| `CARGO_BUILD_JOBS=2 cargo fmt --all -- --check` | 0 | clean |
| `CARGO_BUILD_JOBS=2 cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 | clean |
| `CARGO_BUILD_JOBS=2 cargo test --workspace --locked` | 0 | 201 passed, 0 failed |
| `CARGO_BUILD_JOBS=2 cargo test --workspace --locked --all-features` | 0 | 312 passed, 0 failed, 3 ignored |

Re-verification AFTER installs: `lint-skills` → 0 (`{"validated": 33, "status": "PASS"}`); `unittest discover` → 0 (132 tests OK). CI contracts (`ci.yml`, `harness-validation.yml`) remain satisfiable; no preexisting CI contract was modified.

## 2. Upstream security review (performed BEFORE any installer ran)

Method: read-only inspection of each repository's README/docs/license on GitHub plus `npm view` registry metadata for the installer CLI; after install, local pattern scans of everything that entered the repo tree. No pipe-to-shell installers were used at any point. All installs used the `npx skills add` route (file-copy installer), never `npx antislop` / `npx antislop-ai` (the anti-slop FAQ warns the npm name `antislop` is an unrelated package).

| Upstream | License | HEAD at review (2026-11-05) | Review notes |
|---|---|---|---|
| `miqdadbadjuber/anti-slop` | MIT | `9504989f162a92b1996853eccbb3d8a86e73df68` | Rulebook/skills project (no runtime code in the skills route). Ships `cli/`+`scripts/` for its own installer, which we did NOT run. Installed release: VERSION `3.2.26`. `antislop-ui`/`antislop-human` reference content embeds `LICENSE-web-quality-skills` — also MIT (© 2026 Addy Osmani, derived from web-quality-skills), attribution preserved in-file. `antislop-human` ships `contrast-check.py`/`contrast-mcp.py`: verified locally — no network imports; pure local WCAG contrast computation. |
| `nextlevelbuilder/ui-ux-pro-max-skill` | MIT | `b7d5353e0130feb8c5574ef1251f8a1db1873ccb` | Design-intelligence skill with CSV databases + Python search scripts. Upstream claims scripts are stdlib-only/no-network; **verified locally**: `grep -rnE "import urllib|import requests|import socket|import http|urlopen"` over `scripts/*.py` → only one benign string comparison against a repository URL; no shell/JS files present. |
| `obra/superpowers` | MIT | `22f0b463e7a5e579904dd6211a520321035526f7` | Skills-only route ships no hooks. The `brainstorming` skill carries `scripts/` (a local visual-companion server: `server.cjs`, `helper.js`, start/stop shell scripts) whose only outbound reference is the documented `primeradiant.com` brand-image telemetry and which binds locally with a session token. **Discipline adopted (binding for this workstream):** these scripts are committed as skill content but are NEVER executed; the workflow guidance in `SKILL.md` is the used part. Set `SUPERPOWERS_DISABLE_TELEMETRY=1` whenever invoking these workflows, per the architecture plan's "no telemetry phoning home" rule. |
| `vercel-labs/skills` (installer CLI) | MIT | npm `skills@1.7.6` | Publisher: vercel-labs (maintainers rauchg, quuu). Executes as `npx --yes skills@latest`; clones the target repo, copies skill folders into agent directories. Observed writes: project `.agents/skills/`, `.claude/skills/` + `.zcode/skills/` relative symlinks, `skills-lock.json`. |

## 3. Installed inventory (project-local, all under `.agents/skills/`)

Install command pattern (verbatim route from the goal plan; `-y` selects project scope non-interactively; every invocation exited 0):

```bash
npx --yes skills@latest add miqdadbadjuber/anti-slop --list            # found 6 skills
npx --yes skills@latest add miqdadbadjuber/anti-slop --skill antislop -y
npx --yes skills@latest add miqdadbadjuber/anti-slop --skill antislop-ui -y
npx --yes skills@latest add miqdadbadjuber/anti-slop --skill antislop-human -y
npx --yes skills@latest add miqdadbadjuber/anti-slop --skill antislop-layoutmobile -y
npx --yes skills@latest add miqdadbadjuber/anti-slop --skill antislop-code -y
npx --yes skills@latest add miqdadbadjuber/anti-slop --skill antislop-copywriting -y
npx --yes skills@latest add nextlevelbuilder/ui-ux-pro-max-skill --list  # found 7 skills
npx --yes skills@latest add nextlevelbuilder/ui-ux-pro-max-skill --skill ui-ux-pro-max -y
npx --yes skills@latest add obra/superpowers --list                     # found 15 skills
# (7 workflow skills, one invocation each)
npx --yes skills@latest add obra/superpowers --skill brainstorming -y
npx --yes skills@latest add obra/superpowers --skill writing-plans -y
npx --yes skills@latest add obra/superpowers --skill using-git-worktrees -y
npx --yes skills@latest add obra/superpowers --skill test-driven-development -y
npx --yes skills@latest add obra/superpowers --skill systematic-debugging -y
npx --yes skills@latest add obra/superpowers --skill requesting-code-review -y
npx --yes skills@latest add obra/superpowers --skill verification-before-completion -y
npx --yes skills@latest list                                            # exit 0; sources attributed
```

**AntiSlop was installed FIRST**, before any other tooling, per the Phase 0 gate.

### 3.1 Pins

Two pin layers are committed:

1. **`skills-lock.json`** (repo root, written by the CLI): per-skill `source`, `sourceType: github`, `skillPath`, and `computedHash` (SHA-256 of the skill). `npx skills experimental_install` can restore from it.
2. **SKILL.md SHA-256 pins** (recorded here as a second, finer-grained pin layer; note the two layers hash different domains — the lock file hashes the whole skill directory, the table below hashes only `SKILL.md` — corrected 2026-11-05 in the DASH-01 session; the original draft wrongly implied the hashes would coincide):

| Skill | Source | SKILL.md sha256 (first 16 hex) |
|---|---|---|
| antislop (v3.2.26) | miqdadbadjuber/anti-slop | `30bc532fb59d4c77` |
| antislop-ui | miqdadbadjuber/anti-slop | `e8cb54f3d436de92` |
| antislop-human | miqdadbadjuber/anti-slop | `49421878e0c014fa` |
| antislop-layoutmobile | miqdadbadjuber/anti-slop | `8613395a5109f637` |
| antislop-code | miqdadbadjuber/anti-slop | `004933009d352710` |
| antislop-copywriting | miqdadbadjuber/anti-slop | `d708fc33608b4f1d` |
| ui-ux-pro-max | nextlevelbuilder/ui-ux-pro-max-skill | `9aa50bd44076db28` |
| brainstorming | obra/superpowers | `2c9ca71f94886f9c` |
| writing-plans | obra/superpowers | `a6c67c1900064347` |
| using-git-worktrees | obra/superpowers | `8cfb86f121269e8f` |
| test-driven-development | obra/superpowers | `64b03fce4aee5a97` |
| systematic-debugging | obra/superpowers | `51c6435d98c83c42` |
| requesting-code-review | obra/superpowers | `2180e14a448e9410` |
| verification-before-completion | obra/superpowers | `2befe7fc55bcadaa` |

File counts: antislop 2, antislop-ui 4, antislop-human 6, antislop-layoutmobile 1, antislop-code 1, antislop-copywriting 1, ui-ux-pro-max 74 (CSV data + stdlib Python), brainstorming 8, writing-plans 1, using-git-worktrees 1, test-driven-development 2, systematic-debugging 6, requesting-code-review 2, verification-before-completion 1.

### 3.2 Where files live

- Canonical: `.agents/skills/<name>/` (committed; passes repo `lint-skills` contract — kebab-case name matching directory, description ≥ 25 chars, non-empty body).
- Agent symlinks: `.claude/skills/<name>` (relative symlinks, committed) and `.zcode/skills/<name>` (machine-local; `/.zcode/` is gitignored).
- Pin file: `skills-lock.json` (committed).

### 3.3 AntiSlop mode activation (this session)

- Mode resolution per the antislop core skill: explicit session instruction (mission mandate: "`during` mode … without writing a global preference") → **`antislop active: during (session override).`**
- Verified: `~/.config/antislop/settings.json` does NOT exist (no global preference written; none will be).
- The core skill + Delivery Gate (Hard Gate R-02/R-03/R-17/R-18/R-23/R-24/R-25/R-26/R-27/R-28/R-32/R-33/R-34/R-35/R-36/R-37/R-38/R-39/R-40; Purpose-Gate; Liveliness; C-1..C-5) were read in-session. The gate binds all future Control Tower UI deliverables (DASH-03+); DASH-00 itself ships no UI, so the gate's UI items are `NOT_APPLICABLE` here and its process items (direction before building; no unapproved assets) are satisfied by this audit + the DESIGN.md-first rule.
- Note: skills load at agent-session start. These were installed mid-session and their content was verified by direct read; sessions started after this commit load them automatically from the project directory.

### 3.4 Superpowers note

The goal plan's "official per-host method" resolves on this host to the skills-directory mechanism (`.agents/skills/`, `~/.agents/skills/`, `~/.zcode/skills/`), which is how the CLI installs them; the plugin/hook route was deliberately NOT used (no session-start hooks injected — smaller side-effect surface). `using-superpowers` (the plugin bootstrap skill) was intentionally not installed for the same reason. The 7 workflow skills the master plan requires are pinned above. Host-level copies of several of these skills already existed in `~/.agents/skills/` before this session (pre-existing environment); the project-local pins are now the authoritative in-repo source.

## 4. DEFERRED runtime dependencies (reviewed, NOT installed at DASH-00)

Per the master plan's own gate ("runtime UI libs only after `DESIGN.md` + contracts approved"), the following are planned for DASH-02/DASH-03 with license pre-review recorded now. None are present in the repo yet; no `package.json` exists yet.

| Dependency | License | Planned milestone | Notes |
|---|---|---|---|
| react, react-dom | MIT | DASH-03 | |
| vite + @vitejs/plugin-react | MIT | DASH-03 | |
| typescript | Apache-2.0 | DASH-03 | |
| tailwindcss | MIT | DASH-03 | |
| radix-ui primitives | MIT | DASH-03 | accessible headless components |
| shadcn/ui (copied components) | MIT | DASH-03 | styled with DESIGN.md tokens, not defaults |
| @tanstack/react-query, -table, (-virtual when needed) | MIT | DASH-03 | |
| lightweight-charts | Apache-2.0 | DASH-03 | **attribution required** by its terms |
| recharts OR visx (pick one) | MIT | DASH-03 | decision recorded at DASH-01 |
| lucide-react | ISC | DASH-03 | |
| zod | MIT | DASH-02/03 | browser boundary validation |
| vitest + @testing-library/react | MIT | DASH-03 | |
| @playwright/test | Apache-2.0 | DASH-03+ | browser/a11y/viewport tests |
| axe-core | MPL-2.0 | DASH-03+ | in cargo-deny allowlist |
| Rust: axum, tokio, serde, rusqlite (or sqlx) | MIT/Apache-2.0/ISC | DASH-02 | must pass `cargo deny check` allowlist; SQLite WAL single-writer |

All Rust-side additions remain subject to `deny.toml` (licenses, advisories, sources) and ADR-0003 pinning discipline; all npm additions must be version-pinned with a committed lockfile (goal plan Phase 5).

## 5. Security checklist (DASH-00 scope)

- [x] No pipe-to-shell installers executed; no similarly-named package substitution (`npx antislop` explicitly avoided per upstream FAQ).
- [x] Upstream README/license/side-effect review recorded (§2) BEFORE installs.
- [x] Local content scan of installed files: no network imports in shipped Python (`antislop-human` contrast checkers and `ui-ux-pro-max` scripts verified); the only executable payloads beyond Python are `brainstorming`'s local visual-companion server scripts (node/shell) — committed as content, **never executed by this workstream** (§2 superpowers row); everything else is markdown/data.
- [x] Project-local scope only: no writes to `~/.agents/skills`, no `~/.config/antislop/settings.json`, `~/.claude` untouched (its pre-existing content predates this session). Global mutation check: `ls ~/.agents/skills | grep ^antislop` → none.
- [x] No secrets introduced: installed content contains no credentials; secret-scan CI contract unaffected.
- [x] Repo skill-lint contract intact: 33/33 PASS after installs (was 19).
- [x] Trading-safety surface untouched: no crate, config, gate, or mission file modified; `LIVE_TRADING_ENABLED=false` unchanged; working tree changes limited to `.agents/skills/`, `.claude/skills/`, `skills-lock.json`, and `docs/`.

## 6. DASH-00 acceptance gate — self-assessment

Gate (verbatim from the goal plan): *"recorded evidence that AntiSlop core + relevant skills were really installed and loaded before UI code; UI/UX Pro Max and Superpowers resolved or explicitly blocked; relevant licenses/pins reviewed; no secret leak and no unapproved global changes. Save `docs/dashboard/TOOLING_AUDIT.md` and update `docs/DOC_INDEX.md` if committed."*

| Clause | Status | Evidence |
|---|---|---|
| AntiSlop core + relevant skills really installed and loaded before UI code | **MET** | §3.1 pins; §3.3 mode activation + direct content read; zero UI code exists in the repo (no `package.json`, no frontend files) |
| UI/UX Pro Max resolved | **MET** | project-local install + pin (§3.1); content verified |
| Superpowers resolved | **MET** | 7 required workflows project-local + pinned (§3.1, §3.4) |
| Relevant licenses/pins reviewed | **MET** | §2 license table; §3.1 double pin layer |
| No secret leak | **MET** | §5 scans |
| No unapproved global changes | **MET** | §5 global-mutation checks |
| `docs/dashboard/TOOLING_AUDIT.md` saved | **MET** | this file |
| `docs/DOC_INDEX.md` updated | **MET** | registered in the same commit |

## 7. What is NOT established by DASH-00

- No dashboard code, design system, data contract, API, or UI exists yet — those are DASH-01..DASH-07.
- No runtime dependency (React/Radix/etc.) has been installed; §4 is a review, not an installation.
- AntiSlop's Delivery Gate has not yet been exercised against a real UI deliverable (first applicable at DASH-03).
- The installed skills' effect on future agent sessions is verified by content read + repo lint, not by a fresh-session injection test in this same session (skills load at session start; next session inherits them from the project directory).
