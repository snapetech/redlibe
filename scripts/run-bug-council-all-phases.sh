#!/usr/bin/env bash
#
# Bug Council all-phases runner — TEMPLATE.
#
# This is the command agents should run for a council cycle. Adapt the
# project-specific commands below, but keep the shape: fresh inventory first,
# then every regression/process/semantic/fuzz gate in one command.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

out_dir="${COUNCIL_OUT_DIR:-.council}"
mkdir -p "$out_dir"
scan_out="$out_dir/latest-candidate-counts.md"

printf '==> Fresh candidate inventory\n'
bash scripts/scan-bug-council-candidates.sh | tee "$scan_out"

printf '\n==> Active bughunt discovery queue\n'
bash scripts/run-council-active-bughunt.sh

printf '\n==> Process and regression gates\n'
bash scripts/check-remediation-baseline.sh
bash scripts/check-council-active-backlog.sh
bash scripts/check-council-sweep-counts.sh
bash scripts/check-council-negative-space.sh

printf '\n==> Semantic analyzers\n'
# Replace with your repo's semantic analyzer tests.
# dotnet test tests/CouncilAnalyzers.Tests/CouncilAnalyzers.Tests.csproj --no-restore
# cargo test -p council-lints
# npm run lint:council

printf '\n==> Calibration and adversarial corpus\n'
# Replace with your repo's calibration/fuzz commands.
# dotnet test tests/CouncilAnalyzers.Calibration/CouncilAnalyzers.Calibration.csproj --no-restore
# dotnet test tests/Project.Tests/Project.Tests.csproj --no-restore --filter Category=Fuzz
# cargo test adversarial_

printf '\n==> Pending council phases\n'
if rg -n '^\| [0-9]+ \| .* \| Pending \|' docs/dev/bug-council-phases.md; then
  printf '\nCouncil is not complete: pending phases remain. Pick the first pending row above and burn it down.\n' >&2
  exit 2
fi

printf '\nAll bug council phases passed. Candidate counts saved to %s.\n' "$scan_out"
printf 'Council verdict boundary: this is not proof of no bugs. It means the current calibrated lenses, active backlog, closed sweep counts, fuzz corpus, build, and vulnerability scan passed.\n'
