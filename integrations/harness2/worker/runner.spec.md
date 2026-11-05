# Isolated Worker Spec — model-backed Harness² (PREPARED, NOT ENABLED)

Status: **PREPARED / NOT EXECUTED / NOT ENABLED.** Model-backed harness²
stays `BLOCKED_NO_ISOLATED_WORKER` until an operator completes the
checklist below BY HAND. This file is a specification, not a running
system.

## Why

The upstream `CodexSubstrate` shells out to `codex exec` with
`--dangerously-bypass-approvals-and-sandbox` — by design, upstream
documents that sandboxing must instead be enforced OUTSIDE the process
(DNS/network level). Enabling that on the development host (which has
operator RPC credentials in env files and wallet tooling elsewhere) is
unacceptable. Model-backed research therefore runs only inside a
dedicated unprivileged worker:

## Required controls (all must hold before enabling)

| # | Control | Enforcement |
|---|---|---|
| 1 | Dedicated unprivileged OS user (`h2worker`), no sudo | `useradd -m h2worker` |
| 2 | Dedicated VM or container (not the dev host) | separate host/VM |
| 3 | No wallet/keypair material anywhere on the worker | verified absence |
| 4 | No SSH agent forwarding into the worker | sshd_config `AllowAgentForwarding no` |
| 5 | No docker socket mount inside the worker | mount audit |
| 6 | Egress denied by default; allowlist only the model endpoint + DNS | firewall/DNS |
| 7 | CPU/RAM/process/time caps | systemd slice or container limits |
| 8 | Read-only snapshot of repo sources + fixtures; disposable task dir | tmpfs overlay |
| 9 | Env allowlist ONLY (PATH/HOME/LANG + HARNESS2_CODEX_* minimum set) | scrubbed_env |
| 10 | No SNF_* RPC credentials, no LIVE_TRADING_ENABLED≠false, no signer | scrubbed_env + audit |
| 11 | Run outputs copied out read-only; no live config mutation path | export script |

## Enablement

An operator who has provisioned the above creates this marker ON THE
WORKER (never on the dev host):

    touch integrations/harness2/worker/.operator-enabled

Only then does `assert_model_backing_allowed(True)` stop raising, and
only inside the worker. `harness2ctl doctor` on this host continues to
report `BLOCKED_NO_ISOLATED_WORKER` regardless.
