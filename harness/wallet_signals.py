#!/usr/bin/env python3
"""WA: owner graph + clustering + consensus signals — million's most
valuable adoptable factories (MIT upstream/million @ c89ad07c,
reimplemented originally; attribution in docs/WALLET_ALPHA_MILLION_FEATURES.md).

- Owner graph: funding / fee-payer / co-entry edges; union-find
  clustering with CO-ENTRY-NEVER-BRIDGES and degree>6 hub exclusion
  (one actor's five wallets must read as one owner, and dispersers/
  exchanges must never chain the roster into one blob).
- Consensus: distinct-OWNER net-buyer breadth with the net-flow check
  (buyers must OUTWEIGH sellers, not merely outnumber them — million's
  'fone' exit-liquidity lesson).
- Ladder: >=3 clips of one mint by one wallet in a window, judged on
  CUMULATIVE SOL (per-clip floors hide real accumulation).
All offline-deterministic; no network; no keys.
"""

import argparse
import json
from collections import defaultdict
from pathlib import Path

FUNDING_MIN_SOL = 1.0          # aggregate, per counterparty
FUNDING_TOP_N = 10
FEEPAYER_SHARE = 0.15
COENTRY_MIN_SHARED = 2
COENTRY_JACCARD = 0.25
HUB_DEGREE_MAX = 6

LAMPORTS = 1_000_000_000


class UnionFind:
    def __init__(self):
        self.parent = {}

    def find(self, x):
        self.parent.setdefault(x, x)
        while self.parent[x] != x:
            self.parent[x] = self.parent[self.parent[x]]
            x = self.parent[x]
        return x

    def union(self, a, b):
        ra, rb = self.find(a), self.find(b)
        if ra != rb:
            self.parent[rb] = ra


def build_edges(wallets_funding, top_fee_payers, entered_positions):
    """wallets_funding: {wallet: {counterparty: total_lamports}}
    top_fee_payers:  {wallet: (address, share)}
    entered_positions: {wallet: set(mint)} (positions >= threshold SOL-in)
    Returns (edges, edge_types) with normalized a<b keys."""
    edges = defaultdict(set)  # (a,b) -> {types}

    for wallet, counterparties in wallets_funding.items():
        ranked = sorted(
            ((c, lam) for c, lam in counterparties.items()
             if lam >= FUNDING_MIN_SOL * LAMPORTS),
            key=lambda kv: -kv[1],
        )[:FUNDING_TOP_N]
        for c, _ in ranked:
            a, b = sorted([wallet, c])
            edges[(a, b)].add("funding")

    for wallet, (payer, share) in top_fee_payers.items():
        if share >= FEEPAYER_SHARE and payer != wallet:
            a, b = sorted([wallet, payer])
            edges[(a, b)].add("feepayer")

    wallets = sorted(entered_positions)
    for i, w1 in enumerate(wallets):
        for w2 in wallets[i + 1:]:
            s1, s2 = entered_positions[w1], entered_positions[w2]
            shared = s1 & s2
            if len(shared) >= COENTRY_MIN_SHARED:
                union_len = len(s1 | s2)
                if union_len and len(shared) / union_len >= COENTRY_JACCARD:
                    a, b = sorted([w1, w2])
                    edges[(a, b)].add("coentry")
    return edges


def cluster_owners(edges):
    """Union-find over funding/feepayer edges ONLY (co-entry never
    bridges), excluding nodes with degree > HUB_DEGREE_MAX. Returns
    {wallet: owner_id} where only multi-wallet groups get ids."""
    degree = defaultdict(int)
    for (a, b) in edges:
        degree[a] += 1
        degree[b] += 1
    hubs = {w for w, d in degree.items() if d > HUB_DEGREE_MAX}

    uf = UnionFind()
    for (a, b), types in edges.items():
        if "coentry" in types and len(types) == 1:
            continue  # co-entry alone never bridges clusters
        if a in hubs or b in hubs:
            continue  # never cluster THROUGH a disperser/exchange
        if types - {"coentry"}:  # only funding/feepayer bridge
            uf.union(a, b)

    groups = defaultdict(list)
    for wallet in list(uf.parent):
        groups[uf.find(wallet)].append(wallet)

    owners = {}
    for root, members in groups.items():
        if len(members) > 1:
            owner_id = "owner:" + min(members)  # deterministic
            for m in members:
                owners[m] = owner_id
    return owners, hubs


def owner_key(wallet, owners):
    return owners.get(wallet, "addr:" + wallet)


def consensus(entries, owners=None, min_owners=2, min_sol=1.0, net_flow=True):
    """entries: [{wallet, mint, quote_delta_lamports}] over the live
    window; returns mints with >= min_owners distinct owners net-buying
    >= min_sol each — and, with net_flow, buyers' SOL must outweigh
    sellers' (breadth is not agreement)."""
    owners = owners or {}
    per = defaultdict(lambda: defaultdict(int))  # mint -> owner -> netSol
    for e in entries:
        ok = owner_key(e["wallet"], owners)
        per[e["mint"]][ok] += e["quote_delta_lamports"]

    out = []
    for mint, by_owner in sorted(per.items()):
        # A BUY spends quote (negative delta): the owner's net SPEND in SOL.
        buyers = {o: -sol for o, sol in by_owner.items()
                  if -sol >= min_sol * LAMPORTS}
        sellers_sol = sum(sol for sol in by_owner.values() if sol > 0)
        buyers_sol = sum(buyers.values())
        if len(buyers) >= min_owners and (not net_flow or buyers_sol > sellers_sol):
            out.append({
                "mint": mint,
                "distinct_owner_buyers": len(buyers),
                "buyer_sol": buyers_sol / LAMPORTS,
                "seller_sol": sellers_sol / LAMPORTS,
            })
    out.sort(key=lambda r: (-r["distinct_owner_buyers"], r["mint"]))
    return out[:20]


def ladder(events, min_clips=3, window_ms=10 * 60 * 1000,
           min_total_sol=3.0):
    """events: [{wallet, mint, at_ms, quote_delta_lamports}] buys/sells.
    A ladder: >= min_clips buys of one mint by one wallet inside the
    window, cumulative >= min_total_sol, no sells in-window."""
    by_key = defaultdict(list)
    for e in events:
        if e["quote_delta_lamports"] < 0:  # a buy spends quote
            by_key[(e["wallet"], e["mint"])].append(e)
    sold = {(e["wallet"], e["mint"]) for e in events
            if e["quote_delta_lamports"] > 0}

    ladders = []
    for (wallet, mint), buys in sorted(by_key.items()):
        if (wallet, mint) in sold:
            continue
        buys.sort(key=lambda e: e["at_ms"])
        run = []
        for buy in buys:
            run.append(buy)
            while run and buy["at_ms"] - run[0]["at_ms"] > window_ms:
                run.pop(0)
            total = sum(-b["quote_delta_lamports"] for b in run)
            if len(run) >= min_clips and total >= min_total_sol * LAMPORTS:
                ladders.append({
                    "wallet": wallet, "mint": mint,
                    "clips": len(run),
                    "cumulative_sol": total / LAMPORTS,
                })
                break
    return ladders


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    cons = sub.add_parser("consensus")
    cons.add_argument("--entries", required=True)
    cons.add_argument("--owners", default=None)
    cons.add_argument("--min-owners", type=int, default=2)
    lad = sub.add_parser("ladder")
    lad.add_argument("--events", required=True)
    args = parser.parse_args(argv)
    if args.command == "consensus":
        entries = json.loads(Path(args.entries).read_text())
        owners = json.loads(Path(args.owners).read_text()) if args.owners else {}
        print(json.dumps(consensus(entries, owners, args.min_owners), indent=2))
    else:
        events = json.loads(Path(args.events).read_text())
        print(json.dumps(ladder(events), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
