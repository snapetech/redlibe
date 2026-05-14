#!/usr/bin/env bash
#
# Bug Council remediation baseline — TEMPLATE.
#
# Behavior- and presence-anchored gate. Every closed council finding adds:
#  - a require_pattern line here that asserts the fix symbol is present
#    (text gate);
#  - a behavior test in your test suite that asserts the fix actually does
#    what it claims (behavior gate, see bug-council-behavior-pinning.md).
#
# The two gates together survive renames AND silent regressions.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

failures=0

pass() { printf 'PASS %s\n' "$1"; }
fail() { printf 'FAIL %s\n' "$1" >&2; failures=$((failures + 1)); }

require_file() {
  local path="$1"
  local label="$2"

  if [[ -f "$path" ]]; then
    pass "$label"
  else
    fail "$label: missing $path"
  fi
}

require_pattern() {
  local pattern="$1"
  local path="$2"
  local label="$3"

  if rg -n -U --pcre2 --hidden --glob '!.git/**' "$pattern" "$path" >/dev/null; then
    pass "$label"
  else
    fail "$label"
  fi
}

require_absent_pattern() {
  local pattern="$1"
  local path="$2"
  local label="$3"

  local hit_file
  hit_file="$(mktemp)"

  if rg -n -U --pcre2 --hidden --glob '!.git/**' "$pattern" "$path" >"$hit_file" 2>/dev/null; then
    fail "$label"
    sed 's/^/  /' "$hit_file" >&2
  else
    pass "$label"
  fi

  rm -f "$hit_file"
}

# === Council scaffolding ===================================================
require_file "docs/dev/bug-council-scan-registry.md" "council scan registry exists"
require_file "docs/dev/bug-burndown-ledger.md" "council burndown ledger exists"
require_file "docs/dev/bug-council-severity-schema.md" "council severity/confidence schema exists"
require_file "docs/dev/bug-council-sibling-search.md" "council sibling-search rule exists"
require_file "docs/dev/bug-council-negative-space.md" "council negative-space gate doc exists"
require_file "docs/dev/bug-council-behavior-pinning.md" "council behavior-pinning pattern exists"
require_file "scripts/scan-bug-council-candidates.sh" "candidate scanner exists"
require_file "scripts/check-council-sweep-counts.sh" "sweep-count drift gate exists"
require_file "scripts/check-council-negative-space.sh" "negative-space gate script exists"
require_file "scripts/run-bug-council-all-phases.sh" "all-phases council runner exists"
require_file "scripts/check-bug-council-all-phases.sh" "all-phases council runner registration gate exists"
require_file "scripts/run-council-active-bughunt.sh" "active bughunt runner exists"
require_file "scripts/check-council-active-backlog.sh" "active backlog gate exists"
require_file "docs/dev/bug-council-active-backlog.md" "active backlog exists"
require_pattern "not proof of no bugs" "scripts/run-council-active-bughunt.sh" "active bughunt runner states reports are not no-bug proofs"
require_pattern "Every active-bughunt section must have a row" "docs/dev/bug-council-active-backlog.md" "active backlog documents section coverage rule"
require_pattern "Do not mention council, bughunt, scanners, agents, or other discovery tooling in commit messages" "docs/dev/bug-council-active-backlog.md" "commit wording policy avoids discovery-tool names"
require_pattern "check-council-active-backlog.sh" "scripts/run-bug-council-all-phases.sh" "all-phases runner checks active backlog"

# === All-phases runner registration =======================================
if bash scripts/check-bug-council-all-phases.sh >/dev/null 2>&1; then
  pass "all-phases council runner is registered"
else
  fail "all-phases council runner is not registered; run scripts/check-bug-council-all-phases.sh for details"
fi

# === Active backlog gate ===================================================
if bash scripts/check-council-active-backlog.sh >/dev/null 2>&1; then
  pass "active backlog matches active bughunt report"
else
  fail "active backlog does not match active bughunt report; run scripts/check-council-active-backlog.sh for details"
fi

# === Negative-space gate ===================================================
if bash scripts/check-council-negative-space.sh >/dev/null 2>&1; then
  pass "negative-space gate passes"
else
  fail "negative-space gate failed; run scripts/check-council-negative-space.sh for details"
fi

# === Project-specific fix gates ============================================
# Add one require_pattern (text gate) + behavior test pair per closed finding.
# Examples:
#
# require_pattern "ValidateInputName" "src/path/to/sink" "input validator wired"
# require_pattern "MaxRequestSize" "src/path/to/limit" "request size bound declared"

# === Secret-material absence ===============================================
secret_pattern='-----BEGIN (RSA |DSA |EC |OPENSSH |PGP )?PRIVATE KEY-----|gh[pousr]_[A-Za-z0-9_]{36,}|xox[baprs]-[A-Za-z0-9-]{20,}|AKIA[0-9A-Z]{16}|(?i)(api[_-]?key|access[_-]?token|client[_-]?secret)["'\'']?\s*[:=]\s*["'\''][A-Za-z0-9_./+=-]{24,}["'\'']'
require_absent_pattern "$secret_pattern" "." "tracked text files do not contain high-confidence secret patterns"

if [[ "$failures" -gt 0 ]]; then
  printf '\n%d remediation baseline check(s) failed.\n' "$failures" >&2
  exit 1
fi

printf '\nAll remediation baseline checks passed.\n'
