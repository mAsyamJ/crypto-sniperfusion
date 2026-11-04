"""Offline tests for the read-only fee-config probe (PF0)."""

import base64
import importlib.util
import json
import struct
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import fee_config_probe as fcp
from launch_cohort_capture import WrongCluster


def fees_bytes(lp, protocol, creator):
    return struct.pack("<QQQ", lp, protocol, creator)


def tier_u128(threshold, lp, protocol, creator):
    return threshold.to_bytes(16, "little") + fees_bytes(lp, protocol, creator)


def fee_config_bytes(flat, tiers, stable, exotic):
    body = b"\x01"  # bump
    body += bytes(range(1, 33))  # admin
    body += flat
    body += struct.pack("<I", len(tiers)) + b"".join(tiers)
    body += struct.pack("<I", len(stable)) + b"".join(stable)
    body += exotic
    return fcp.anchor_account_discriminator("FeeConfig") + body


class FakeRpc:
    def __init__(self, accounts):
        self.accounts = accounts

    def get_genesis_hash(self):
        return fcp.MAINNET_GENESIS

    def get_slot(self):
        return 500

    def get_account_info(self, address):
        raw = self.accounts.get(address)
        if raw is None:
            return {"value": None}
        return {"value": {"owner": "pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ",
                          "lamports": 2_339_280,
                          "data": [base64.b64encode(raw).decode(), "base64"]}}


class DecodeFeeConfig(unittest.TestCase):
    def test_full_layout_decodes_tiers(self):
        raw = fee_config_bytes(
            fees_bytes(20, 5, 0),
            [tier_u128(0, 125, 0, 0), tier_u128(10**17, 50, 25, 25)],
            [],
            fees_bytes(25, 25, 0),
        )
        decoded = fcp.decode_fee_config(raw)
        self.assertTrue(decoded["decode_ok"])
        self.assertEqual(decoded["flat_fees"], {"lp_fee_bps": 20, "protocol_fee_bps": 5, "creator_fee_bps": 0})
        self.assertEqual(len(decoded["fee_tiers"]), 2)
        self.assertEqual(decoded["fee_tiers"][0]["fees"]["lp_fee_bps"], 125)
        self.assertEqual(decoded["fee_tiers"][1]["market_cap_lamports_threshold"], 10**17)

    def test_wrong_discriminator_fails_loudly(self):
        decoded = fcp.decode_fee_config(b"\x00" * 64)
        self.assertFalse(decoded["decode_ok"])
        self.assertIn("discriminator", decoded["reason"])

    def test_truncated_data_reports_reason(self):
        raw = fee_config_bytes(fees_bytes(1, 2, 3), [], [], fees_bytes(0, 0, 0))[:20]
        decoded = fcp.decode_fee_config(raw)
        self.assertFalse(decoded["decode_ok"])


class DecodeGlobalConfigs(unittest.TestCase):
    def test_amm_global_config_prefix_and_trailing(self):
        body = bytes(range(1, 33))          # admin
        body += struct.pack("<Q", 20)        # lp bps
        body += struct.pack("<Q", 5)         # protocol bps
        body += b"\x00"                      # disable_flags
        body += bytes(32 * 8)                # protocol_fee_recipients
        body += struct.pack("<Q", 42)        # coin_creator_fee_basis_points
        raw = fcp.anchor_account_discriminator("GlobalConfig") + body
        decoded = fcp.decode_amm_global_config(raw)
        self.assertTrue(decoded["decode_ok"])
        self.assertEqual(decoded["lp_fee_basis_points"], 20)
        self.assertEqual(decoded["protocol_fee_basis_points"], 5)
        self.assertEqual(decoded["coin_creator_fee_basis_points"], 42)

    def test_amm_global_config_tolerates_missing_trailing(self):
        body = bytes(range(1, 33)) + struct.pack("<Q", 20) + struct.pack("<Q", 5)
        raw = fcp.anchor_account_discriminator("GlobalConfig") + body
        decoded = fcp.decode_amm_global_config(raw)
        self.assertTrue(decoded["decode_ok"])
        self.assertEqual(decoded["lp_fee_basis_points"], 20)

    def test_pump_global_decodes_fallback_fees(self):
        body = b"\x01"                            # initialized
        body += bytes(range(1, 33))               # authority
        body += bytes(range(33, 65))              # fee_recipient
        body += struct.pack("<QQQQ", 1, 2, 3, 4)  # initial reserves + supply
        body += struct.pack("<Q", 100)            # fee_basis_points
        body += bytes(range(65, 97))              # withdraw_authority
        body += b"\x01"                           # enable_migrate
        body += struct.pack("<Q", 500)            # pool_migration_fee
        body += struct.pack("<Q", 0)              # creator_fee_basis_points
        raw = fcp.anchor_account_discriminator("Global") + body
        decoded = fcp.decode_pump_global(raw)
        self.assertTrue(decoded["decode_ok"])
        self.assertEqual(decoded["fee_basis_points"], 100)
        self.assertEqual(decoded["creator_fee_basis_points"], 0)
        self.assertEqual(decoded["pool_migration_fee"], 500)


class ProbeFlow(unittest.TestCase):
    def test_probe_writes_receipt_with_observed_tiers(self):
        raw = fee_config_bytes(
            fees_bytes(20, 5, 0),
            [tier_u128(0, 125, 0, 0)],
            [],
            fees_bytes(25, 25, 0),
        )
        rpc = FakeRpc({fcp.CURVE_FEE_CONFIG: raw, fcp.AMM_FEE_CONFIG: raw})
        with tempfile.TemporaryDirectory() as tmp:
            result = fcp.probe(rpc, tmp)
            receipt = json.loads(Path(result["receipt_path"]).read_text())
            self.assertTrue(receipt["nonpromotable"])
            self.assertEqual(receipt["evidence_class"], "MAINNET_READ_ONLY_OBSERVED")
            curve = receipt["accounts"][fcp.CURVE_FEE_CONFIG]
            self.assertTrue(curve["decode_ok"])
            self.assertEqual(curve["fee_tiers"][0]["fees"]["lp_fee_bps"], 125)
            self.assertEqual(receipt["discrepancy_check"]["canonical_reference_bps_in_code"], 25)

    def test_missing_account_is_recorded_not_fabricated(self):
        rpc = FakeRpc({})
        with tempfile.TemporaryDirectory() as tmp:
            result = fcp.probe(rpc, tmp)
            receipt = json.loads(Path(result["receipt_path"]).read_text())
            for entry in receipt["accounts"].values():
                self.assertIsNone(entry.get("data_len"))

    def test_wrong_genesis_refuses(self):
        rpc = FakeRpc({})
        rpc.get_genesis_hash = lambda: "wrong"
        with tempfile.TemporaryDirectory() as tmp:
            self.assertRaises(WrongCluster, fcp.probe, rpc, tmp)

    def test_refuses_live_env(self):
        with mock.patch.dict(
            "os.environ",
            {"LIVE_TRADING_ENABLED": "false", "PRIVATE_KEY": "x"},
        ):
            self.assertRaises(Exception, fcp.probe, FakeRpc({}), "/tmp/never")


if __name__ == "__main__":
    unittest.main()
