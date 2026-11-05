"""Offline tests for the sampled failed-create diagnostic (D0, 2026-11-05).

The diagnostic previously counted create attempts ONLY for the legacy
`global:create` discriminator, so `create_v2` attempts landed in the generic
histogram and the G0 receipt's "zero create attempts" claim was actually
"zero LEGACY create attempts". These tests pin:

- both creation-family discriminators against the Rust-pinned hex
  (crates/sniper-protocols/src/curve.rs `discriminators_match_idl_derived_constants`),
- instruction-attempt counting (ok/fail, outer+inner) for BOTH families,
- versioned-transaction key resolution: static + ALT writable + readonly in
  canonical index order, so a programIdIndex into the ALT region resolves.

No network access; the RPC path is only exercised via injected blocks.
"""

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import diag_sample_failed_creates as diag
import launch_cohort_capture as lcc  # b58encode reuse; single encoder source

PUMP = diag.PUMP
OTHER = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"


def ix_data(name, extra=b""):
    return lcc.b58encode(diag.discriminator(name) + extra)


class DiscriminatorPins(unittest.TestCase):
    def test_both_create_families_match_rust_pinned_hex(self):
        self.assertEqual(diag.DISC_CREATE.hex(), "181ec828051c0777")
        self.assertEqual(diag.DISC_CREATE_V2.hex(), "d6904cec5f8b31b4")

    def test_discriminator_uses_anchor_global_namespace(self):
        # Anchor convention pinned by curve.rs: sha256("global:<name>")[:8].
        self.assertEqual(diag.discriminator("buy").hex(), "66063d1201daebea")
        self.assertEqual(diag.discriminator("migrate").hex(), "9beae792ec9ea21e")


class ScanBlock(unittest.TestCase):
    def _block(self):
        # tx1: SUCCESSFUL, legacy create as an OUTER instruction, PUMP static.
        ok_tx = {
            "transaction": {
                "signatures": ["SigOk"],
                "message": {
                    "accountKeys": ["FeeKeEP9aaaa", PUMP, "UserACCaaaa1"],
                    "instructions": [{"programIdIndex": 1, "data": ix_data("create")}],
                },
            },
            "meta": {"err": None, "loadedAddresses": {"writable": [], "readonly": []},
                     "innerInstructions": []},
        }
        # tx2: FAILED, create_v2 as an INNER instruction, and the pump
        # program id is ONLY ALT-resident (writable), so its
        # programIdIndex (2) resolves through the ALT region.
        failed_tx = {
            "transaction": {
                "signatures": ["SigFail"],
                "message": {
                    "accountKeys": ["FeeKeEP9bbbb", "UserACCaaaa2"],
                    "instructions": [{"programIdIndex": 0, "data": ix_data("buy")}],
                },
            },
            "meta": {
                "err": {"InstructionError": [0, "Custom(1)"]},
                "loadedAddresses": {"writable": [PUMP], "readonly": []},
                "innerInstructions": [
                    {"index": 0, "instructions": [
                        {"programIdIndex": 2, "data": ix_data("create_v2")},
                    ]},
                ],
            },
        }
        # tx3: successful, but no pump involvement at all.
        other_tx = {
            "transaction": {
                "signatures": ["SigOther"],
                "message": {
                    "accountKeys": ["FeeKeEP9cccc", OTHER],
                    "instructions": [{"programIdIndex": 1, "data": ix_data("create")}],
                },
            },
            "meta": {"err": None, "loadedAddresses": {"writable": [], "readonly": []},
                     "innerInstructions": []},
        }
        return {"slot": 465625744, "blockhash": "Bl0k",
                "transactions": [ok_tx, failed_tx, other_tx]}

    def test_counts_both_create_families_ok_and_fail(self):
        stats = diag.scan_block(self._block())
        self.assertEqual(stats["all_tx"], {"ok": 2, "fail": 1})
        self.assertEqual(stats["pump_tx"], {"ok": 1, "fail": 1})
        self.assertEqual(stats["create_attempts"], {
            "create": {"ok": 1, "fail": 0},
            "create_v2": {"ok": 0, "fail": 1},
        })

    def test_discriminator_histograms_split_by_outcome(self):
        stats = diag.scan_block(self._block())
        self.assertEqual(stats["disc_ok"][diag.DISC_CREATE.hex()], 1)
        self.assertEqual(stats["disc_fail"][diag.DISC_CREATE_V2.hex()], 1)
        # Exactly one successful pump instruction (the legacy create) and
        # exactly one failed pump instruction (the inner create_v2).
        self.assertEqual(sum(stats["disc_ok"].values()), 1)
        self.assertEqual(sum(stats["disc_fail"].values()), 1)

    def test_inner_instructions_are_scanned(self):
        stats = diag.scan_block(self._block())
        # The only create_v2 attempt lives in an INNER instruction of tx2.
        self.assertEqual(stats["create_attempts"]["create_v2"]["fail"], 1)
        self.assertEqual(stats["create_attempts"]["create_v2"]["ok"], 0)

    def test_resolved_keys_canonical_order(self):
        tx = {
            "transaction": {"message": {"accountKeys": ["A", "B"]}},
            "meta": {"loadedAddresses": {"writable": ["W"], "readonly": ["R"]}},
        }
        self.assertEqual(diag.resolved_keys(tx), ["A", "B", "W", "R"])

    def test_module_import_has_no_side_effects(self):
        # The script used to execute its whole CLI body on import (RPC
        # assert + checkpoint read). It must now be importable offline.
        self.assertTrue(callable(diag.scan_block))
        self.assertTrue(callable(diag.main))


if __name__ == "__main__":
    unittest.main()
