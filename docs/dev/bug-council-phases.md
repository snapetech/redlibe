# Bug Council Phase Tracker — TEMPLATE

Copy into `docs/dev/bug-council-phases.md` when a multi-phase council upgrade is in flight. The tracker is the resumable plan: every agent that picks up the work updates this file as phases progress.

| # | Name | Status | Owner | Exit criteria |
| --- | --- | --- | --- | --- |
| 1 | Council process upgrades | _Pending / In progress / Done_ | _agent_ | Severity/confidence schema added, sibling-search rule documented, negative-space gate doc + script, behavior-pinning pattern documented. |
| 2 | Semantic analyzer beachhead | _Pending / In progress / Done_ | _agent_ | One language-appropriate semantic analyzer (Roslyn / Clippy / ESLint) implementing a taint-to-allocation or taint-to-path lens, with tests. |
| 3 | Adversarial fuzz harness | _Pending / In progress / Done_ | _agent_ | Roundtrip + adversarial-input property tests for protocol/parsing surfaces, gated by the baseline. |
| 4 | Broaden first semantic lens | _Pending / In progress / Done_ | _agent_ | First lens covers more than the first MVP source/sink pair; sources, sinks, and validators are documented and behavior-pinned. |
| 5 | Add second semantic lens | _Pending / In progress / Done_ | _agent_ | A second lens catches a distinct bug shape so zero-finding runs are less dependent on one narrow detector. |
| 6 | Mutation/calibration fixture | _Pending / In progress / Done_ | _agent_ | Dedicated fixture project/corpus contains known-bad and known-good examples for every semantic lens. |
| 7 | Multi-seed adversarial corpus | _Pending / In progress / Done_ | _agent_ | Fuzz harness runs multiple deterministic seeds plus explicit hostile corpus inputs; baseline gates the corpus and seed list. |
| 8 | All-phases council runner | _Pending / In progress / Done_ | _agent_ | `scripts/run-bug-council-all-phases.sh` runs inventory, remediation, sweep-count drift, negative-space, semantic analyzers, calibration, fuzz/adversarial corpus, and pending-phase checks in one command; `scripts/check-bug-council-all-phases.sh` is wired into remediation. |
| 9 | Active backlog pile gate | _Pending / In progress / Done_ | _agent_ | `docs/dev/bug-council-active-backlog.md` records every active-discovery pile with current count/status, `scripts/check-council-active-backlog.sh` fails on stale or untriaged rows, and the all-phases runner invokes the gate every cycle. |
| 10 | Additional semantic lens batch | _Pending / In progress / Done_ | _agent_ | Add several distinct semantic lenses in one batch, such as tainted protocol offsets, paths, timeouts, endpoints, enum/status conversions, slice bounds, diagnostic/log-line text, outbound messages, cache keys, crypto trust material, dynamic execution, parser runtimes, resource capacities, and buffer operations, with unit tests and calibration. |
| 11 | _project-specific phase_ | _Pending_ | _agent_ | _exit criteria_ |

## How to resume

1. Read recent product/fix commits and the ledger to see what landed. Commit messages must describe the product change, not the discovery tool or process.
2. Read this file's phase table to find the first non-Done row.
3. Run `bash scripts/run-bug-council-all-phases.sh`; do not substitute a single remediation gate for a council cycle.
4. Pick up the phase, update its status to In Progress, and follow its exit checklist.

If a phase has been partially completed by another agent, treat the on-disk artifacts as the source of truth and reconcile this tracker against them rather than re-doing work.
