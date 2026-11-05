"""Offline tests for the bounded creation-cohort collector (H6-2).

All RPC behavior is faked; no network access. These tests pin: pagination,
versioned create decoding, block-level ordering, duplicate handling, missing
metadata, skipped slots, rate limits, honest coverage states, genesis
verification, write-once outputs, and safe-env refusal.
"""

import base64
import json
import os
import struct
import sys
import tempfile
import unittest
import urllib.error
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import launch_cohort_capture as lcc

PUMP = lcc.PUMP_PROGRAM_ID
GENESIS = lcc.MAINNET_GENESIS


def build_create_payload(
    name="Coin",
    symbol="CN",
    uri="https://x/y.json",
    mint=None,
    curve=None,
    user=None,
    creator=None,
    timestamp=1_700_000_000,
    trailing=True,
):
    mint = mint or bytes(range(1, 33))
    curve = curve or bytes(range(33, 65))
    user = user or bytes(range(65, 97))
    creator = creator or bytes(range(97, 129))

    def string(s):
        b = s.encode()
        return struct.pack("<I", len(b)) + b

    body = string(name) + string(symbol) + string(uri)
    body += mint + curve + user + creator
    body += struct.pack("<q", timestamp)
    if trailing:
        body += struct.pack("<Q", 1_073_000_000_000_000)  # virtual token reserves
        body += struct.pack("<Q", 30_000_000_000)  # virtual sol reserves
        body += struct.pack("<Q", 793_100_000_000_000)  # real token reserves
        body += struct.pack("<Q", 1_000_000_000_000_000)  # total supply
        body += bytes(range(129, 161))  # token program
        body += b"\x00"  # mayhem
        body += b"\x01"  # cashback
        body += bytes(range(161, 193))  # quote mint
        body += struct.pack("<Q", 30_000_000_000)  # virtual quote reserves
        body += struct.pack("<Q", 50)  # creator fee bps
        body += b"\x00"  # holder reward
    return bytes(lcc.DISC_CREATE_EVENT) + body


def make_create_tx(signature, variant="create", failed=False, mint_bytes=None, trailing=True):
    instruction = "Create" if variant == "create" else "CreateV2"
    payload = build_create_payload(trailing=trailing, mint=mint_bytes)
    logs = [
        f"Program {PUMP} invoke [1]",
        f"Program log: Instruction: {instruction}",
        "Program data: " + base64.b64encode(payload).decode(),
        f"Program {PUMP} success",
    ]
    return {
        "transaction": {"signatures": [signature], "message": {"accountKeys": ["FeeKeEP9aaaa", PUMP, "MintACCaaaa"], "instructions": []}},
        "meta": {
            "err": {"InstructionError": [0, "Custom(1)"]} if failed else None,
            "logMessages": logs,
            "loadedAddresses": [],
        },
    }


def make_block(slot, txs, blockhash="Bl0k"):
    return {
        "slot": slot,
        "blockhash": blockhash,
        "transactions": txs,
    }


class FakeRpc:
    """Duck-typed stand-in for JsonRpc with a tiny in-memory chain."""

    def __init__(self, blocks=None, sig_pages=None, genesis=GENESIS, tip=2000,
                 null_block_slots=(), rate_limit_slots=(), missing_sigs=()):
        self.blocks = {b["slot"]: b for b in (blocks or [])}
        self.sig_pages = list(sig_pages or [])
        self.genesis = genesis
        self.tip = tip
        self.null_block_slots = set(null_block_slots)
        self.rate_limit_slots = set(rate_limit_slots)
        self.missing_sigs = set(missing_sigs)
        self.calls = []
        self._page = 0

    def get_genesis_hash(self):
        return self.genesis

    def get_slot(self, commitment=None):
        return self.tip

    def get_signatures_for_address(self, address, before=None, limit=1000):
        self.calls.append(("sigs", before))
        if self._page >= len(self.sig_pages):
            return []
        page = self.sig_pages[self._page]
        self._page += 1
        return page

    def get_block(self, slot):
        self.calls.append(("block", slot))
        if slot in self.rate_limit_slots:
            raise lcc.RpcError("HTTP 429", code=429)
        if slot in self.null_block_slots:
            return None
        return self.blocks.get(slot)

    def get_transaction(self, signature):
        self.calls.append(("tx", signature))
        if signature in self.missing_sigs:
            return None
        for block in self.blocks.values():
            for tx in block["transactions"]:
                if tx["transaction"]["signatures"][0] == signature:
                    return {"slot": block["slot"], "meta": tx["meta"],
                            "transaction": tx["transaction"]}
        return None


def capture_tmp_dir():
    return tempfile.mkdtemp(prefix="lcc-test-")


class EnvSafety(unittest.TestCase):
    def test_refuses_live_trading_and_key_envs(self):
        with mock.patch.dict(os.environ, {"LIVE_TRADING_ENABLED": "true"}):
            self.assertRaises(lcc.EnvRefused, lcc.require_safe_env)
        with mock.patch.dict(os.environ, {"LIVE_TRADING_ENABLED": "false", "PRIVATE_KEY": "x"}):
            self.assertRaises(lcc.EnvRefused, lcc.require_safe_env)
        with mock.patch.dict(os.environ, {"LIVE_TRADING_ENABLED": "false"}, clear=False):
            os.environ.pop("PRIVATE_KEY", None)
            os.environ.pop("SOLANA_PRIVATE_KEY", None)
            lcc.require_safe_env()  # ok


class Base58(unittest.TestCase):
    def test_known_vectors(self):
        self.assertEqual(lcc.b58encode(b"hello"), "Cn8eVZg")
        self.assertEqual(lcc.b58encode(b"\x00\x00"), "11")


class WireParams(unittest.TestCase):
    """Mission-A regression: the collector MUST request transaction
    version 1 on getBlock/getTransaction (version 0 made every H6 live
    manifest fail with UnsupportedTxVersion on versioned blocks)."""

    class _Capture(JsonRpc if False else object):
        pass

    def _capturing_client(self):
        import launch_cohort_capture as lcc

        captured = []

        def transport(body):
            captured.append(body)
            if body["method"] == "getGenesisHash":
                return {"result": lcc.MAINNET_GENESIS}
            if body["method"] == "getSlot":
                return {"result": 2000}
            if body["method"] == "getBlock":
                return {"result": None}
            return {"result": []}

        client = lcc.JsonRpc(url="https://x", transport=transport)
        return client, captured

    def test_get_block_and_tx_request_version_1(self):
        client, captured = self._capturing_client()
        client.get_block(123)
        client.get_transaction("Sig")
        blocks = [b for b in captured if b["method"] == "getBlock"]
        txs = [b for b in captured if b["method"] == "getTransaction"]
        self.assertTrue(blocks and blocks[0]["params"][1]["maxSupportedTransactionVersion"] == 1)
        self.assertTrue(txs and txs[0]["params"][1]["maxSupportedTransactionVersion"] == 1)


class HttpTransport(unittest.TestCase):
    def test_http_429_becomes_rate_limited_rpc_error(self):
        client = lcc.JsonRpc(url="https://example.invalid")

        def raise_429(req, timeout=None):
            raise urllib.error.HTTPError(req.full_url, 429, "Too Many Requests", hdrs=None, fp=None)

        with mock.patch.object(urllib.request, "urlopen", side_effect=raise_429):
            with self.assertRaises(lcc.RpcError) as ctx:
                client.get_block(123)
        self.assertEqual(ctx.exception.code, 429)


class DecodeCreateEvent(unittest.TestCase):
    def test_full_layout_decodes_all_nineteen_fields(self):
        payload = build_create_payload()
        event = lcc.decode_create_event(payload[8:])
        self.assertEqual(event["name"], "Coin")
        self.assertEqual(event["symbol"], "CN")
        self.assertEqual(event["uri"], "https://x/y.json")
        self.assertEqual(event["timestamp"], 1_700_000_000)
        self.assertEqual(event["decoded_fields"], 19)
        self.assertFalse(event["is_truncated"])

    def test_truncated_prefix_is_kept_not_dropped(self):
        payload = build_create_payload(trailing=False)
        event = lcc.decode_create_event(payload[8:])
        self.assertEqual(event["decoded_fields"], 8)
        self.assertTrue(event["is_truncated"])
        self.assertEqual(event["virtual_token_reserves"], 0)

    def test_garbage_returns_none(self):
        self.assertIsNone(lcc.decode_create_event(b"\x00" * 5))


class InstructionVariant(unittest.TestCase):
    def test_create_and_create_v2_variants_are_recorded(self):
        logs_v1 = make_create_tx("SigA", variant="create")["meta"]["logMessages"]
        logs_v2 = make_create_tx("SigB", variant="create_v2")["meta"]["logMessages"]
        events = lcc.extract_creates_from_logs(logs_v1, PUMP)
        self.assertEqual(len(events), 1)
        self.assertEqual(events[0]["instruction_variant"], "create")
        events2 = lcc.extract_creates_from_logs(logs_v2, PUMP)
        self.assertEqual(events2[0]["instruction_variant"], "create_v2")

    def test_non_create_program_data_is_ignored(self):
        logs = [
            f"Program {PUMP} invoke [1]",
            "Program log: Instruction: Buy",
            "Program data: " + base64.b64encode(build_create_payload()).decode(),
            f"Program {PUMP} success",
        ]
        self.assertEqual(lcc.extract_creates_from_logs(logs, PUMP), [])

    def test_multiple_creates_get_ordinals(self):
        logs = make_create_tx("SigA")["meta"]["logMessages"]
        logs = logs[:-1] + [
            "Program data: " + base64.b64encode(build_create_payload(mint=bytes([9] * 32))).decode(),
            f"Program {PUMP} success",
        ]
        events = lcc.extract_creates_from_logs(logs, PUMP)
        self.assertEqual(len(events), 2)
        self.assertEqual([e["event_ordinal"] for e in events], [0, 1])


class CaptureFlow(unittest.TestCase):
    def setUp(self):
        self.dir = capture_tmp_dir()
        self.txs_100 = [
            make_create_tx("SigCreate100"),
            make_create_tx("SigTrade100x", variant="create", failed=False, mint_bytes=bytes([5] * 32)),
        ]
        # Second tx has same shape; give it non-create logs to prove only
        # create instructions are captured.
        self.txs_100[1]["meta"]["logMessages"] = [
            f"Program {PUMP} invoke [1]",
            "Program log: Instruction: Buy",
            f"Program {PUMP} success",
        ]
        self.txs_101 = [make_create_tx("SigCreate101", variant="create_v2")]

    def capture(self, rpc, from_slot=100, to_slot=101, **kw):
        return lcc.capture(rpc, from_slot, to_slot, out_dir=self.dir, sleep=lambda s: None, **kw)

    def test_happy_path_complete_with_block_ordering(self):
        rpc = FakeRpc(
            blocks=[make_block(100, self.txs_100), make_block(101, self.txs_101)],
            sig_pages=[[
                {"signature": "SigCreate101", "slot": 101, "err": None},
                {"signature": "SigCreate100", "slot": 100, "err": None},
            ]],
        )
        report = self.capture(rpc)
        self.assertEqual(report["coverage_state"], "COMPLETE")
        self.assertEqual(report["blocks_fetched"], 2)
        creates = report["creates"]
        self.assertEqual(len(creates), 2)
        by_sig = {c["signature"]: c for c in creates}
        self.assertEqual(by_sig["SigCreate100"]["transaction_index"], 0)
        self.assertEqual(by_sig["SigCreate101"]["transaction_index"], 0)
        self.assertEqual(by_sig["SigCreate100"]["instruction_variant"], "create")
        self.assertEqual(by_sig["SigCreate101"]["instruction_variant"], "create_v2")
        self.assertEqual(report["reconciliation"]["signatures_without_block_evidence"], [])
        self.assertTrue(report["nonpromotable"])

    def test_pagination_uses_cursor_and_stops_at_window(self):
        pages = [
            [{"signature": "SigNew", "slot": 105, "err": None},
             {"signature": "SigCreate101", "slot": 101, "err": None}],
            [{"signature": "SigCreate100", "slot": 100, "err": None},
             {"signature": "SigOld", "slot": 99, "err": None}],
            [{"signature": "SigStale", "slot": 90, "err": None}],
        ]
        rpc = FakeRpc(blocks=[make_block(100, self.txs_100), make_block(101, self.txs_101)], sig_pages=pages)
        report = self.capture(rpc)
        sig_calls = [c for c in rpc.calls if c[0] == "sigs"]
        # First page has no cursor; the second continues after the oldest
        # in-window signature of page one; pagination stopped at the first
        # page fully below the window floor.
        self.assertEqual(sig_calls[0][1], None)
        self.assertEqual(sig_calls[1][1], "SigCreate101")
        self.assertLessEqual(len(sig_calls), 3)
        self.assertEqual(report["coverage_state"], "COMPLETE")

    def test_duplicate_provider_events_are_deduplicated(self):
        tx = make_create_tx("SigDup")
        # Same signature surfaced twice in the signature feed AND once in the
        # block: exactly one create record.
        dup_logs = list(tx["meta"]["logMessages"])
        tx["meta"]["logMessages"] = dup_logs
        rpc = FakeRpc(
            blocks=[make_block(100, [tx])],
            sig_pages=[
                [{"signature": "SigDup", "slot": 100, "err": None}],
                [{"signature": "SigDup", "slot": 100, "err": None}],
            ],
        )
        report = self.capture(rpc, from_slot=100, to_slot=100)
        self.assertEqual(len(report["creates"]), 1)

    def test_failed_transactions_yield_no_creates(self):
        tx = make_create_tx("SigFailed", failed=True)
        rpc = FakeRpc(blocks=[make_block(100, [tx])], sig_pages=[[{"signature": "SigFailed", "slot": 100, "err": {}}]])
        report = self.capture(rpc, from_slot=100, to_slot=100)
        self.assertEqual(report["creates"], [])
        self.assertEqual(report["coverage_state"], "COMPLETE")

    def test_missing_block_metadata_is_explicit_gap(self):
        rpc = FakeRpc(
            blocks=[make_block(100, self.txs_100)],
            sig_pages=[[{"signature": "SigCreate101", "slot": 101, "err": None}]],
            null_block_slots={101},
        )
        report = self.capture(rpc)
        self.assertEqual(report["coverage_state"], "INCOMPLETE")
        kinds = [g["kind"] for g in report["gaps"]]
        self.assertIn("ProviderUnavailable", kinds)

    def test_skipped_slots_recorded(self):
        rpc = FakeRpc(blocks=[make_block(100, self.txs_100)], null_block_slots={101})
        report = self.capture(rpc)
        self.assertEqual(report["coverage_state"], "INCOMPLETE")

    def test_rate_limited_block_becomes_gap_after_bounded_retries(self):
        rpc = FakeRpc(
            blocks=[make_block(100, self.txs_100)],
            rate_limit_slots={101},
        )
        report = self.capture(rpc, retries=2)
        self.assertEqual(report["coverage_state"], "INCOMPLETE")
        kinds = [g["kind"] for g in report["gaps"]]
        self.assertIn("RateLimited", kinds)
        block_attempts = [c for c in rpc.calls if c == ("block", 101)]
        self.assertEqual(len(block_attempts), 3)  # initial + 2 bounded retries

    def test_signature_without_block_evidence_is_a_gap_not_silence(self):
        rpc = FakeRpc(
            blocks=[make_block(100, self.txs_100), make_block(101, self.txs_101)],
            sig_pages=[[{"signature": "SigGhost", "slot": 101, "err": None}]],
            missing_sigs={"SigGhost"},
        )
        report = self.capture(rpc)
        self.assertEqual(report["coverage_state"], "INCOMPLETE")
        self.assertIn("SigGhost", report["reconciliation"]["signatures_without_block_evidence"])
        kinds = [g["kind"] for g in report["gaps"]]
        self.assertTrue(any(k.startswith("MissingMetadata") for k in kinds))

    def test_recovered_transaction_creates_are_flagged_not_fabricated(self):
        # Provider cannot serve blocks (Infura-style getBlock refusal) but
        # getTransaction works: creates recovered from logs carry
        # block_evidence=False and transaction_index None; coverage stays
        # INCOMPLETE and a NoBlockEvidence gap remains.
        tx = make_create_tx("SigRecover")
        recovered = {"slot": 100, "meta": tx["meta"], "transaction": tx["transaction"]}
        rpc = FakeRpc(blocks=[], sig_pages=[[{"signature": "SigRecover", "slot": 100, "err": None}]])
        rpc.get_transaction = lambda signature: recovered
        report = self.capture(rpc, from_slot=100, to_slot=100)
        self.assertEqual(len(report["creates"]), 1)
        create = report["creates"][0]
        self.assertFalse(create["block_evidence"])
        self.assertIsNone(create["transaction_index"])
        self.assertEqual(create["signature"], "SigRecover")
        self.assertEqual(report["coverage_state"], "INCOMPLETE")
        kinds = [g["kind"] for g in report["gaps"]]
        self.assertIn("NoBlockEvidence:SigRecover", kinds)

    def test_window_past_tip_is_right_censored(self):
        rpc = FakeRpc(blocks=[make_block(100, self.txs_100)], tip=100)
        report = self.capture(rpc, from_slot=100, to_slot=150)
        self.assertEqual(report["coverage_state"], "CENSORED")

    def test_unknown_when_no_evidence_at_all(self):
        rpc = FakeRpc(blocks=[], sig_pages=[[]])
        report = self.capture(rpc, from_slot=100, to_slot=101)
        self.assertEqual(report["coverage_state"], "UNKNOWN")

    def test_wrong_genesis_refuses_before_any_market_read(self):
        rpc = FakeRpc(blocks=[make_block(100, self.txs_100)], genesis="WrongGenesisHash")
        self.assertRaises(lcc.WrongCluster, self.capture, rpc)
        market_reads = [c for c in rpc.calls if c[0] in ("sigs", "block", "tx")]
        self.assertEqual(market_reads, [])

    def test_window_bounds_validated(self):
        rpc = FakeRpc()
        self.assertRaises(ValueError, self.capture, rpc, from_slot=200, to_slot=100)
        self.assertRaises(ValueError, self.capture, rpc, from_slot=0, to_slot=100000)

    def test_recent_slots_window_is_tip_relative(self):
        rpc = FakeRpc(
            blocks=[make_block(998, []), make_block(999, [])],
            sig_pages=[[]],
            tip=999,
        )
        report = lcc.capture_recent_slots(rpc, 1, self.dir, sleep=lambda s: None)
        self.assertEqual(report["manifest"]["window"]["from_slot"], 998)
        self.assertEqual(report["manifest"]["window"]["to_slot"], 999)
        self.assertEqual(report["blocks_fetched"], 2)


class CheckpointResumeBudget(unittest.TestCase):
    def test_resume_skips_verified_slots_without_duplicate_blocks(self):
        out = tempfile.mkdtemp(prefix="lcc-ckpt-")
        ckpt = str(Path(out) / "ckpt.json")
        blocks = [make_block(100, [make_create_tx("SigA")]),
                  make_block(101, [make_create_tx("SigB")])]
        rpc1 = FakeRpc(blocks=blocks, sig_pages=[[{"signature": "SigA", "slot": 100, "err": None},
                                                  {"signature": "SigB", "slot": 101, "err": None}]],
                       tip=101)
        # First run: budget stops after the signature page + first block.
        lcc.capture(rpc1, 100, 101, out_dir=out, sleep=lambda s: None,
                    checkpoint=ckpt, max_requests=4)
        # RPC call order: genesis, slot, sigs(page), block(100) = 4 -> stop
        first_blocks = [c for c in rpc1.calls if c[0] == "block"]
        self.assertEqual(first_blocks, [("block", 100)])
        state = json.loads(Path(ckpt).read_text())
        self.assertEqual(state["collected_to_slot"], 101)
        # Resume: genesis+slot+ (no re-fetch of slot 100) + block 101.
        rpc2 = FakeRpc(blocks=blocks, tip=101)
        report = lcc.capture(rpc2, 100, 101, out_dir=out, sleep=lambda s: None,
                             checkpoint=ckpt)
        second_blocks = [c for c in rpc2.calls if c[0] == "block"]
        self.assertEqual(second_blocks, [("block", 101)], "resume must not re-fetch verified slots")
        self.assertEqual(report["coverage_state"], "COMPLETE")
        self.assertEqual(len(report["creates"]), 2)
        self.assertTrue(Path(ckpt).exists())

    def test_budget_stop_report_carries_summary_keys_regression(self):
        """Regression (2026-11-05): budget-stop reports must satisfy the same
        summary extraction main() applies to completed runs — previously a
        budget stop crashed main() with KeyError 'blocks_fetched'."""
        out = tempfile.mkdtemp(prefix="lcc-budgetreg-")
        ckpt = str(Path(out) / "ckpt.json")
        blocks = [make_block(100, [make_create_tx("SigA")]),
                  make_block(101, [make_create_tx("SigB")])]
        rpc = FakeRpc(blocks=blocks,
                      sig_pages=[[{"signature": "SigA", "slot": 100, "err": None},
                                  {"signature": "SigB", "slot": 101, "err": None}]],
                      tip=101)
        # genesis, slot, sigs(page), block(100) = 4 -> stop before block 101
        report = lcc.capture(rpc, 100, 101, out_dir=out, sleep=lambda s: None,
                             checkpoint=ckpt, max_requests=4)
        self.assertEqual(report["status"], "BUDGET_EXHAUSTED_RESUMABLE")
        # exact main() extraction contract — must not raise
        summary = {k: report[k] for k in ("coverage_state", "blocks_fetched",
                                          "pages_fetched", "raw_path",
                                          "manifest_path", "nonpromotable")}
        summary["creates"] = len(report["creates"])
        summary["gaps"] = len(report["gaps"])
        self.assertEqual(summary["coverage_state"], "INCOMPLETE")
        self.assertGreaterEqual(summary["blocks_fetched"], 1,
                                "progress-so-far from the checkpoint")
        self.assertEqual(report["checkpoint"], ckpt)
        self.assertIn("collected_to_slot", report)

    def test_checkpoint_lock_blocks_concurrent_capture(self):
        """Incident regression (2026-11-05): a zombie sweep left running
        raced a new capture on the same checkpoint and corrupted it. A
        second capture while the lock is held must fail fast."""
        out = tempfile.mkdtemp(prefix="lcc-lock-")
        ckpt = str(Path(out) / "ckpt.json")
        blocks = [make_block(100, [make_create_tx("SigA")])]
        rpc = FakeRpc(blocks=blocks, sig_pages=[[{"signature": "SigA", "slot": 100, "err": None}]],
                      tip=100)
        with lcc._checkpoint_lock(ckpt):
            # flock is per open-fd: a second open in this same process must
            # fail to take the exclusive lock, exactly like another process.
            with self.assertRaises(lcc.CheckpointLocked):
                with lcc._checkpoint_lock(ckpt):
                    pass
            # and a full capture attempt fails fast with the same error
            with self.assertRaises(lcc.CheckpointLocked):
                lcc.capture(rpc, 100, 100, out_dir=out, sleep=lambda s: None,
                            checkpoint=ckpt)
        # lock released on exit: sequential captures both succeed (fresh
        # out_dir for the second run — raw evidence is write-once, so the
        # same out_dir must and does refuse with FileExistsError)
        r1 = lcc.capture(rpc, 100, 100, out_dir=out, sleep=lambda s: None,
                         checkpoint=ckpt)
        self.assertEqual(r1["coverage_state"], "COMPLETE")
        out2 = tempfile.mkdtemp(prefix="lcc-lock2-")
        r2 = lcc.capture(rpc, 100, 100, out_dir=out2, sleep=lambda s: None,
                         checkpoint=ckpt)
        self.assertEqual(r2["coverage_state"], "COMPLETE")

    def test_checkpoint_none_runs_without_lock_files(self):
        out = tempfile.mkdtemp(prefix="lcc-nolock-")
        blocks = [make_block(100, [make_create_tx("SigA")])]
        rpc = FakeRpc(blocks=blocks, sig_pages=[[{"signature": "SigA", "slot": 100, "err": None}]],
                      tip=100)
        report = lcc.capture(rpc, 100, 100, out_dir=out, sleep=lambda s: None)
        self.assertEqual(report["coverage_state"], "COMPLETE")
        self.assertEqual(list(Path(out).glob("*.lock")), [])

    def test_budget_stop_report_is_blocked_and_resumable(self):
        out = tempfile.mkdtemp(prefix="lcc-budget-")
        rpc = FakeRpc(blocks=[make_block(100, [])], tip=100)
        report = lcc.capture(rpc, 100, 100, out_dir=out, sleep=lambda s: None,
                             max_requests=2)  # genesis + slot -> stop before any block
        self.assertEqual(report["status"], "BUDGET_EXHAUSTED_RESUMABLE")
        self.assertEqual(report["coverage_state"], "INCOMPLETE")
        stops = list(Path(out).glob("budget-stop-*.json"))
        self.assertTrue(stops and oct(stops[0].stat().st_mode & 0o777) == oct(0o444))

    def test_rate_limit_backoff_is_exponential_and_capped(self):
        delays = []
        def sleep_fn(d):
            delays.append(d)
            if len(delays) >= 3:
                raise KeyboardInterrupt
        class Always429:
            def __call__(self):
                raise lcc.RpcError("HTTP 429", code=429)
        try:
            lcc.with_retry(Always429(), retries=5, sleep=sleep_fn, sleep_base=0.5)
        except KeyboardInterrupt:
            pass
        self.assertEqual(delays[:3], [0.5, 1.0, 2.0])


class Outputs(unittest.TestCase):
    def test_write_once_outputs_are_immutable_and_content_addressed(self):
        out = capture_tmp_dir()
        txs = [make_create_tx("SigOut1")]
        rpc = FakeRpc(blocks=[make_block(100, txs)], sig_pages=[[{"signature": "SigOut1", "slot": 100, "err": None}]])
        report = lcc.capture(rpc, 100, 100, out_dir=out, sleep=lambda s: None)
        raw = Path(report["raw_path"])
        manifest = Path(report["manifest_path"])
        self.assertTrue(raw.exists() and manifest.exists())
        self.assertEqual(oct(raw.stat().st_mode & 0o777), oct(0o444))
        self.assertEqual(oct(manifest.stat().st_mode & 0o777), oct(0o444))
        # Content addressing: manifest filename derives from the raw file hash.
        import hashlib
        digest = hashlib.sha256(raw.read_bytes()).hexdigest()
        self.assertIn(digest[:16], manifest.name)
        manifest_json = json.loads(manifest.read_text())
        self.assertEqual(manifest_json["dataset_sha256"], digest)
        self.assertEqual(manifest_json["coverage_state"], "COMPLETE")
        self.assertTrue(manifest_json["nonpromotable"])
        # Raw lines parse and preserve never-traded mints.
        lines = [json.loads(l) for l in raw.read_text().splitlines() if l.strip()]
        self.assertEqual(len(lines), 1)
        self.assertEqual(lines[0]["signature"], "SigOut1")
        # Second capture into the same dir with same content refuses overwrite.
        rpc2 = FakeRpc(blocks=[make_block(100, txs)], sig_pages=[[{"signature": "SigOut1", "slot": 100, "err": None}]])
        # Identical timestamp bucket: force same name by reusing paths.
        with mock.patch.object(lcc, "write_once") as wo:
            wo.side_effect = FileExistsError
            self.assertRaises(FileExistsError, lcc.capture, rpc2, 100, 100, out_dir=out, sleep=lambda s: None)


if __name__ == "__main__":
    unittest.main()
