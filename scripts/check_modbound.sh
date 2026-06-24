#!/usr/bin/env bash
set -euo pipefail

# Feature modules must not import from each other directly.
# All inter-module communication must go through BUS (crate::core::bus).
# Importing from infrastructure modules (core, db) is allowed.

SRC_DIR="src-tauri/src"
INFRA=("core" "db")
failed=0

feature_mods=()
for d in "$SRC_DIR"/*/; do
  [ -d "$d" ] || continue
  name=$(basename "$d")
  is_infra=0
  for i in "${INFRA[@]}"; do
    [ "$name" = "$i" ] && is_infra=1 && break
  done
  [ "$is_infra" -eq 0 ] && feature_mods+=("$name")
done

for mod in "${feature_mods[@]}"; do
  for other in "${feature_mods[@]}"; do
    [ "$mod" = "$other" ] && continue
    if grep -rqE "use crate::${other}(::|;)" "$SRC_DIR/$mod/" 2>/dev/null; then
      echo "MODBOUND FAIL: '$mod' directly imports from '$other' — route through BUS instead"
      failed=1
    fi
  done
done

[ "$failed" -eq 0 ] && echo "MODBOUND OK"
exit "$failed"
