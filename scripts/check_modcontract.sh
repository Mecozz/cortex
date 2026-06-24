#!/usr/bin/env bash
set -euo pipefail

# Every feature module directory under src-tauri/src/ must contain:
#   health.rs  — implements the HealthCheck trait
#   mod.rs     — module root
#   README.md  — module documentation
# Infrastructure modules (core, db) are exempt.

SRC_DIR="src-tauri/src"
INFRA=("core" "db")
failed=0

for d in "$SRC_DIR"/*/; do
  [ -d "$d" ] || continue
  mod=$(basename "$d")
  is_infra=0
  for i in "${INFRA[@]}"; do
    [ "$mod" = "$i" ] && is_infra=1 && break
  done
  [ "$is_infra" -eq 1 ] && continue

  for required in mod.rs health.rs README.md; do
    if [ ! -f "$d/$required" ]; then
      echo "MODCONTRACT FAIL: module '$mod' is missing $required"
      failed=1
    fi
  done
done

[ "$failed" -eq 0 ] && echo "MODCONTRACT OK"
exit "$failed"
