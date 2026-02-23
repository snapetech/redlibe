#!/usr/bin/env python3
"""
extract_firefox_token.py — pull a Reddit bearer token (and matching User-Agent)
from a Firefox or LibreWolf install on a remote machine via SSH.

Usage:
    python3 extract_firefox_token.py [SSH_HOST] [OPTIONS]

Examples:
    # Remote machine (default profile auto-detected):
    python3 extract_firefox_token.py user@192.168.1.50

    # Specify a profile path explicitly:
    python3 extract_firefox_token.py user@192.168.1.50 --profile ~/.librewolf/default/

    # Local machine (no SSH):
    python3 extract_firefox_token.py --local

    # Emit shell export lines ready to paste or eval:
    python3 extract_firefox_token.py user@192.168.1.50 --export
    eval "$(python3 extract_firefox_token.py user@192.168.1.50 --export)"

Output (default):
    REDLIB_RAW_TOKEN=<bearer_token>
    REDLIB_USER_AGENT=<matched_firefox_ua>

Requirements:
    pip install paramiko   (only needed for SSH mode; not needed for --local)
"""

import argparse
import base64
import json
import os
import platform
import shutil
import sqlite3
import subprocess
import sys
import tempfile
from pathlib import Path


# ── helpers ──────────────────────────────────────────────────────────────────

def b64url_decode(s: str) -> bytes:
    """Base64url-decode without requiring padding."""
    s += "=" * (-len(s) % 4)
    return base64.urlsafe_b64decode(s)


def decode_token_v2(raw: str) -> str | None:
    """
    Attempt to extract a bearer token from a Reddit token_v2 JWT.

    Reddit's token_v2 cookie is a JWT whose payload may contain one of:
      - "access_token"
      - "token"
      - "accessToken"

    Returns the bearer token string, or None if decoding fails.
    """
    parts = raw.split(".")
    if len(parts) < 2:
        return None
    try:
        payload_bytes = b64url_decode(parts[1])
        payload = json.loads(payload_bytes)
        for field in ("access_token", "token", "accessToken"):
            val = payload.get(field)
            if val and isinstance(val, str):
                return val
    except Exception:
        pass
    return None


def build_firefox_ua(version: str, arch: str) -> str:
    """Construct a Firefox User-Agent string from version + CPU architecture."""
    # Normalise architecture to the string Firefox uses in its UA
    arch_map = {
        "x86_64": "x86_64",
        "amd64": "x86_64",
        "aarch64": "aarch64",
        "arm64": "aarch64",
        "i686": "i686",
        "i386": "i686",
    }
    ua_arch = arch_map.get(arch.lower(), arch)
    major = version.split(".")[0]
    return f"Mozilla/5.0 (X11; Linux {ua_arch}; rv:{version}) Gecko/20100101 Firefox/{version}"


# ── profile discovery ─────────────────────────────────────────────────────────

FIREFOX_PROFILE_DIRS = [
    "~/.mozilla/firefox",
    "~/.librewolf",
    "~/.floorp",           # Floorp (Firefox fork)
    "~/.waterfox",
]

def find_profiles_local() -> list[Path]:
    """Return a list of candidate Firefox/LibreWolf profile directories on the local machine."""
    candidates = []
    for base in FIREFOX_PROFILE_DIRS:
        p = Path(base).expanduser()
        if not p.is_dir():
            continue
        # Look for profiles.ini to enumerate profiles
        ini = p / "profiles.ini"
        if ini.exists():
            # Quick parse — find Path= lines
            for line in ini.read_text(errors="replace").splitlines():
                if line.startswith("Path="):
                    rel = line[5:].strip()
                    profile_path = (p / rel) if not rel.startswith("/") else Path(rel)
                    if profile_path.is_dir():
                        candidates.append(profile_path)
        else:
            # Fallback: any subdir containing cookies.sqlite
            for subdir in p.iterdir():
                if subdir.is_dir() and (subdir / "cookies.sqlite").exists():
                    candidates.append(subdir)
    return candidates


def get_firefox_version_local() -> str | None:
    """Try to detect the local Firefox/LibreWolf version."""
    for binary in ("firefox", "librewolf", "floorp", "waterfox"):
        try:
            out = subprocess.check_output([binary, "--version"], stderr=subprocess.DEVNULL, text=True)
            # e.g. "Mozilla Firefox 133.0"
            for word in out.split():
                if word[0].isdigit() and "." in word:
                    return word.strip()
        except (FileNotFoundError, subprocess.CalledProcessError):
            continue
    return None


# ── cookie extraction ─────────────────────────────────────────────────────────

def extract_reddit_token_from_db(cookies_db: Path) -> str | None:
    """
    Query a Firefox cookies.sqlite for Reddit's token_v2 cookie value.
    Returns the raw cookie value string, or None if not found.
    """
    if not cookies_db.exists():
        return None
    try:
        # Open read-only copy to avoid locking a live Firefox instance
        tmp = tempfile.NamedTemporaryFile(suffix=".sqlite", delete=False)
        shutil.copy2(cookies_db, tmp.name)
        tmp.close()
        try:
            conn = sqlite3.connect(f"file:{tmp.name}?mode=ro", uri=True)
            cur = conn.cursor()
            cur.execute(
                "SELECT value FROM moz_cookies "
                "WHERE host IN ('.reddit.com', 'reddit.com') "
                "  AND name = 'token_v2' "
                "ORDER BY lastAccessed DESC LIMIT 1"
            )
            row = cur.fetchone()
            conn.close()
            return row[0] if row else None
        finally:
            os.unlink(tmp.name)
    except Exception as e:
        print(f"  [warn] Could not read {cookies_db}: {e}", file=sys.stderr)
        return None


# ── SSH mode ──────────────────────────────────────────────────────────────────

def run_ssh(host: str, cmd: str) -> str:
    """Run a shell command on a remote host via ssh(1) and return stdout."""
    result = subprocess.run(
        ["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=10", host, cmd],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(f"SSH command failed: {result.stderr.strip()}")
    return result.stdout.strip()


def scp_file(host: str, remote_path: str, local_dest: Path) -> None:
    """Copy a file from a remote host using scp(1)."""
    result = subprocess.run(
        ["scp", "-q", "-o", "BatchMode=yes", "-o", "ConnectTimeout=10",
         f"{host}:{remote_path}", str(local_dest)],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(f"scp failed: {result.stderr.strip()}")


def find_remote_profiles(host: str) -> list[str]:
    """Enumerate Firefox/LibreWolf profile directories on the remote host."""
    script = (
        "for d in ~/.mozilla/firefox ~/.librewolf ~/.floorp ~/.waterfox; do "
        "  [ -f \"$d/profiles.ini\" ] && "
        "  grep -oP '(?<=^Path=).*' \"$d/profiles.ini\" | "
        "  while read p; do "
        "    case \"$p\" in /*) echo \"$p\";; *) echo \"$d/$p\";; esac; "
        "  done; "
        "done"
    )
    try:
        out = run_ssh(host, script)
        return [l.strip() for l in out.splitlines() if l.strip()]
    except RuntimeError:
        return []


def get_remote_firefox_version(host: str) -> str | None:
    """Detect Firefox/LibreWolf version on the remote host."""
    for binary in ("firefox", "librewolf", "floorp", "waterfox"):
        try:
            out = run_ssh(host, f"{binary} --version 2>/dev/null || true")
            for word in out.split():
                if word and word[0].isdigit() and "." in word:
                    return word.strip()
        except RuntimeError:
            continue
    return None


def get_remote_arch(host: str) -> str:
    """Get the CPU architecture of the remote host."""
    try:
        return run_ssh(host, "uname -m")
    except RuntimeError:
        return "x86_64"


# ── main extraction logic ─────────────────────────────────────────────────────

def extract_local(profile_hint: str | None) -> tuple[str | None, str | None]:
    """
    Extract token and UA from the local machine.
    Returns (bearer_token, user_agent) — either may be None on failure.
    """
    if profile_hint:
        profiles = [Path(profile_hint).expanduser()]
    else:
        profiles = find_profiles_local()
        if not profiles:
            print("No Firefox/LibreWolf profiles found on this machine.", file=sys.stderr)
            return None, None

    token_raw = None
    for profile in profiles:
        cookies_db = profile / "cookies.sqlite"
        val = extract_reddit_token_from_db(cookies_db)
        if val:
            token_raw = val
            print(f"  Found token_v2 in: {cookies_db}", file=sys.stderr)
            break

    if not token_raw:
        print("Reddit token_v2 cookie not found in any profile.", file=sys.stderr)
        return None, None

    bearer = decode_token_v2(token_raw)
    if not bearer:
        print("  Could not decode token_v2 as JWT; treating raw value as bearer token.", file=sys.stderr)
        bearer = token_raw

    version = get_firefox_version_local()
    arch = platform.machine()
    ua = build_firefox_ua(version or "133.0", arch) if version else None

    return bearer, ua


def extract_remote(host: str, profile_hint: str | None) -> tuple[str | None, str | None]:
    """
    Extract token and UA from a remote machine over SSH.
    Returns (bearer_token, user_agent) — either may be None on failure.
    """
    print(f"Connecting to {host} ...", file=sys.stderr)

    arch = get_remote_arch(host)
    version = get_remote_firefox_version(host)

    if profile_hint:
        remote_profiles = [profile_hint]
    else:
        remote_profiles = find_remote_profiles(host)
        if not remote_profiles:
            print("No Firefox/LibreWolf profiles found on remote host.", file=sys.stderr)
            return None, None

    token_raw = None
    for remote_profile in remote_profiles:
        remote_db = f"{remote_profile}/cookies.sqlite"
        with tempfile.NamedTemporaryFile(suffix=".sqlite", delete=False) as tmp:
            tmp_path = Path(tmp.name)
        try:
            print(f"  Fetching {remote_db} ...", file=sys.stderr)
            scp_file(host, remote_db, tmp_path)
            val = extract_reddit_token_from_db(tmp_path)
            if val:
                token_raw = val
                print(f"  Found token_v2 in: {remote_db}", file=sys.stderr)
                break
        except RuntimeError as e:
            print(f"  [skip] {remote_db}: {e}", file=sys.stderr)
        finally:
            if tmp_path.exists():
                os.unlink(tmp_path)

    if not token_raw:
        print("Reddit token_v2 cookie not found in any remote profile.", file=sys.stderr)
        return None, None

    bearer = decode_token_v2(token_raw)
    if not bearer:
        print("  Could not decode token_v2 as JWT; treating raw value as bearer token.", file=sys.stderr)
        bearer = token_raw

    ua = build_firefox_ua(version or "133.0", arch) if version else None

    return bearer, ua


# ── CLI ───────────────────────────────────────────────────────────────────────

def main() -> int:
    parser = argparse.ArgumentParser(
        description="Extract Reddit bearer token from a Firefox/LibreWolf install (local or SSH).",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument(
        "host",
        nargs="?",
        default=None,
        help="SSH host (user@hostname or hostname). Omit for local machine.",
    )
    parser.add_argument(
        "--local",
        action="store_true",
        help="Force local mode (ignore any host argument).",
    )
    parser.add_argument(
        "--profile",
        default=None,
        metavar="PATH",
        help="Path to a specific Firefox profile directory (local or remote).",
    )
    parser.add_argument(
        "--export",
        action="store_true",
        help="Output shell export lines (eval-safe). Suitable for: eval \"$(...)\"",
    )
    parser.add_argument(
        "--token-only",
        action="store_true",
        help="Print only the bearer token, nothing else.",
    )
    args = parser.parse_args()

    if args.local or args.host is None:
        token, ua = extract_local(args.profile)
    else:
        token, ua = extract_remote(args.host, args.profile)

    if not token:
        print("ERROR: Could not extract a Reddit bearer token.", file=sys.stderr)
        return 1

    if args.token_only:
        print(token)
        return 0

    if args.export:
        print(f"export REDLIB_RAW_TOKEN='{token}'")
        if ua:
            print(f"export REDLIB_USER_AGENT='{ua}'")
    else:
        print(f"REDLIB_RAW_TOKEN={token}")
        if ua:
            print(f"REDLIB_USER_AGENT={ua}")
        else:
            print("REDLIB_USER_AGENT=(Firefox version not detected — use default or set manually)", file=sys.stderr)

    return 0


if __name__ == "__main__":
    sys.exit(main())
