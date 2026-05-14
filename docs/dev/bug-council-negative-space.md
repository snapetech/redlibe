# Bug Council Negative-Space Gate — TEMPLATE

Copy into `docs/dev/bug-council-negative-space.md` and replace the boundaries below with the trust boundaries of your codebase.

The candidate scanner finds **call sites that exist**. It cannot find **call sites that should exist but don't** — for example, a new public boundary that takes untrusted input but never calls a validator. This document declares your trust boundaries and the validator each one must run, so a missing validator is itself a CI failure.

The gate is enforced by `scripts/check-council-negative-space.sh`, which is invoked from `scripts/check-remediation-baseline.sh`.

## Boundaries

A boundary is a code seam where data crosses from a less-trusted source into your runtime. For every boundary, this document records:

- **Source** — where the data comes from.
- **Sink file(s)** — the file(s) where the boundary is implemented.
- **Required validator** — a symbol that must appear in the sink file. The symbol's presence does not prove correctness; it proves the developer thought about the boundary. Behavior is pinned separately by `bug-council-behavior-pinning.md`.

| Boundary | Source | Sink file(s) | Required validator |
| --- | --- | --- | --- |
| _replace_with_your_boundary_ | _network input_ | `src/path/to/sink.ext` | `ValidateInputName` |

## Adding a new boundary

When a new public surface accepts untrusted input:

1. Add a row above with the boundary, the file(s) it lives in, and the validator symbol you've placed in those file(s).
2. Add a PAIR of lines to `scripts/check-council-negative-space.sh`:
   - `assert_validator_present "<boundary>" "<sink>" "<symbol>"` — catches "validator deleted."
   - `assert_baseline_anchor "<boundary>" "<symbol>"` — catches "remediation gate silently removed." Both halves required.
3. Add a behavior-pinned test per `bug-council-behavior-pinning.md`.

## Why two halves

The single-half version of this gate (validator-symbol-only) was itself a council bug. A maintainer could remove the corresponding `require_pattern` in `scripts/check-remediation-baseline.sh` while the symbol still existed, and the gate would pass — silently weakening the fix gate. The two-half pattern requires both the symbol AND the baseline check to remain, so a half-removal fails CI.

## Removing a boundary

Removing a row requires a council sweep entry explaining why the boundary no longer exists (refactored away, code deleted, source moved to trusted). The remediation baseline must be updated in the same change.

## Why this matters

Most council catches in mature codebases are of the shape "a guard exists for boundary A, was forgotten for boundary B." The negative-space gate inverts the search: instead of sweeping all call sites for missing guards, it lists every boundary by name and asserts the guard symbol is in place. That makes "I added a new boundary and forgot to think about it" the failure mode that's hardest to commit.
