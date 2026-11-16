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
# Older packages: keep only the newest of each kind (.dmg on macOS, the NSIS installer on Windows)
for kind in dmg/*.dmg nsis/*.exe; do
  [[ -d "app/target/release/bundle/${kind%%/*}" ]] || continue
  { ls -t app/target/release/bundle/$kind 2>/dev/null || true; } | tail -n +2 | while read -r f; do rmrf "$f"; done
done
# Superseded test and binary executables: keep only the newest build of each name (name-hash, with no extension,
# or `.exe` and its `.pdb` on Windows)
for d in app/target/debug/deps app/target/release/deps; do
  [[ -d "$d" ]] || continue
  find "$d" -maxdepth 1 -type f \( \( -perm -u+x ! -name '*.*' \) -o -name '*.exe' \) | sed -E 's/-[0-9a-f]{16}(\.exe)?$//'     | sort -u | while read -r stem; do
    { ls -t "$stem"-???????????????? "$stem"-????????????????.exe 2>/dev/null || true; } | tail -n +2 | while read -r f; do
      rmrf "$f"; rmrf "${f%.exe}.pdb"
    done
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
