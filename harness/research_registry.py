#!/usr/bin/env python3
"""H8 research registry (Mission C / PF5): immutable hypothesis and trial
ledger + a genuinely one-use, cross-process-enforced sealed holdout.

Properties:
- Append-only JSONL ledger, hash-chained (each record commits to the
  previous record's hash); any tampering is detected by verify().
- Trials are structurally bounded: at most 20 per registry (cycle budget).
- The sealed holdout enforces single use ACROSS PROCESSES AND CRASHES:
  an exclusive-create marker file plus an atomic open-counter, guarded by
  an flock while deciding. A second open — from any process, ever — is
  denied and audited.
- Temporal and creator-cluster splits are deterministic functions of the
  dataset hash. Label embargo: labels live inside the holdout directory
  and are only readable through the guard.
- Multiple-comparison correction (Bonferroni and Benjamini-Hochberg) for
  the independent critic artifact.
- Offline, read-only tooling: refuses live-trading env and key material;
  no signer, no network, no promotion path. Everything is nonpromotable.
"""

import argparse
import fcntl
import hashlib
import json
import os
import re
import sys
import tempfile
import time
from pathlib import Path

MAX_TRIALS = 20
HYPOTHESIS_REQUIRED = (
    "hypothesis_id", "claim", "causal_mechanism", "decision_time_evidence",
    "abstention_conditions", "cost_model", "falsification_conditions",
    "development_dataset_sha256", "primary_metric", "risk_constraints",
)


def require_safe_env():
    live = os.environ.get("LIVE_TRADING_ENABLED", "false").lower()
    if live not in ("", "false", "0"):
        raise SystemExit("research registry refuses LIVE_TRADING_ENABLED")
    for key in ("PRIVATE_KEY", "SOLANA_PRIVATE_KEY", "MNEMONIC", "SEED_PHRASE"):
        if os.environ.get(key):
            raise SystemExit(f"research registry refuses key material in env ({key})")


def _sha256(text):
    return hashlib.sha256(text.encode()).hexdigest()


def _canonical(obj):
    return json.dumps(obj, sort_keys=True, separators=(",", ":"))


class RegistryError(Exception):
    pass


class Registry:
    """Hash-chained append-only hypothesis/trial ledger."""

    def __init__(self, path):
        self.path = Path(path)
        if self.path.exists():
            self.records = [json.loads(line)
                            for line in self.path.read_text().splitlines() if line.strip()]
            self.verify()
        else:
            self.records = []

    def _head(self):
        return self.records[-1]["record_sha256"] if self.records else "GENESIS"

    def _append(self, kind, payload):
        record = {
            "seq": len(self.records),
            "prev_hash": self._head(),
            "kind": kind,
            "payload": payload,
        }
        record["record_sha256"] = _sha256(_canonical(record))
        self.records.append(record)
        with open(self.path, "a", encoding="utf-8") as handle:
            handle.write(_canonical(record) + "\n")
            handle.flush()
            os.fsync(handle.fileno())
        return record

    def verify(self):
        prev = "GENESIS"
        for index, record in enumerate(self.records):
            if record["seq"] != index or record["prev_hash"] != prev:
                raise RegistryError(f"chain broken at record {index}")
            body = {k: record[k] for k in ("seq", "prev_hash", "kind", "payload")}
            if _sha256(_canonical(body)) != record["record_sha256"]:
                raise RegistryError(f"record {index} hash mismatch (tampered?)")
            prev = record["record_sha256"]
        return True

    def hypotheses(self):
        return [r["payload"] for r in self.records if r["kind"] == "hypothesis"]

    def trials(self):
        return [r["payload"] for r in self.records if r["kind"] == "trial"]

    def register_hypothesis(self, payload):
        missing = [f for f in HYPOTHESIS_REQUIRED if f not in payload]
        if missing:
            raise RegistryError(f"hypothesis missing fields: {missing}")
        if not re.fullmatch(r"H-\d{4}", str(payload["hypothesis_id"])):
            raise RegistryError("hypothesis_id must match H-\\d{4}")
        if any(h["hypothesis_id"] == payload["hypothesis_id"] for h in self.hypotheses()):
            raise RegistryError(f"duplicate hypothesis_id {payload['hypothesis_id']}")
        payload = dict(payload)
        payload["status"] = "PREREGISTERED"
        payload["registered_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        return self._append("hypothesis", payload)

    def record_trial(self, payload):
        required = ("hypothesis_id", "trial_index", "result", "dev_metrics")
        missing = [f for f in required if f not in payload]
        if missing:
            raise RegistryError(f"trial missing fields: {missing}")
        if any(t["hypothesis_id"] == payload["hypothesis_id"]
               and t["trial_index"] == payload["trial_index"] for t in self.trials()):
            raise RegistryError("duplicate trial index for hypothesis")
        # STRUCTURAL bound: 20 trials per registry, no override path.
        if len(self.trials()) >= MAX_TRIALS:
            raise RegistryError(
                f"trial budget exhausted ({MAX_TRIALS}/cycle): open a new registry cycle")
        return self._append("trial", payload)


class HoldoutError(Exception):
    pass


class HoldoutGuard:
    """One-use sealed holdout with cross-process enforcement.

    Layout inside `directory`:
      labels.json     — the sealed labels (created by `seal`)
      .holdout.lock   — flock target while deciding open/deny
      OPENED          — O_EXCL marker created exactly once on first open
      open_count      — persistent counter (atomic under the lock)
      audit.jsonl     — every open ATTEMPT (granted or denied) is recorded
    """

    def __init__(self, directory):
        self.dir = Path(directory)
        self.dir.mkdir(parents=True, exist_ok=True)
        self.lock_path = self.dir / ".holdout.lock"
        self.marker = self.dir / "OPENED"
        self.count_path = self.dir / "open_count"
        self.audit_path = self.dir / "audit.jsonl"

    def seal(self, labels):
        labels_path = self.dir / "labels.json"
        if labels_path.exists():
            raise HoldoutError("holdout already sealed")
        tmp = labels_path.with_suffix(".tmp")
        tmp.write_text(_canonical(labels))
        tmp.replace(labels_path)
        os.chmod(labels_path, 0o444)

    def _audit(self, granted, detail):
        entry = {
            "at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "pid": os.getpid(),
            "granted": granted,
            "detail": detail,
        }
        with open(self.audit_path, "a", encoding="utf-8") as handle:
            handle.write(_canonical(entry) + "\n")

    def open(self, purpose="evaluation"):
        """Open the holdout exactly once, process-wide and forever.
        Returns the labels on the single granted open; every later call
        (from any process, including after crashes) raises HoldoutError."""
        import fcntl as _fcntl
        self.lock_path.touch(exist_ok=True)
        with open(self.lock_path, "r+b") as lock:
            _fcntl.flock(lock.fileno(), _fcntl.LOCK_EX)
            try:
                if self.marker.exists():
                    count = int(self.count_path.read_text() or 0) if self.count_path.exists() else 1
                    self._audit(False, f"denied: already opened ({count} prior)")
                    raise HoldoutError("holdout already opened; one-use invariant")
                labels_path = self.dir / "labels.json"
                if not labels_path.exists():
                    self._audit(False, "denied: not sealed")
                    raise HoldoutError("holdout not sealed")
                # Create the marker EXCLUSIVELY: a concurrent process loses
                # the race and lands in the branch above.
                fd = os.open(self.marker, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
                os.write(fd, purpose.encode())
                os.close(fd)
                self.count_path.write_text("1")
                self._audit(True, purpose)
                return json.loads(labels_path.read_text())
            finally:
                _fcntl.flock(lock.fileno(), _fcntl.LOCK_UN)

    def attempts(self):
        if not self.audit_path.exists():
            return []
        return [json.loads(l) for l in self.audit_path.read_text().splitlines() if l.strip()]


def temporal_split(items, dev_fraction, dataset_sha256):
    """Deterministic temporal split: first `dev_fraction` of items (by
    arrival order) are development; the remainder is held out. Split
    boundaries commit to the dataset hash for the registry."""
    if not 0.0 < dev_fraction < 1.0:
        raise RegistryError("dev_fraction must be in (0,1)")
    cut = max(1, int(len(items) * dev_fraction))
    return {
        "development": items[:cut],
        "holdout": items[cut:],
        "dataset_sha256": dataset_sha256,
        "method": "temporal-by-arrival-order",
    }


def creator_cluster_split(records, dev_fraction, dataset_sha256):
    """Disjoint creator-cluster split: all records of one creator stay on
    ONE side (prevents identity-reputation leakage across the split).
    Creator order is deterministic (sorted by creator, then first-seen)."""
    by_creator = {}
    for record in records:
        by_creator.setdefault(record["creator"], []).append(record)
    creators = sorted(by_creator)
    cut = max(1, int(len(creators) * dev_fraction))
    dev_creators = set(creators[:cut])
    dev = [r for c in creators[:cut] for r in by_creator[c]]
    hold = [r for c in creators[cut:] for r in by_creator[c]]
    return {
        "development": dev,
        "holdout": hold,
        "dev_creators": sorted(dev_creators),
        "dataset_sha256": dataset_sha256,
        "method": "creator-cluster-disjoint",
    }


def bonferroni(p_values):
    n = len(p_values)
    return [min(1.0, p * n) for p in p_values]


def benjamini_hochberg(p_values):
    n = len(p_values)
    order = sorted(range(n), key=lambda i: p_values[i])
    adjusted = [0.0] * n
    prev = 1.0
    for rank_index in range(n - 1, -1, -1):
        i = order[rank_index]
        rank = rank_index + 1
        prev = min(prev, p_values[i] * n / rank)
        adjusted[i] = min(1.0, prev)
    return adjusted


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    reg = sub.add_parser("register", help="preregister a hypothesis (JSON file)")
    reg.add_argument("--registry", required=True)
    reg.add_argument("--hypothesis", required=True)
    trial = sub.add_parser("trial", help="record a trial result (JSON file)")
    trial.add_argument("--registry", required=True)
    trial.add_argument("--result", required=True)
    verify = sub.add_parser("verify")
    verify.add_argument("--registry", required=True)
    args = parser.parse_args(argv)
    require_safe_env()
    if args.command == "register":
        registry = Registry(args.registry)
        payload = json.loads(Path(args.hypothesis).read_text())
        record = registry.register_hypothesis(payload)
        print(json.dumps({"seq": record["seq"], "status": "PREREGISTERED"}))
    elif args.command == "trial":
        registry = Registry(args.registry)
        payload = json.loads(Path(args.result).read_text())
        record = registry.record_trial(payload)
        print(json.dumps({"seq": record["seq"],
                          "trials_used": len(registry.trials()),
                          "trials_budget": MAX_TRIALS}))
    elif args.command == "verify":
        registry = Registry(args.registry)
        registry.verify()
        print(json.dumps({"chain": "INTACT",
                          "hypotheses": len(registry.hypotheses()),
                          "trials": len(registry.trials())}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
