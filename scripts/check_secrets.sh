#!/usr/bin/env bash
set -euo pipefail

failed=0

check_pattern() {
  local desc="$1"
  local pattern="$2"
  local matches
  matches=$(git ls-files -z | xargs -0 grep -lE "$pattern" 2>/dev/null \
    | grep -v "scripts/check_secrets.sh" || true)
  if [ -n "$matches" ]; then
    echo "SECRETS FAIL [$desc] found in:"
    echo "$matches" | sed 's/^/  /'
    failed=1
  fi
}

check_pattern "GitHub PAT"       'ghp_[A-Za-z0-9]{36}'
check_pattern "Anthropic API key" 'sk-ant-[A-Za-z0-9_-]{20,}'
check_pattern "OpenAI API key"    'sk-[A-Za-z0-9]{48,}'
check_pattern "Hardcoded secret"  '(password|secret|api_key)\s*=\s*"[^"]{12,}"'

[ "$failed" -eq 0 ] && echo "SECRETS OK"
exit "$failed"
