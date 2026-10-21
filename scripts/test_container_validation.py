"""Regression tests for the container review findings; no Docker daemon required."""

from contextlib import redirect_stderr
import io
import json
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import check_static_image
import container_validation as validation
import smoke_container_https


class PlatformTests(unittest.TestCase):
    """Keep supported platform variants consistent with published manifest names."""

    def test_supported_variants(self):
        """Treat supported variants as their compatible baseline architecture."""
        for value, expected in [
            ("linux/amd64", "linux/amd64"), ("linux/amd64/v1", "linux/amd64"),
            ("linux/amd64/v3", "linux/amd64"), ("linux/arm64", "linux/arm64"),
            ("linux/arm64/v8", "linux/arm64"), ("linux/arm64/v8.2", "linux/arm64"),
        ]:
            with self.subTest(value=value):
                self.assertEqual(validation.normalize_platform(value), expected)

    def test_invalid_platforms_fail_the_entire_list(self):
        """Reject unsupported architectures and malformed lists before publishing."""
        for value in ["linux/ppc64le", "linux/riscv64", "windows/amd64", "linux/arm/v7",
                      "linux/arm64/v7", "linux/amd64/v99", "", "linux/amd64,", "linux/amd64,linux/s390x"]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                validation.normalize_platforms(value)

    def test_list_and_manifest_spellings_agree(self):
        """Whitespace, duplicates and manifest variants must not cause false failures."""
        requested = validation.normalize_platforms(" linux/amd64, linux/arm64,linux/arm64/v8 ")
        manifest = validation.normalize_platforms("linux/amd64,linux/arm64/v8")
        self.assertEqual(requested, manifest)


class PackageVersionTests(unittest.TestCase):
    """Exercise the CA floor and Debian epoch, prerelease and revision ordering."""

    def test_ca_package_floor(self):
        """Reject every older CA version while retaining fixed and newer releases."""
        for version in ["20200101", "20250418", "20250419", "20250419+deb13u0",
                        "20250419+deb13u1~rc1", "0:20250419"]:
            with self.subTest(version=version), self.assertRaises(ValueError):
                check_static_image.check_ca_package(self.package_record(version))
        for version in ["20250419+deb13u1", "20250419+deb13u2", "20250419+deb13u10",
                        "20260419", "20250419+deb13u1-1", "1:20200101"]:
            with self.subTest(version=version):
                check_static_image.check_ca_package(self.package_record(version))

    @staticmethod
    def package_record(version):
        """Create the same installed supplier stanza shipped in the final image."""
        return f"Package: ca-certificates\nStatus: install ok installed\nVersion: {version}\n"

    def test_debian_ordering(self):
        """Cover lexical traps where ordinary string or semantic-version ordering fails."""
        for older, newer in [("1.0~rc1", "1.0"), ("1.0~~", "1.0~"), ("1.9", "1.10"),
                             ("1.0-2", "1.0-10"), ("1.0a", "1.0+"), ("2.0", "1:1.0"),
                             ("1.0-1~bpo1", "1.0-1"), ("1.0-a", "1.0-b")]:
            with self.subTest(older=older, newer=newer):
                self.assertLess(validation.compare_debian_versions(older, newer), 0)
                self.assertGreater(validation.compare_debian_versions(newer, older), 0)
        self.assertEqual(validation.compare_debian_versions("1.01", "1.1-0"), 0)

    def test_invalid_package_records(self):
        """Missing, malformed and uninstalled package provenance must fail closed."""
        for record in [self.package_record(""), self.package_record("invalid"), self.package_record("1.0-"),
                       self.package_record("x:1.0"), self.package_record("١.0"),
                       self.package_record("20250419+deb13u1").replace("ca-certificates", "other"),
                       self.package_record("20250419+deb13u1").replace("install ok installed", "deinstall ok config-files")]:
            with self.subTest(record=record), self.assertRaises(ValueError):
                check_static_image.check_ca_package(record)


class CleanupTests(unittest.TestCase):
    """Preserve primary errors, bound cleanup and fail if cleanup alone fails."""

    def test_cleanup_is_bounded(self):
        """Always run Docker removal with a finite timeout and checked exit status."""
        with patch.object(validation.subprocess, "run") as run:
            with validation.cleanup_container("fixture-container", force=True):
                pass
        self.assertEqual(run.call_args.args[0], ["docker", "rm", "-f", "fixture-container"])
        self.assertEqual(run.call_args.kwargs["timeout"], validation.CLEANUP_TIMEOUT)
        self.assertTrue(run.call_args.kwargs["check"])

    def test_primary_failure_survives_cleanup_failure(self):
        """A failed or timed-out cleanup cannot replace the original validation error."""
        errors = [validation.subprocess.CalledProcessError(1, ["docker", "rm"]),
                  validation.subprocess.TimeoutExpired(["docker", "rm"], 30), OSError("daemon unavailable")]
        for cleanup_error in errors:
            primary = ValueError("original validation failure")
            stderr = io.StringIO()
            with self.subTest(error=cleanup_error), redirect_stderr(stderr):
                with patch.object(validation.subprocess, "run", side_effect=cleanup_error):
                    with self.assertRaises(ValueError) as raised:
                        with validation.cleanup_container("fixture-container"):
                            raise primary
                self.assertIs(raised.exception, primary)
                self.assertIn("Container cleanup also failed", stderr.getvalue())

    def test_cleanup_failure_after_success_is_not_hidden(self):
        """A cleanup-only failure must still fail validation rather than leak silently."""
        error = validation.subprocess.CalledProcessError(1, ["docker", "rm"])
        with patch.object(validation.subprocess, "run", side_effect=error):
            with self.assertRaises(validation.subprocess.CalledProcessError):
                with validation.cleanup_container("fixture-container"):
                    pass

    def test_static_check_preserves_failure_and_bounds_commands(self):
        """Exercise the static check's real entry point with failed validation and cleanup."""
        outputs = ["fixture-container", json.dumps([{"Image": "sha256:fixture"}]),
                   json.dumps([{"Architecture": "arm64"}])]
        cleanup_error = validation.subprocess.TimeoutExpired(["docker", "rm"], 30)
        with patch("sys.argv", ["check_static_image.py", "fixture-image"]), redirect_stderr(io.StringIO()):
            with patch.object(validation.subprocess, "check_output", side_effect=outputs) as output:
                with patch.object(validation.subprocess, "run", side_effect=cleanup_error):
                    with patch.object(check_static_image, "check_configuration", side_effect=ValueError("original")):
                        with self.assertRaisesRegex(ValueError, "original"):
                            check_static_image.main()
                for call in output.call_args_list:
                    self.assertEqual(call.kwargs["timeout"], validation.DOCKER_TIMEOUT)

    def test_signal_failure_releases_server_and_preserves_error(self):
        """A failed signal probe must release the server even if forced removal fails."""
        fixture = smoke_container_https.ContainerFixture(Path("."), "fixture-image", "linux/arm64")
        fixture.server = SimpleNamespace(server_port=443)
        error = validation.subprocess.CalledProcessError(1, ["docker", "rm", "-f"])
        with patch.object(fixture.holding, "wait", return_value=False), redirect_stderr(io.StringIO()):
            with patch.object(validation.subprocess, "check_output", return_value="fixture-container") as output:
                with patch.object(validation.subprocess, "run", side_effect=error):
                    with self.assertRaisesRegex(RuntimeError, "did not reach"):
                        fixture.check_signal()
                self.assertEqual(output.call_args.kwargs["timeout"], validation.DOCKER_TIMEOUT)
        self.assertTrue(fixture.release.is_set())


if __name__ == "__main__":
    unittest.main()
