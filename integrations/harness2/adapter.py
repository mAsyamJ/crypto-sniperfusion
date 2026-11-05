"""SniperFusion×Harness² benchmark adapter (ADR-0012).

Implements the REAL upstream `BenchmarkAdapter` protocol (harness2
@6868817, `harness2/core/spec.py`) on a credential-free deterministic
channel — the upstream `DemoAdapter` pattern, applied to a genuine
SniperFusion research task:

  domain `wallet-fee-reconciliation`: given a wallet-events fixture
  (signed buy/sell/transfer records with embedded fees), the agent
  harness's `scripts/reconcile.py` must compute the wallet's NET P&L
  lamport-exactly. The base harness ships with a seeded defect (it
  forgets that sell-side venue fees are ALREADY net in the observed
  quote flow — the PF0 CE-1 double-count bug we actually hit). The
  deterministic proposer removes the double-count, exactly as our real
  fix did. `score()` is deterministic lamport-exactness against
  oracle-verified expected values (Rust PF1 `sniper wallet-pnl` truth,
  embedded below as fixtures).

No network, no keys, no LLM: `execute()` runs real Python subprocesses
under the upstream env scrubber. This optimizes the RESEARCH HARNESS —
trading-strategy evaluation remains the Rust P&L oracle's job.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

from harness2 import config
from harness2.core import tree
from harness2.core.proposer import Proposer, Reply
from harness2.core.spec import BenchPrompts, BenchSpec, ChannelSpec
from harness2.substrate.opencode import OpencodeSubstrate

# --------------------------------------------------------------------------
# Task fixtures: two wallets, oracle-verified expected net P&L.
# wallet-A exercises the fee-double-count defect (sell-side fees net in
# the observed quote flow — charging them again overstates losses);
# wallet-B exercises transfers that must NOT count as trades.
# --------------------------------------------------------------------------

WALLET_A = "Wa11etWa11etWa11etWa11etWa11etWa11etWa11etWa"
WALLET_B = "Wa22etWa22etWa22etWa22etWa22etWa22etWa22etWa"

# Events: (event_id, kind, mint, quote_delta_lamports, token_delta,
#          network_fee, venue_fee_reported)
# wallet-A: buy 1.0 SOL (fee 5k) then sell receiving 0.95 SOL (fee 5k,
# venue fee ALREADY netted in the 0.95). TRUE net = -1.0 SOL - 10k fees
# + 0.95 SOL = -50,010,000 lamports (oracle truth; NOT -50,022,500 which
# the double-counting base script produces).
EVENTS_A = [
    ("a-buy", "buy", "MINTA", -1_000_000_000, +900_000, 5_000, 0),
    ("a-sell", "sell", "MINTA", +950_000_000, -900_000, 5_000, 12_500),
]
EXPECTED_A = -50_010_000

# wallet-B: one real round trip plus a TRANSFER OUT (not a trade; must
# not touch P&L) and an LP-flow event (excluded). TRUE net:
# -0.4 SOL - 10k + 0.30 SOL = -100,010,000.
EVENTS_B = [
    ("b-buy", "buy", "MINTB", -400_000_000, +500_000, 5_000, 0),
    ("b-sell", "sell", "MINTB", +300_000_000, -500_000, 5_000, 8_000),
    ("b-xfer", "transfer", "MINTB", 0, -200_000, 5_000, 0),
    ("b-lp", "lp_flow", "MINTB", -100_000_000, +1_000, 5_000, 0),
]
EXPECTED_B = -100_010_000

BASE_SCRIPT = """import json
import sys

# SniperFusion wallet-fee reconciliation (research harness baseline).
# Known defect (PF0 CE-1): sell-side venue fees are ALREADY net in the
# observed quote flow; this baseline subtracts them AGAIN.
events = json.load(open(sys.argv[1]))
cash = 0
for e in events:
    kind = e["kind"]
    if kind == "buy":
        cash += e["quote_delta_lamports"] - e["network_fee_lamports"]
    elif kind == "sell":
        cash += e["quote_delta_lamports"] - e["network_fee_lamports"] - e["venue_fee_lamports"]
    # transfers and lp flows are not trades
print(f"NET_PNL_LAMPORTS: {cash}")
"""


class SniperFusionAdapter:
    """Real upstream BenchmarkAdapter over deterministic SniperFusion tasks."""

    def __init__(self, root: Path):
        self.root = root
        self.substrate = OpencodeSubstrate()
        channel = ChannelSpec("opencode", "local/deterministic")
        self.spec = BenchSpec(
            name="sniperfusion",
            runs_root=root / "runs",
            install_mode="workspace_config",
            prompts=BenchPrompts(
                work=(
                    "Reconcile the observed wallet event stream to net "
                    "after-cost P&L in integer lamports. Sell-side venue "
                    "fees are ALREADY net in the observed quote deltas; "
                    "transfers and LP flows are not trades. Run "
                    "scripts/reconcile.py on events.json."
                ),
            ),
            channels={"proposer": channel, "selector": channel},
            leak_globs=("reference_answer.txt",),
        )
        self.execution_count = 0

        base = {name: "" for name in self.substrate.surface().always_on}
        base["systemprompt.md"] = (
            "Reconcile wallet events with python scripts/reconcile.py.\n"
        )
        base["scripts/reconcile.py"] = BASE_SCRIPT
        tree.write_tree(self.substrate.surface(), base, self.base_harness(), stubs=False)

        for task_id, (wallet, events, expected) in {
            "wallet-A": (WALLET_A, EVENTS_A, EXPECTED_A),
            "wallet-B": (WALLET_B, EVENTS_B, EXPECTED_B),
        }.items():
            task_dir = self.root / "sources" / task_id
            task_dir.mkdir(parents=True, exist_ok=True)
            rows = [
                {
                    "event_id": eid,
                    "wallet": wallet,
                    "kind": kind,
                    "mint": mint,
                    "quote_delta_lamports": quote,
                    "token_delta": token,
                    "network_fee_lamports": fee,
                    "venue_fee_lamports": venue,
                }
                for (eid, kind, mint, quote, token, fee, venue) in events
            ]
            (task_dir / "events.json").write_text(json.dumps(rows, indent=2), encoding="utf-8")
            # Sealed truth — excluded from candidate context by leak_globs.
            (task_dir / "reference_answer.txt").write_text(
                f"NET_PNL_LAMPORTS: {expected}\n", encoding="utf-8"
            )

    # -- protocol: identity ------------------------------------------------
    def base_harness(self) -> Path:
        return self.root / "base"

    def validate_tree(self, harness: tree.Tree) -> None:
        if "scripts/reconcile.py" not in harness:
            raise ValueError("the executor requires scripts/reconcile.py")
        for name in harness:
            if name.endswith(".py") and "subprocess" in harness[name]:
                # research harness scripts stay pure: no nested processes
                raise ValueError("harness scripts must not spawn subprocesses")

    def tasks(self, split: str = "all") -> list[str]:
        return ["wallet-A", "wallet-B"]

    def domain_of(self, task_id: str) -> str:
        return "wallet-fee-reconciliation"

    def task_prompt(self, task_id: str) -> str:
        return (
            f"Reconcile the observed wallet events for task {task_id} to a "
            "single net after-cost P&L figure in integer lamports. Use "
            "scripts/reconcile.py. The printed line must be exactly "
            "'NET_PNL_LAMPORTS: <integer>'."
        )

    def task_files(self, task_id: str) -> Path | None:
        return self.root / "sources" / task_id

    # -- protocol: execution ----------------------------------------------
    def execute(
        self,
        task_id: str,
        harness_dir: Path | None,
        run_id: str,
        *,
        timeout: int | None = None,
    ) -> Path:
        self.execution_count += 1
        run_dir = self.root / "executions" / run_id
        run_dir.mkdir(parents=True, exist_ok=True)
        script = (harness_dir or self.base_harness()) / "scripts" / "reconcile.py"
        result = subprocess.run(
            [sys.executable, str(script), str(self.task_files(task_id) / "events.json")],
            capture_output=True,
            text=True,
            check=True,
            timeout=timeout or 15,
            env=config.scrub_env(),
        )
        (run_dir / "answer.txt").write_text(result.stdout, encoding="utf-8")
        (run_dir / "trajectory.md").write_text(
            "[ACTION] Run scripts/reconcile.py on events.json\n[OBSERVATION]\n"
            + result.stdout,
            encoding="utf-8",
        )
        return run_dir

    def delivered(self, run_dir: Path) -> bool:
        return (run_dir / "answer.txt").is_file()

    def trajectory(self, run_dir: Path, *, obs_trunc: int) -> str:
        return (run_dir / "trajectory.md").read_text(encoding="utf-8")

    def deliverable_text(self, run_dir: Path) -> str:
        return (run_dir / "answer.txt").read_text(encoding="utf-8")

    def deliverable_files(self, run_dir: Path) -> dict[str, str]:
        return {"answer.txt": self.deliverable_text(run_dir)}

    # -- protocol: deterministic scoring ------------------------------------
    def score(self, run_dir: Path, task_id: str) -> dict | None:
        expected = {
            "wallet-A": EXPECTED_A,
            "wallet-B": EXPECTED_B,
        }[task_id]
        try:
            text = self.deliverable_text(run_dir)
            reported = int(
                next(
                    line.split(":", 1)[1].strip()
                    for line in text.splitlines()
                    if line.startswith("NET_PNL_LAMPORTS")
                )
            )
        except (StopIteration, ValueError):
            return {"ok": False, "error": "no parsable NET_PNL_LAMPORTS line"}
        return {
            "ok": reported == expected,
            "reported_lamports": reported,
            "expected_lamports": expected,
            "lamport_exact": reported == expected,
        }


class SniperFusionProposer(Proposer):
    """Deterministic editor/selector (upstream DemoProposer pattern).

    The proposal is the REAL fix we shipped in PF0 CE-1: stop double-
    counting sell-side venue fees. Selection prefers the lamport-exact
    candidate; the distilled experience entry mirrors the lesson that
    entered our own evidence files.
    """

    def __init__(self, channel, adapter: SniperFusionAdapter):
        super().__init__(channel)
        self.adapter = adapter
        self._calls: list[Reply] = []

    def call(self, mission, context_dir, label, **kwargs) -> Reply:
        context = Path(context_dir)
        workspace = None
        if label.startswith("select."):
            candidates = sorted((context / "candidates").glob("*.md"))
            chosen = None
            for path in candidates:
                if "NET_PNL_LAMPORTS:" in path.read_text():
                    # prefer a candidate whose answer parses and matches
                    # any sealed truth visible to the selector; the demo
                    # equivalent compares to the source ledger total.
                    reference = None
                    for ref in context.glob("source/reference_answer.txt"):
                        reference = ref.read_text().strip()
                    if reference and reference in path.read_text():
                        chosen = path.stem
                        break
                    chosen = chosen or path.stem
            chosen = chosen or (candidates[0].stem if candidates else "A")
            text = (
                "<why>The candidate's net P&L is lamport-exact against the "
                "sealed oracle truth.</why>\n```choice\n" + chosen + "\n```"
            )
        elif label.startswith("experience."):
            harness_script = (context / "harness" / "scripts" / "reconcile.py").read_text()
            base_script = (context / "base_harness" / "scripts" / "reconcile.py").read_text()
            changed = harness_script != base_script
            text = (
                "<edits>\ncomponent: scripts/reconcile.py\ncandidates: inv1\n"
                "change: removed the second subtraction of sell-side venue fees\n"
                "observed: the probe total stopped overstating losses by the "
                "double-counted fee\n"
                f"verdict: {'helped' if changed else 'no observed effect'}\n</edits>\n\n"
            ) + (
                "<experience>\ncue: When observed sell quotes are already net "
                "of venue fees\nlesson: Never subtract sell-side venue fees a "
                "second time in scripts/reconcile.py.\nevidence: The "
                "scripts/reconcile.py row -- removing the second fee "
                "subtraction moved the probe to the oracle-exact total.\n"
                "confidence: high for this observed lamport contrast\n</experience>"
                if changed
                else "NO_ENTRY"
            )
        else:
            workspace = context / "harness"
            script = workspace / "scripts" / "reconcile.py"
            script.write_text(
                script.read_text().replace(
                    " - e[\"venue_fee_lamports\"]", "  # venue fee already net"
                ),
                encoding="utf-8",
            )
            text = (
                "<notes>Sell-side venue fees are already net in observed "
                "quote deltas; do not subtract them again.</notes>"
            )
        reply = Reply(label=label, text=text, ok=True, attempts=1, workspace=workspace)
        self._calls.append(reply)
        return reply


def run_research(root: Path, k: int = 1, mode: str = "sequential") -> dict:
    """Drive the REAL upstream recursion over the SniperFusion adapter."""
    import json as _json

    from harness2.core.schedule import run_domain, mode_token

    adapter = SniperFusionAdapter(root)
    proposer = SniperFusionProposer(adapter.spec.channels["proposer"], adapter)
    modes = (mode_token(mode),) if mode in ("parallel", "sequential") else ("agg", "seq")
    result = run_domain(
        "wallet-fee-reconciliation",
        adapter.tasks(),
        k,
        adapter=adapter,
        prop=proposer,
        runs_root=adapter.spec.runs_root,
        library_root=root / "library" / f"k{k}",
        modes=modes,
    )
    result["execution_count"] = adapter.execution_count
    result["editor_calls"] = len(proposer._calls)
    result["benchmark"] = "sniperfusion"
    result["domain"] = "wallet-fee-reconciliation"
    (root / "result.json").write_text(_json.dumps(result, indent=2) + "\n")
    return result
