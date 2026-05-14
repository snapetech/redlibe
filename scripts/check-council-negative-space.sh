#!/usr/bin/env bash
#
# Bug Council negative-space gate — TEMPLATE.
#
# Asserts TWO halves for every declared trust boundary in
# docs/dev/bug-council-negative-space.md:
#
#   1. assert_validator_present  — the validator symbol still exists in the
#                                  sink file. Catches "validator deleted."
#   2. assert_baseline_anchor    — a remediation-baseline check still
#                                  references the same anchor. Catches
#                                  "remediation gate silently removed."
#
# Both halves are required. The single-half version of this gate was itself
# a council bug: a baseline pattern could be removed while the gate kept
# passing because it only looked at the sink file. The two-half pattern was
# discovered when slskdN's hooks strengthened it; this template carries the
# strengthened pattern by default.
#
# Wired into scripts/check-remediation-baseline.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

failures=0

pass() { printf 'PASS %s\n' "$1"; }
fail() { printf 'FAIL %s\n' "$1" >&2; failures=$((failures + 1)); }

assert_validator_present() {
  local boundary="$1"
  local sink="$2"
  local symbol="$3"

  if [[ ! -e "$sink" ]]; then
    fail "negative-space: sink missing for boundary [$boundary]: $sink"
    return
  fi

  if rg -n --fixed-strings -- "$symbol" "$sink" >/dev/null; then
    pass "negative-space: [$boundary] $symbol present in $sink"
  else
    fail "negative-space: [$boundary] $symbol missing from $sink"
  fi
}

assert_baseline_anchor() {
  local boundary="$1"
  local anchor="$2"

  if rg -n --fixed-strings -- "$anchor" scripts/check-remediation-baseline.sh >/dev/null; then
    pass "negative-space: [$boundary] baseline anchor '$anchor' is registered"
  else
    fail "negative-space: [$boundary] baseline anchor '$anchor' is missing from check-remediation-baseline.sh"
  fi
}

# Replace the placeholder rows below with one PAIR per trust boundary
# declared in docs/dev/bug-council-negative-space.md. Both halves required.
#
# assert_validator_present \
#   "boundary-name" \
#   "src/path/to/sink.ext" \
#   "ValidatorSymbol"
# assert_baseline_anchor "boundary-name" "ValidatorSymbol"

if [[ "$failures" -gt 0 ]]; then
  printf '\n%d negative-space gate check(s) failed.\n' "$failures" >&2
  exit 1
fi

printf '\nAll negative-space gate checks passed.\n'
