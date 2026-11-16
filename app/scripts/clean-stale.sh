#!/usr/bin/env bash
# Removes build and test leftovers that are no longer needed (FOUNDATION §9, rule 20).
# Touches only git-ignored paths inside the repository; never tracked files, never local/.
set -euo pipefail
repo="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo"
rmrf() { for p in "$@"; do [[ -e "$p" ]] || continue
  if git check-ignore -q "$p"; then rm -rf "$p" && echo "removed $p"; else echo "skip (not ignored): $p"; fi; done; }
rmrf app/target/debug/incremental app/target/release/incremental app/target/tmp
rmrf app/ui/test-results app/ui/playwright-report app/ui/dist
# The release profile is only used by `make bench` and `make build`: dropped while no package was built from it.
if [[ -d app/target/release && ! -d app/target/release/bundle ]]; then rmrf app/target/release; fi
# Older packages: keep only the newest .dmg
if [[ -d app/target/release/bundle/dmg ]]; then
  ls -t app/target/release/bundle/dmg/*.dmg 2>/dev/null | tail -n +2 | while read -r f; do rmrf "$f"; done
fi
# Superseded test and binary executables: keep only the newest build of each name (name-hash, no extension)
for d in app/target/debug/deps app/target/release/deps; do
  [[ -d "$d" ]] || continue
  find "$d" -maxdepth 1 -type f -perm -u+x ! -name '*.*' | sed -E 's/-[0-9a-f]{16}$//' | sort -u | while read -r stem; do
    ls -t "$stem"-???????????????? 2>/dev/null | tail -n +2 | while read -r f; do rmrf "$f"; done
  done
done
# Dependency artefacts not used for 7 days (superseded versions)
for d in app/target/debug/deps app/target/release/deps; do
  [[ -d "$d" ]] && find "$d" -type f -atime +7 -print -delete | sed 's/^/removed /' || true
done
total=$(du -sk . | cut -f1)
if (( total > 20 * 1048576 )); then
  echo "still above 20 GB: cargo clean"; (cd app && cargo clean)
fi
bash app/scripts/disk.sh
