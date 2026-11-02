#!/usr/bin/env bash
set -euo pipefail
ROOT="${HOME}/project/crypto-bot"
mkdir -p "$ROOT/upstream" "$ROOT/crates" "$ROOT/docs" "$ROOT/config" "$ROOT/fixtures" "$ROOT/research" "$ROOT/benchmarks" "$ROOT/scripts" "$ROOT/.github/workflows"
cd "$ROOT"
if [[ ! -d .git ]]; then git init -b main; fi
clone_if_absent() {
  local url="$1" name="$2"
  if [[ -e "upstream/$name" ]]; then
    echo "Preserving existing upstream/$name"
  else
    git clone "$url" "upstream/$name"
  fi
}
clone_if_absent https://github.com/chainstacklabs/pumpfun-bonkfun-bot.git chainstack
clone_if_absent https://github.com/yuto-kazuma/solana-sniper-bot.git yuto
clone_if_absent https://github.com/m8s-lab/solana-sniping-bot.git m8s
clone_if_absent https://github.com/bigmacman1129/solana-sniper-trading-mev-bot.git mev-bot
if [[ ! -f .gitignore ]]; then
  cat > .gitignore <<'IGNORE'
/upstream/
/target/
/.venv/
/.env
/.env.*
!/.env.example
/.secrets/
*.key
*.pem
*.log
IGNORE
fi
for repo in upstream/*; do
  if [[ -d "$repo/.git" ]]; then
    printf '%-25s %s\n' "$repo" "$(git -C "$repo" rev-parse HEAD)"
  fi
done
printf '\nWorkspace prepared: %s\nTrading is NOT enabled.\n' "$ROOT"
