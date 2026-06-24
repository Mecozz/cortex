#!/usr/bin/env bash
set -euo pipefail

MAX_LINES=400
failed=0

while IFS= read -r file; do
  lines=$(wc -l < "$file")
  if [ "$lines" -gt "$MAX_LINES" ]; then
    echo "SIZELIMIT FAIL: $file has $lines lines (limit: $MAX_LINES)"
    failed=1
  fi
done < <(git ls-files | grep -E '\.(rs|ts|svelte)$')

[ "$failed" -eq 0 ] && echo "SIZELIMIT OK"
exit "$failed"
