#!/usr/bin/env bash
# Disk usage of the repository (FOUNDATION §9, rules 19-20). With --check, fail above the 20 GB ceiling.
set -euo pipefail
repo="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo"
kb() { du -sk "$1" 2>/dev/null | cut -f1 || echo 0; }
human() { awk -v k="$1" 'BEGIN { if (k > 1048576) printf "%.2f GB", k/1048576; else printf "%.1f MB", k/1024 }'; }
total=$(kb .)
if [[ "${1:-}" == "--check" ]]; then
  limit=$((20 * 1048576))
  if (( total > limit )); then echo "disk: $(human "$total") is over the 20 GB ceiling (NFR-9): run make clean-stale or cargo clean" >&2; exit 1; fi
  echo "disk: $(human "$total") (ceiling 20 GB)"; exit 0
fi
printf "%-34s %12s\n" "repository (total)" "$(human "$total")"
for p in app/target/debug app/target/release app/target/tmp app/target/criterion app/target/doc \
         app/target/debug/incremental app/target/release/bundle app/ui/node_modules app/ui/dist \
         app/ui/test-results app/ui/playwright-report .git local poc/.venv; do
  [[ -e "$p" ]] && printf "%-34s %12s\n" "$p" "$(human "$(kb "$p")")"
done
if (( total > 10 * 1048576 )); then echo "warning: above 10 GB — run make clean-stale"; fi
