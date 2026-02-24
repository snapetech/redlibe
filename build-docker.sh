#!/usr/bin/env bash
# Build the redlibe Docker image. Uses host network so the build container
# can resolve index.crates.io (fixes DNS when the host uses Pi-hole or
# Docker's default bridge DNS doesn't work).
set -euo pipefail
cd "$(dirname "$0")"
docker build --network=host -f Dockerfile.ubuntu -t redlibe:latest . "$@"
