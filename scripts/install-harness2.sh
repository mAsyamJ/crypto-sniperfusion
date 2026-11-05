#!/usr/bin/env bash
# Idempotent installer for Google Research Harness² (ADR-0012).
# Pinned checkout under .external/harness2 + uv venv .venv-harness2.
# Deterministic, credential-free; no pipe-to-shell; fails loudly.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOCK_REPO="https://github.com/google-research/harness2.git"
LOCK_COMMIT="6868817518876de8fd170c6a8354daca273b6191"
EXT="$ROOT/.external/harness2"
VENV="$ROOT/.venv-harness2"

fail() { echo "install-harness2: $*" >&2; exit 1; }

command -v git >/dev/null || fail "git missing"
command -v uv  >/dev/null || fail "uv missing (https://docs.astral.sh/uv/)"
python3 -c 'import sys; assert sys.version_info >= (3,12,) and sys.version_info < (3,14,)' \
  || fail "python3 must be 3.12/3.13 (upstream requires >=3.12,<3.14)"

if [ -d "$EXT/.git" ]; then
  CURRENT="$(git -C "$EXT" rev-parse HEAD)"
  [ "$CURRENT" = "$LOCK_COMMIT" ] || fail "existing checkout at $CURRENT != lock $LOCK_COMMIT; resolve manually"
  echo "[ok] checkout already at pinned commit"
else
  mkdir -p "$ROOT/.external"
  git clone -q "$LOCK_REPO" "$EXT"
  git -C "$EXT" checkout -q --detach "$LOCK_COMMIT"
  echo "[ok] cloned + pinned at $LOCK_COMMIT"
fi

if [ ! -x "$VENV/bin/python" ]; then
  uv venv --python 3.12 "$VENV" >/dev/null
  echo "[ok] venv created"
fi

if ! "$VENV/bin/python" -c "import harness2" >/dev/null 2>&1 \
   || [ "$("$VENV/bin/python" -c "import harness2,os;print(os.path.dirname(harness2.__file__))" 2>/dev/null)" != "$EXT/harness2" ]; then
  uv pip install --python "$VENV/bin/python" -q -e "$EXT[render]"
  echo "[ok] installed -e .external/harness2[render]"
else
  echo "[ok] editable install already resolves to .external/harness2"
fi

"$VENV/bin/python" -c "import harness2,os; p=os.path.dirname(harness2.__file__); assert p=='$EXT/harness2', p" \
  || fail "import does not resolve to the pinned checkout"
for entry in harness2 harness2-demo harness2-judge harness2-report harness2-setup harness2-check harness2-bridge; do
  [ -x "$VENV/bin/$entry" ] || fail "CLI entrypoint missing: $entry"
done
echo "[ok] import + all 7 CLI entrypoints verified"
echo "OK: harness2 @ $LOCK_COMMIT ready in $VENV"
