#!/usr/bin/env python3
"""Shared platform, package-version and cleanup rules for container checks."""

import argparse
from contextlib import contextmanager
from itertools import zip_longest
import re
# Fixed Docker argv without shell execution; see docs/security/validation-script-review.md.
import subprocess  # nosec B404
import sys


DOCKER_TIMEOUT = 300
OPENSSL_TIMEOUT = 30
CLEANUP_TIMEOUT = 30
CA_PACKAGE_FLOOR = "20250419+deb13u1"


def normalize_platform(value):
    """Normalize supported baseline variants while rejecting other architectures."""
    value = value.strip()
    if re.fullmatch(r"linux/amd64(?:/v[1-4])?", value):
        return "linux/amd64"
    if re.fullmatch(r"linux/arm64(?:/v8(?:\.[0-9]+)?)?", value):
        return "linux/arm64"
    raise ValueError(f"unsupported Docker platform: {value!r}; expected linux/amd64 or linux/arm64")


def platform_argument(value):
    """Expose platform validation through argparse's normal error reporting."""
    try:
        return normalize_platform(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError(str(error)) from error


def normalize_platforms(value):
    """Validate every requested platform and deduplicate its canonical spelling."""
    return ",".join(dict.fromkeys(normalize_platform(part) for part in value.split(",")))


def version_parts(value):
    """Parse Debian epoch, upstream version and optional package revision."""
    epoch, separator, remaining = value.partition(":")
    if not separator:
        epoch, remaining = "0", value
    upstream, separator, revision = remaining.rpartition("-")
    if not separator:
        upstream, revision = remaining, "0"
    if (not re.fullmatch(r"[0-9]+", epoch)
        or not re.fullmatch(r"[0-9][A-Za-z0-9.+:~\-]*", upstream)
        or not re.fullmatch(r"[A-Za-z0-9+.~]+", revision)):
        raise ValueError(f"invalid Debian package version: {value!r}")
    return int(epoch), upstream, revision


def character_order(character):
    """Order Debian version characters: tilde, end, letters, then punctuation."""
    if character == "~":
        return -1
    if not character:
        return 0
    return ord(character) if character.isalpha() else ord(character) + 256


def compare_text(left, right):
    """Compare nondigit version segments using Debian's special ordering."""
    for first, second in zip_longest(left, right, fillvalue=""):
        difference = character_order(first) - character_order(second)
        if difference:
            return difference
    return 0


def compare_part(left, right):
    """Compare alternating text and numeric runs without integer-size limits."""
    while left or right:
        left_text = re.match(r"[^0-9]*", left).group()
        right_text = re.match(r"[^0-9]*", right).group()
        difference = compare_text(left_text, right_text)
        if difference:
            return difference
        left = left[len(left_text):]
        right = right[len(right_text):]
        left_digits = re.match(r"[0-9]*", left).group()
        right_digits = re.match(r"[0-9]*", right).group()
        first, second = left_digits.lstrip("0"), right_digits.lstrip("0")
        first_key, second_key = (len(first), first), (len(second), second)
        if first_key != second_key:
            return 1 if first_key > second_key else -1
        left, right = left[len(left_digits):], right[len(right_digits):]
    return 0


def compare_debian_versions(left, right):
    """Compare Debian versions including epochs, prereleases and revisions."""
    left_epoch, left_upstream, left_revision = version_parts(left)
    right_epoch, right_upstream, right_revision = version_parts(right)
    if left_epoch != right_epoch:
        return 1 if left_epoch > right_epoch else -1
    return compare_part(left_upstream, right_upstream) or compare_part(left_revision, right_revision)


@contextmanager
def cleanup_container(container, force=False):
    """Bound cleanup and preserve an existing failure while reporting cleanup errors."""
    failed = False
    try:
        yield
    except BaseException:
        failed = True
        raise
    finally:
        command = ["docker", "rm"] + (["-f"] if force else []) + [container]
        try:
            # Docker's own container ID is one argv item; no shell is involved.
            # nosemgrep
            subprocess.run(command, stdout=subprocess.DEVNULL, check=True, timeout=CLEANUP_TIMEOUT)  # nosec B603
        except (OSError, subprocess.SubprocessError) as error:
            if not failed:
                raise
            print(f"Container cleanup also failed: {error}", file=sys.stderr)


def main():
    """Print a canonical platform list for workflow selection and manifest checks."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("platforms")
    args = parser.parse_args()
    try:
        print(normalize_platforms(args.platforms))
    except ValueError as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
