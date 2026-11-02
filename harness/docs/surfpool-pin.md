# Surfpool pin record — reviewed, pinned install (H1 / M3c)
# The binary itself lives in harness/bin/ (gitignored). Reproduce with the
# commands below and verify the digest before use.
#
# Upstream      : github.com/solana-foundation/surfpool (Apache-2.0, Solana Foundation)
# Version       : v1.7.0 (release published 2026-10-28)
# Asset         : surfpool-linux-x64.tar.gz (34,588,771 bytes)
# SHA-256       : 5bef2b0b186a74622508f104b6d2e4e2db040932b6d8f3a811172e241f802a63
# Install method: direct release-asset download (NOT the `curl | bash` installer,
#                 which is unpinned pipe-to-shell and was rejected for that reason)
#
# Reproduce:
#   curl -sSL -o harness/bin/surfpool-linux-x64.tar.gz \
#     https://github.com/solana-foundation/surfpool/releases/download/v1.7.0/surfpool-linux-x64.tar.gz
#   sha256sum harness/bin/surfpool-linux-x64.tar.gz   # must match the digest above
#   tar xzf harness/bin/surfpool-linux-x64.tar.gz -C harness/bin/
#   harness/bin/surfpool --version                    # must print: surfpool 1.7.0
#
# Architecture note (for the M3c Surfpool-vs-LiteSVM comparison): Surfpool is
# built on the low-level solana-svm API via the LiteSVM wrapper, adding an
# RPC/WS server, JIT mainnet account fetching, and IaC tooling on top.
