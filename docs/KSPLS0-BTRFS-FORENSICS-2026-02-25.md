# kspls0 Btrfs Corruption Forensics / Offline Test Runbook (2026-02-25)

This runbook is for investigating recurring Btrfs corruption on `kspls0` (`/dev/nvme0n1p2`, root fs) that causes the filesystem to remount read-only and takes down `k3s`/Postgres.

## Confirmed facts (from live evidence)

- Root fs remounted read-only:
  - `/dev/nvme0n1p2 on / type btrfs (ro,...)`
- Immediate trigger:
  - `BTRFS critical ... corrupt leaf ... bad key order`
- Btrfs device stats show corruption count but no I/O errors:
  - `corruption_errs > 0`
  - `read_io_errs = 0`, `write_io_errs = 0`, `flush_io_errs = 0`
- NVMe SMART (latest captured) did not report media errors:
  - `media_errors = 0`
  - `num_err_log_entries = 0`
- Corruption existed before the RO remount:
  - Earlier same boot: Btrfs scrub found checksum errors across multiple unrelated files (Plex, k3s/containerd layers, Ollama blobs, user files)

## Likely cause classes (ranked)

1. RAM / memory-path corruption (especially non-ECC or unstable timings/OC/undervolt)
2. Prior unsafe shutdown / hard reset causing latent corruption that later surfaced
3. PCIe/NVMe firmware/controller path instability
4. Kernel/Btrfs bug
5. NVMe media failure (less supported by current SMART/error logs, but not ruled out)

## Rules of engagement (important)

- Prefer evidence capture before repair.
- Do not run mutating Btrfs repair commands until you have backups / images.
- `btrfs check --repair` is a last resort.
- Single-device Btrfs scrub detects corruption but cannot heal it.

## Phase 1: While system is booted (if it comes back RW)

Run these first and save output off-host (`scp`, NAS, another machine).

### 1. Capture kernel + Btrfs evidence

```bash
sudo journalctl -k --no-pager -b > /root/forensics/journal-kernel-boot.txt
sudo journalctl -k --no-pager | grep -Ei 'btrfs|nvme|I/O error|corrupt|edac|mce|pcie|aer' \
  > /root/forensics/journal-kernel-storage-filtered.txt
sudo dmesg -T > /root/forensics/dmesg.txt
```

### 2. Capture Btrfs health and layout state (non-mutating)

```bash
sudo btrfs filesystem show / > /root/forensics/btrfs-filesystem-show.txt
sudo btrfs filesystem usage -T / > /root/forensics/btrfs-filesystem-usage.txt
sudo btrfs device stats / > /root/forensics/btrfs-device-stats.txt
sudo btrfs subvolume list -t / > /root/forensics/btrfs-subvolume-list.txt
sudo btrfs inspect-internal dump-super -f /dev/nvme0n1p2 > /root/forensics/btrfs-super.txt
```

Optional (large output, but useful if metadata damage keeps recurring):

```bash
sudo btrfs inspect-internal tree-stats / > /root/forensics/btrfs-tree-stats.txt
```

### 3. Capture NVMe health / firmware / logs

```bash
sudo nvme list > /root/forensics/nvme-list.txt
sudo nvme id-ctrl /dev/nvme0 > /root/forensics/nvme-id-ctrl.txt
sudo nvme smart-log /dev/nvme0 > /root/forensics/nvme-smart-log.txt
sudo nvme error-log /dev/nvme0 -e 256 > /root/forensics/nvme-error-log.txt
sudo smartctl -x /dev/nvme0 > /root/forensics/smartctl-nvme0.txt
```

### 4. Capture hardware error telemetry (if available)

```bash
sudo journalctl -k --no-pager | grep -Ei 'mce|machine check|edac|ras' \
  > /root/forensics/journal-kernel-ras.txt
command -v ras-mc-ctl >/dev/null && sudo ras-mc-ctl --errors --summary \
  > /root/forensics/ras-mc-summary.txt || true
command -v edac-util >/dev/null && sudo edac-util -v \
  > /root/forensics/edac-util.txt || true
```

### 5. Capture system firmware and kernel versions

```bash
uname -a > /root/forensics/uname.txt
cat /proc/cmdline > /root/forensics/proc-cmdline.txt
sudo dmidecode -t bios > /root/forensics/dmidecode-bios.txt
sudo dmidecode -t memory > /root/forensics/dmidecode-memory.txt
```

### 6. If filesystem is RW and stable enough: scrub (read-only-ish validation, but records state)

This is not a repair on single-device Btrfs, but it confirms scope and locations.

```bash
sudo btrfs scrub start -Bd /
sudo btrfs scrub status /
```

Capture output. Expect checksum errors if corruption persists.

## Phase 2: Offline testing (recommended)

This is the safest path to determine root cause and stop recurrence.

### A. Memory / CPU stability (highest priority)

Reason: repeated corruption across unrelated files with clean NVMe SMART often points to RAM/path corruption.

1. Reset BIOS to known-stable defaults (temporarily)
   - Disable EXPO/XMP
   - Remove undervolt/OC/PBO tweaks
   - Disable memory overclocking

2. Run memory tests
   - `memtest86+` or MemTest86 from boot media
   - Run multiple full passes (overnight, ideally 8+ hours)

3. If ECC exists
   - Check corrected/uncorrected counts in BIOS/IPMI/EDAC

4. Optional stress in Linux (after system is stable)
   - `stress-ng --vm ...`
   - `y-cruncher` / `prime95` blend
   - Watch for segfaults / MCE / EDAC increments

### B. NVMe firmware / storage path checks

1. Record current firmware:

```bash
sudo nvme id-ctrl /dev/nvme0 | grep -E 'fr|mn|sn'
```

2. Check vendor firmware updates (WD/SanDisk/etc. if applicable)
3. Check motherboard BIOS/chipset updates
4. If drive is behind a riser/switch/backplane, inspect:
   - seating
   - power
   - cooling
   - PCIe link stability

5. If possible, run vendor diagnostic or long SMART self-test (if supported)

### C. Btrfs offline checks (preserve-first)

1. Boot a maintenance/live environment.
2. Do **not** auto-mount the root filesystem RW.
3. Capture image/metadata if possible before invasive commands:

```bash
# Metadata dump (safer than full image if space is limited)
sudo btrfs-image -r /dev/nvme0n1p2 /mnt/backup/kspls0-root.btrfs-image
```

4. Run non-destructive checks first:

```bash
sudo btrfs check --readonly /dev/nvme0n1p2
sudo btrfs rescue super-recover -v /dev/nvme0n1p2   # inspect output before writing
sudo btrfs rescue chunk-recover -y /dev/nvme0n1p2   # only if chunk tree issues are indicated
```

Notes:
- `super-recover` and `chunk-recover` can write changes; use only after preserving evidence/image.
- `btrfs check --repair` is last resort and should be taken only after backup + explicit decision.

5. If mountable read-only in live env, capture more evidence:

```bash
sudo mount -o ro,subvol=@ /dev/nvme0n1p2 /mnt
sudo btrfs device stats /mnt
sudo btrfs scrub start -Bd /mnt
```

### D. Power event / unsafe shutdown analysis

The NVMe reports many historical unsafe shutdowns.

Check:
- UPS health/runtime and event logs
- PSU quality/cabling
- motherboard power stability
- any recent hard resets/watchdogs

If unsafe shutdowns continue increasing unexpectedly, that is a leading contributor.

## How to make recurrence stop (not just recover once)

### Immediate mitigations

- Move critical services (k3s datastore/Postgres) off `kspls0` until root cause is fixed
- Avoid writing heavily to the damaged filesystem before evidence capture
- Stop automated scrubs/maintenance on a repeatedly failing node until forensic baseline is captured

### Structural fixes

- Stabilize memory path (BIOS defaults, memory tests, replace suspect DIMM)
- Update BIOS + NVMe firmware
- Validate PCIe path / cabling / risers / cooling
- Consider ECC RAM for this role if not already
- Reduce single-point failure:
  - mirrored Btrfs profile for metadata/data, or
  - separate durable storage for k3s datastore and critical state

## Emergency cluster restore (preserve evidence first)

Use this when the priority is getting cluster services back without destroying forensic evidence on `kspls0`.

### Goal

- Restore control plane + GitLab/Redlib/OpenWebUI availability
- Minimize writes to the suspect `kspls0` root filesystem
- Preserve evidence for later root-cause analysis

### 0. Freeze unnecessary writes on `kspls0` (if reachable)

- Do not run cleanup scripts (`k3s-killall.sh`) on a read-only / unstable fs unless required for shutdown
- Do not run `btrfs check --repair`
- Do not restart high-write apps on `kspls0`

### 1. Bring back k3s datastore dependency first (Postgres on `kspls0`)

`kspld0` k3s depends on Postgres at `192.168.50.85:5432`.

Options:

1. If `kspls0` boots cleanly enough:
   - start only the host Postgres service
   - confirm `tcp/5432` is reachable from `kspld0`
   - let `kspld0` k3s finish starting
2. If `kspls0` root remains unstable:
   - restore/move the k3s datastore Postgres to another node (temporary failover)
   - point `K3S_DATASTORE_ENDPOINT` at the alternate Postgres instance

### 2. Keep `kspls0` out of cluster scheduling until root cause is addressed

From a healthy control-plane node (once k3s API is back):

```bash
kubectl cordon kspls0
kubectl drain kspls0 --ignore-daemonsets --delete-emptydir-data --force
```

If drain is too disruptive, at minimum `cordon` it.

### 3. Re-home critical workloads away from `kspls0`

Prioritize:

- k3s datastore / Postgres dependency
- GitLab Shell / GitLab components
- any ingress-critical services

Quick tactical fix example (used during this incident):

- delete `gitlab-shell` pods so they reschedule onto `kspld0`
- ensure Traefik TCP port `22` is exposed and `gitlab-shell` endpoints are `Ready`

### 4. Restore GitLab SSH path

Verify all three:

```bash
# On cluster
kubectl -n kube-system get svc traefik
kubectl -n gitlab get ingressroutetcp gitlab-gitlab-shell -o yaml
kubectl -n gitlab get endpoints gitlab-gitlab-shell

# From client
ssh -T git@gitlab.home
git push origin main
```

### 5. Preserve forensic snapshot before any invasive repair

If `kspls0` is reachable and can mount root read-only:

- copy `/var/log/journal`
- copy `/root/forensics/*` outputs (from Phase 1)
- capture `btrfs-image -r` if possible

### 6. Only then schedule offline maintenance

- reboot into maintenance/live media
- run offline checks and hardware tests (Phase 2 in this runbook)
- keep `kspls0` cordoned until validated stable

## Interpretation guide (what results mean)

- `NVMe media_errors=0`, `error_log_entries=0`, but recurring Btrfs checksum corruption:
  - strongly consider RAM/controller/kernel path corruption
- Rising `read_io_errs` / `media_errors` / NVMe error log entries:
  - stronger drive/controller case
- Memtest failures:
  - treat RAM instability as primary until fixed
- Btrfs scrub errors recur on new files after repair:
  - corruption source is still active (not just old damage)

## Current incident-specific notes (2026-02-25)

- `kspls0` root remounted `ro` due Btrfs metadata corruption at `13:40:36`
- `k3s` on `kspls0` failed because root was RO
- `kspld0` k3s also became unavailable because its datastore Postgres was on `kspls0` (`192.168.50.85:5432`)
- GitLab SSH outage was compounded by:
  - Traefik losing TCP port 22 entrypoint (fixed)
  - GitLab Shell pods scheduled on `kspls0` and marked unready when node went `NotReady`
