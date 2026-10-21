#!/usr/bin/env python3
"""Verify the shipped image has static executables and no shared runtimes."""

import argparse
import json
from pathlib import PurePosixPath
import struct
import subprocess
import tarfile
import tempfile


def check_elf(name, data, architecture):
    if data[:6] != b"\x7fELF\x02\x01":
        raise ValueError(f"{name}: expected a little-endian ELF64 executable")
    if name == "usr/local/bin/app" and struct.unpack_from("<H", data, 16)[0] != 3:
        raise ValueError(f"{name}: static executable must retain PIE address randomization")
    machine = struct.unpack_from("<H", data, 18)[0]
    if machine != {"amd64": 62, "arm64": 183}[architecture]:
        raise ValueError(f"{name}: wrong ELF architecture {machine}")
    offset = struct.unpack_from("<Q", data, 32)[0]
    size, count = struct.unpack_from("<HH", data, 54)
    if not count or size < 56:
        raise ValueError(f"{name}: missing or invalid ELF program headers")
    for index in range(count):
        header = struct.unpack_from("<IIQQQQQQ", data, offset + index * size)
        if header[0] == 3:  # PT_INTERP: needs an external ELF loader.
            raise ValueError(f"{name}: has a dynamic interpreter")
        if header[0] == 2:  # PT_DYNAMIC: static PIE can have this, but no NEEDED.
            for entry in range(header[2], header[2] + header[5], 16):
                tag, _ = struct.unpack_from("<qQ", data, entry)
                if tag == 0:
                    break
                if tag == 1:
                    raise ValueError(f"{name}: has a shared-library dependency")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image")
    parser.add_argument("--platform")
    args = parser.parse_args()
    create = ["docker", "create"]
    if args.platform:
        create += ["--platform", args.platform]
    container = subprocess.check_output(create + [args.image], text=True).strip()
    try:
        info = json.loads(subprocess.check_output(["docker", "inspect", container]))[0]
        image = json.loads(subprocess.check_output(["docker", "image", "inspect", info["Image"]]))[0]
        if info["Config"]["User"] != "65532:65532":
            raise ValueError("runtime must use the nonroot 65532:65532 account")
        if info["Config"]["Entrypoint"] != ["/usr/bin/tini", "--", "/usr/local/bin/app"]:
            raise ValueError("runtime must launch the CLI through its static init")
        if "SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt" not in info["Config"]["Env"]:
            raise ValueError("runtime must configure the shipped CA trust bundle")
        binaries = {"usr/local/bin/app", "usr/bin/tini"}
        seen = set()
        with tempfile.TemporaryFile() as archive:
            subprocess.run(["docker", "export", container], stdout=archive, check=True)
            archive.seek(0)
            with tarfile.open(fileobj=archive) as filesystem:
                files = {str(PurePosixPath(m.name)): m for m in filesystem if m.isfile()}
                for name, member in files.items():
                    if member.mode & 0o6000:
                        raise ValueError(f"privileged executable mode shipped: {name}")
                    if ".so" in PurePosixPath(name).name:
                        raise ValueError(f"shared runtime library shipped: {name}")
                    with filesystem.extractfile(member) as source:
                        magic = source.read(4)
                        if magic == b"\x7fELF":
                            if name not in binaries:
                                raise ValueError(f"unexpected runtime executable: {name}")
                            source.seek(0)
                            check_elf(name, source.read(), image["Architecture"])
                            if member.uid != 0 or member.mode & 0o022:
                                raise ValueError(f"{name}: executable must be root-owned and not writable by others")
                            seen.add(name)
                if seen != binaries:
                    raise ValueError(f"missing static executables: {binaries - seen}")
                with filesystem.extractfile(files["etc/ssl/certs/ca-certificates.crt"]) as source:
                    if b"-----BEGIN CERTIFICATE-----" not in source.read():
                        raise ValueError("missing CA certificates")
                with filesystem.extractfile(files["usr/share/doc/app/sbom.cdx.json"]) as source:
                    sbom = json.load(source)
                    if sbom.get("bomFormat") != "CycloneDX" or not sbom.get("components"):
                        raise ValueError("missing application SBOM components")
                with filesystem.extractfile(files["usr/share/doc/app/native-build.txt"]) as source:
                    provenance = source.read().decode()
                    target = {"amd64": "x86_64", "arm64": "aarch64"}[image["Architecture"]]
                    if (f"host: {target}-unknown-linux-musl" not in provenance
                        or "/usr/lib/libc.a" not in provenance
                        or "self-contained/libc.a" in provenance):
                        raise ValueError("missing system-musl linker provenance")
        print(f"Static runtime verified: {args.image} ({image['Architecture']})")
    finally:
        subprocess.run(["docker", "rm", container], stdout=subprocess.DEVNULL, check=True)


if __name__ == "__main__":
    main()
