"""Offline wallet-alpha evaluation tests: all fixtures synthetic, no RPC."""
import importlib.util
import json
import tempfile
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("wallet_alpha_lab", ROOT / "harness" / "wallet_alpha_lab.py")
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)
FIXTURE = ROOT / "harness" / "fixtures" / "wallet_alpha_synthetic.jsonl"

class WalletAlphaLabTests(unittest.TestCase):
    def test_net_after_costs_and_no_trade_control(self):
        r = lab.metrics(lab.load_records(FIXTURE), min_trades=2)
        self.assertFalse(r["promotable"])
        self.assertFalse(r["portfolio_nav_reconciled"])
        self.assertIsNone(r["realized_mainnet_pnl"])
        self.assertEqual(r["excluded_incomplete_rows"]["INCOMPLETE"], 1)
        alpha = next(x for x in r["wallets"] if x["wallet"] == "SYNTHETIC_WALLET_ALPHA")
        self.assertEqual(alpha["complete_roundtrips"], 2)
        self.assertEqual(alpha["follower_net_pnl_lamports"], -11000)
        self.assertEqual(alpha["net_edge_vs_no_trade_lamports"], -11000)
        self.assertEqual(alpha["verdict"], "NEGATIVE_NET")

    def test_tiny_sample_not_called_profitable(self):
        r = lab.metrics(lab.load_records(FIXTURE))
        self.assertTrue(all(x["verdict"] == "INSUFFICIENT_SAMPLE" for x in r["wallets"]))

    def test_future_signal_or_ranking_rejected(self):
        item = lab.load_records(FIXTURE)[0]
        item["ranking_asof_ms"] = item["leader_buy_ms"] + 1
        with self.assertRaisesRegex(ValueError, "lookahead"):
            lab.validate_row(item, 1)

    def test_future_quote_used_before_signal_rejected(self):
        item = lab.load_records(FIXTURE)[0]
        item["follower_entry_quote_at_ms"] = item["signal_observed_ms"] - 1
        with self.assertRaisesRegex(ValueError, "lookahead"):
            lab.validate_row(item, 1)

    def test_transfer_not_a_trade_and_wrong_proof_rejected(self):
        item = lab.load_records(FIXTURE)[0]
        item["proof_state"] = "MAINNET_EXECUTED"
        with self.assertRaisesRegex(ValueError, "proof_state"):
            lab.validate_row(item, 1)

    def test_duplicate_trade_rejected(self):
        row = FIXTURE.read_text(encoding="utf-8").splitlines()[0]
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "dupe.jsonl"
            p.write_text(row+"\n"+row+"\n")
            with self.assertRaisesRegex(ValueError, "duplicate"):
                lab.load_records(p)

    def test_boolean_amount_and_negative_amount_rejected(self):
        item = lab.load_records(FIXTURE)[0]
        item["tip_lamports"] = True
        with self.assertRaisesRegex(ValueError, "tip_lamports"):
            lab.validate_row(item, 1)
        item["tip_lamports"] = -1
        with self.assertRaisesRegex(ValueError, "tip_lamports"):
            lab.validate_row(item, 1)

    def test_stable_replay_hash(self):
        a = lab.metrics(lab.load_records(FIXTURE))
        b = lab.metrics(lab.load_records(FIXTURE))
        self.assertEqual(a, b)

    def test_no_unverified_rows_count_as_profit(self):
        rows = lab.load_records(FIXTURE)
        last = rows[-1]
        self.assertEqual(last["coverage"], "INCOMPLETE")
        result = lab.metrics(rows)
        self.assertNotIn(last["wallet"], [w["wallet"] for w in result["wallets"]])


class WalletRosterTests(unittest.TestCase):
    def test_discovery_excludes_infrastructure_and_failed(self):
        import wallet_roster as wr
        pump = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
        w = "Wa11etWa11etWa11etWa11etWa11etWa11etWa11etWa"
        tx_ok = {
            "transaction": {"signatures": ["S1"], "message": {"accountKeys": [w, pump]}},
            "meta": {"err": None,
                     "preTokenBalances": [], "postTokenBalances": [
                         {"owner": w, "mint": "M", "accountIndex": 2,
                          "uiTokenAmount": {"amount": "5", "decimals": 6}}]},
        }
        tx_failed = {
            "transaction": {"signatures": ["S2"], "message": {"accountKeys": [pump, pump]}},
            "meta": {"err": {"InstructionError": [0, "Custom(1)"]}, "preTokenBalances": [], "postTokenBalances": []},
        }
        wallets = wr.discover_wallets([tx_ok, tx_failed])
        self.assertIn(w, wallets)
        self.assertNotIn(pump, wallets, "infrastructure never ranks as a wallet")

    def test_ranking_tiers_and_uncertainty(self):
        import wallet_roster as wr
        receipts = {
            "profitableObserved": {"net_pnl_lamports": 1_000, "records": 5, "skipped": 0,
                                   "incomplete": 0, "unobserved_opening_inventory_events": 0,
                                   "unobserved_deposits_lamports": 0},
            "loserObserved": {"net_pnl_lamports": -4_961_316_798, "records": 3, "skipped": 0,
                              "incomplete": 0, "unobserved_opening_inventory_events": 0,
                              "unobserved_deposits_lamports": 4_961_316_798},
            "unknownHistory": {"net_pnl_lamports": 0, "records": 3, "skipped": 0,
                               "incomplete": 0, "unobserved_opening_inventory_events": 3,
                               "unobserved_deposits_lamports": 0},
            "noTrades": {"net_pnl_lamports": 0, "records": 0, "skipped": 2,
                         "incomplete": 0, "unobserved_opening_inventory_events": 0,
                         "unobserved_deposits_lamports": 0},
        }
        rows = wr.rank(receipts)
        tiers = {r["wallet"]: r["tier"] for r in rows}
        self.assertEqual(tiers["profitableObserved"], "CANDIDATE")
        self.assertEqual(tiers["loserObserved"], "REJECTED_LOSS")
        self.assertEqual(tiers["unknownHistory"], "LOW_CONFIDENCE")
        self.assertEqual(tiers["noTrades"], "UNRANKABLE")
        self.assertEqual(rows[0]["wallet"], "profitableObserved", "net P&L dominates rank")
        for row in rows:
            self.assertIn("uncertainty", row, "uncertainty column mandatory")


class MillionIntegrationTests(unittest.TestCase):
    """Facts and heuristics adopted from million (MIT, upstream/million
    @ c89ad07c) — reimplemented, not copied."""

    def test_million_infra_addresses_excluded(self):
        import wallet_roster as wr
        # Raydium AMM v4 authority / pump fee recipient / Jupiter referral
        for infra in ("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",
                      "39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg",
                      "CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM",
                      "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDd1v"):
            self.assertIn(infra, wr.INFRASTRUCTURE)

    def test_blocklist_excludes_wallets(self):
        import tempfile as _temp
        import wallet_roster as wr
        from pathlib import Path as _P
        with _temp.TemporaryDirectory() as tmp:
            banned = _P(tmp) / "banned.txt"
            banned.write_text("# operator blocklist\nBadWa11etBadWa11etBadWa11etBadWa11etBadWa11et\n")
            bad = "BadWa11etBadWa11etBadWa11etBadWa11etBadWa11et"
            tx = {"transaction": {"signatures": ["S1"], "message": {"accountKeys": [bad]}},
                  "meta": {"err": None, "preTokenBalances": [], "postTokenBalances": []}}
            wallets = wr.discover_wallets([tx], banned=wr.load_banned(banned))
            self.assertEqual(wallets, {})

    def test_cross_token_top_buyer_is_probable_infrastructure(self):
        import wallet_roster as wr
        # A bot buying across 3 unrelated mints -> flagged; a human trading
        # 2 mints -> not flagged.
        mint_buyers = {
            "MintAaa": ["Bot11111111111111111111111111111111111111111", "Human1"],
            "MintBbb": ["Bot11111111111111111111111111111111111111111", "Human2"],
            "MintCcc": ["Bot11111111111111111111111111111111111111111"],
        }
        flagged = wr.flag_probable_infrastructure(mint_buyers, distinct_mints_threshold=3)
        self.assertIn("Bot11111111111111111111111111111111111111111", flagged)
        self.assertNotIn("Human1", flagged)


class WalletWatchTests(unittest.TestCase):
    """handi-cat UX concept (add/remove wallet + alerts), reimplemented
    offline. No handi-cat code (no license)."""

    def _watchlist(self, tmp):
        return str(Path(tmp) / "watchlist.json")

    def test_subscribe_unsubscribe_and_alert_gating(self):
        import tempfile as _temp
        import wallet_watch as ww
        W1 = "Wa11etWa11etWa11etWa11etWa11etWa11etWa11etWa"
        W2 = "Wa22etWa22etWa22etWa22etWa22etWa22etWa22etWa"
        with _temp.TemporaryDirectory() as tmp:
            wl = self._watchlist(tmp)
            ww.subscribe(wl, W1)
            with self.assertRaises(ww.WatchlistError):
                ww.subscribe(wl, W1)  # duplicate refused
            ww.subscribe(wl, W2)
            ww.unsubscribe(wl, W2)
            signals = str(Path(tmp) / "signals.jsonl")
            Path(signals).write_text("\n".join([
                json.dumps({"signal_id": "s1", "wallet": W1, "mint": "M", "leader_side": "Buy", "observed_at_ms": 1, "quote_available_at_ms": 1}),
                json.dumps({"signal_id": "s2", "wallet": W2, "mint": "M", "leader_side": "Buy", "observed_at_ms": 2, "quote_available_at_ms": 2}),
            ]) + "\n")
            out = str(Path(tmp) / "alerts.jsonl")
            summary = ww.process_signals(wl, signals, out)
            self.assertEqual(summary["alerted"], 1)
            self.assertEqual(summary["skipped"], 1)
            rows = [json.loads(l) for l in Path(out).read_text().splitlines()]
            kinds = {r["signal_id"]: r["kind"] for r in rows}
            self.assertEqual(kinds["s1"], "ALERT")
            self.assertEqual(kinds["s2"], "SKIPPED_NOT_SUBSCRIBED")
            # Immutable ledger.
            with self.assertRaises(ww.WatchlistError):
                ww.process_signals(wl, signals, out)

    def test_resubscribe_after_unsubscribe(self):
        import tempfile as _temp
        import wallet_watch as ww
        W = "Wa33etWa33etWa33etWa33etWa33etWa33etWa33etWa"
        with _temp.TemporaryDirectory() as tmp:
            wl = self._watchlist(tmp)
            ww.subscribe(wl, W)
            ww.unsubscribe(wl, W)
            ww.subscribe(wl, W)  # history preserved, active again
            data = ww.load_watchlist(wl)
            self.assertTrue(data["subscriptions"][W]["active"])
            self.assertIsNotNone(data["subscriptions"][W]["unsubscribed_at"])


if __name__ == "__main__":
    unittest.main()


class OwnerGraphTests(unittest.TestCase):
    """million's owner-graph + consensus factories, reimplemented."""

    def _owners(self):
        import wallet_signals as ws
        # A funds B and C (same actor); D/E/F co-enter 3 shared mints but
        # with NO funding bridge (copytraders: co-entry must NOT merge
        # them); G is a hub paying fees for 7 wallets (exchange) and must
        # be excluded from clustering.
        funding = {
            "A": {"B": 5 * 1_000_000_000, "C": 2 * 1_000_000_000},
            "B": {"A": 1.2 * 1_000_000_000},
            "D": {}, "E": {}, "F": {}, "G": {},
            "w1": {"G": 5 * 1_000_000_000}, "w2": {"G": 5 * 1_000_000_000},
            "w3": {"G": 5 * 1_000_000_000}, "w4": {"G": 5 * 1_000_000_000},
            "w5": {"G": 5 * 1_000_000_000}, "w6": {"G": 5 * 1_000_000_000},
            "w7": {"G": 5 * 1_000_000_000},
        }
        top_fee = {}  # fee-payer path exercised by hub exclusion above
        entered = {
            "A": {"m1"}, "B": {"m2"}, "C": set(),
            "D": {"x", "y", "z"}, "E": {"x", "y", "z"}, "F": {"x", "y", "z"},
        }
        edges = ws.build_edges(funding, top_fee, entered)
        owners, hubs = ws.cluster_owners(edges)
        return edges, owners, hubs

    def test_funding_bridges_same_actor_into_one_owner(self):
        edges, owners, hubs = self._owners()
        self.assertEqual(owners.get("A"), owners.get("B"))
        self.assertEqual(owners.get("A"), owners.get("C"))

    def test_coentry_alone_never_bridges(self):
        edges, owners, hubs = self._owners()
        # D/E/F shared 3 mints with Jaccard 1.0 — but ONLY co-entry edges:
        self.assertNotIn("D", owners)
        self.assertNotIn("E", owners)
        self.assertNotIn("F", owners)

    def test_high_degree_hub_excluded(self):
        edges, owners, hubs = self._owners()
        self.assertIn("G", hubs, "the 7-edge fee hub is a disperser/exchange")
        self.assertNotIn("G", owners)
        # wallets paid THROUGH G are not merged with each other
        ids = {owners.get(w) for w in ("w1", "w2", "w3")}
        self.assertEqual(ids, {None})


class ConsensusTests(unittest.TestCase):
    def test_owner_dedup_and_net_flow_gate(self):
        import wallet_signals as ws
        owners = {
            "A1": "owner:x", "A2": "owner:x",  # one actor, two wallets
            "B": "owner:y",
        }
        entries = [
            {"wallet": "A1", "mint": "m", "quote_delta_lamports": -2_000_000_000},
            {"wallet": "A2", "mint": "m", "quote_delta_lamports": -1_500_000_000},
            {"wallet": "B", "mint": "m", "quote_delta_lamports": -1_200_000_000},
        ]
        # Without net-flow check: 2 distinct owners -> consensus fires.
        result = ws.consensus(entries, owners, min_owners=2, net_flow=False)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["distinct_owner_buyers"], 2)
        # A2 ALSO sells big: buyers 4.7 SOL vs sellers 6 SOL -> net-flow
        # gate kills the signal (breadth is not agreement).
        entries.append({"wallet": "A2", "mint": "m", "quote_delta_lamports": +6_000_000_000})
        self.assertEqual(ws.consensus(entries, owners, min_owners=2, net_flow=True), [])

    def test_ladder_judged_on_cumulative_sol(self):
        import wallet_signals as ws
        base = 1_700_000_000_000
        clips = [
            {"wallet": "W", "mint": "m", "at_ms": base + i * 60_000,
             "quote_delta_lamports": -1_400_000_000}
            for i in range(3)
        ]
        # 3 x 1.4 SOL clips = 4.2 SOL cumulative: per-clip floors at 2 SOL
        # would hide it; the ladder detector catches it.
        self.assertEqual(len(ws.ladder(clips)), 1)
        self.assertEqual(ws.ladder(clips)[0]["clips"], 3)
        # A sell in-window disqualifies.
        with_sell = clips + [{"wallet": "W", "mint": "m", "at_ms": base + 200_000,
                              "quote_delta_lamports": +1_000_000_000}]
        self.assertEqual(ws.ladder(with_sell), [])
