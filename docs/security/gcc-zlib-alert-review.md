# GCC and zlib code-scanning review

This records the original glibc-based artifacts. See the [subsequent remediation](remediation.md) for the static runtime fix and current alert dispositions.

Reviewed 2026-10-21. Scope: all ten open GCC-family/zlib alerts in the supplied GitHub API export. No alerts were dismissed. TP means the vulnerable component is present, independently of application reachability. FP means the alert's component/version/target mapping does not match the shipped vulnerable code; it does **not** mean merely that the CLI does not call it.

## Artifact and observed evidence

All ten alerts refer to commit `3b990a75b9cef5fc43dffa45ad8851078177998f`, workflow `.github/workflows/docker.yml:scan`. Inspected the published image `ghcr.io/threatflux/vertex_rust_sdk:3b990a7`, index digest `sha256:324c223852180cc4d704255060d4f0e12a5c1bfa30df4864107e1e7b62446e81`, by pulling its immutable platform manifests, creating stopped containers, and exporting files (no application execution):

| Platform | Manifest digest |
| --- | --- |
| linux/amd64 | `sha256:13578551123923484a9ed91632f25ff81b1c272093c407da7baf0927bf5ca9d5` |
| linux/arm64 | `sha256:ab7e79b71273131f7ecb5c9099c94fc34c2920f6d9fcf1a17377dfcc395c6429` |

Both platforms contain `libstdc++6`, `libgomp1`, `libgcc-s1`, and `gcc-14-base` version `14.2.0-19`, and `zlib1g` version `1:1.3.dfsg+really1.3.1-1+b1`. The `app` binary on both platforms imports only `libgcc_s.so.1`, `libm.so.6`, and `libc.so.6` (amd64 also explicitly lists its loader). `tini` imports libc. Neither executable directly imports libstdc++, libgomp, or libz. This reduces demonstrated reachability but alone cannot invalidate an installed-package finding. See [captured package and ELF evidence](gcc-zlib-image-evidence.txt).

## Per-alert verdicts

| Alert | CVE | Reported package | Verdict | Reason and action |
| --- | --- | --- | --- | --- |
| [358](https://github.com/ThreatFlux/vertex_rust_sdk/security/code-scanning/358) | CVE-2026-95619 | libstdc++6 | FP, target-specific | Actual aligned-new implementation calls `posix_memalign`; affected allocation rounding is absent. See G1. |
| [349](https://github.com/ThreatFlux/vertex_rust_sdk/security/code-scanning/349) | CVE-2026-95619 | libgomp1 | FP, component mapping | A libstdc++ allocation flaw is propagated through the shared gcc-14 source-package name; libgomp is the OpenMP runtime, not the affected library. See G1. |
| [340](https://github.com/ThreatFlux/vertex_rust_sdk/security/code-scanning/340) | CVE-2026-95619 | libgcc-s1 | FP, component mapping | GCC support/unwind runtime is not libstdc++ aligned new. See G1. |
| [336](https://github.com/ThreatFlux/vertex_rust_sdk/security/code-scanning/336) | CVE-2026-95619 | gcc-14-base | FP, component mapping | Common GCC metadata/documentation package does not implement aligned new. See G1. |
| [357](https://github.com/ThreatFlux/vertex_rust_sdk/security/code-scanning/357) | CVE-2026-102010 | libstdc++6 | FP for this runtime package; compiled consumers require separate review | Fix is to a PBDS template header, not the libstdc++ shared object. The runtime image ships no C++ development headers. See G2. |
| [348](https://github.com/ThreatFlux/vertex_rust_sdk/security/code-scanning/348) | CVE-2026-102010 | libgomp1 | FP, component mapping | PBDS C++ template implementation is not the OpenMP runtime. See G2. |
| [339](https://github.com/ThreatFlux/vertex_rust_sdk/security/code-scanning/339) | CVE-2026-102010 | libgcc-s1 | FP, component mapping | PBDS template implementation is not the GCC support/unwind runtime. See G2. |
| [335](https://github.com/ThreatFlux/vertex_rust_sdk/security/code-scanning/335) | CVE-2026-102010 | gcc-14-base | FP, component mapping | Base package contains no affected PBDS template implementation. See G2. |
| [316](https://github.com/ThreatFlux/vertex_rust_sdk/security/code-scanning/316) | CVE-2026-85091 | zlib1g | Undetermined, affected-version disagreement | Shipped 1.3.1 predates introduced nonblocking gz writer machinery; Debian still labels it vulnerable. Retain open pending vendor adjudication. See Z1. |
| [98](https://github.com/ThreatFlux/vertex_rust_sdk/security/code-scanning/98) | CVE-2026-27171 | zlib1g | TP, application reachability unproven | Installed 1.3.1 contains arithmetic-shift CRC combine loop without negative-length guard on both platforms. Fix plan Z2. |

## G1: aligned-new integer overflow

The [GCC 14 upstream fix](https://github.com/gcc-mirror/gcc/commit/200183d4ab5d57575f3dfebc6af3baf649910b9e) limits the issue to allocation implementations that round sizes before allocation. Its explanation explicitly excludes modern `posix_memalign` implementations. Disassembly of `_ZnwmSt11align_val_t` in **both actual image libraries** shows the size passed to `posix_memalign` without the affected alignment rounding: amd64 call at `0xb3c81`, arm64 call at `0xaa218`. Thus alert 358 is an architecture/build-specific FP even though [Debian's source-level tracker](https://security-tracker.debian.org/tracker/CVE-2026-95619) marks gcc-14 `14.2.0-19` vulnerable. The other three binary packages are different components of that source package. Keeping GCC current remains prudent, but changing Rust versions cannot fix an OS source-package mapping.

Validation on any replacement image: repeat `objdump -d --disassemble-symbols=_ZnwmSt11align_val_t` for each architecture, or verify that the package has a vendor fix. Reassess if a future target uses `aligned_alloc`, `_aligned_malloc`, or the fallback allocator. Debian currently lists a fix in unstable `14.4.0-3`; this is not evidence of a trixie stable fix.

## G2: PBDS binary-heap use-after-free

The [upstream patch](https://github.com/gcc-mirror/gcc/commit/aaa8351f4d2e636f9680a1f0a8ebc2f0a60611e6) changes `include/ext/pb_ds/detail/binary_heap_/erase_fn_imps.hpp`: the template must update its entry pointer after reallocation. The relevant code is instantiated into C++ consumers when built, rather than repaired by replacing `libstdc++.so.6` at runtime. The detected runtime binary packages do not provide that development header. Therefore these four package-level alerts are component-mapping FPs, not evidence that all software built with GCC is safe. [Debian](https://security-tracker.debian.org/tracker/CVE-2026-102010) still tracks the GCC 14 source package as affected in trixie and lists unstable `14.4.0-3` as fixed.

Repository source is Rust; its directly linked runtime libraries show no C++ runtime dependency. No demonstrated use of GNU PBDS was found. If C++ consumers or build-time tooling using PBDS are introduced, build with the patched headers and rerun upstream `priority_queues_erase_if.cc` under ASan; merely updating the runtime shared library would be insufficient. Do not apply a blanket CVE suppression to an image containing development headers or arbitrary precompiled C++ code.

## Z1: nonblocking gzip writer overflow

[Debian's tracker](https://security-tracker.debian.org/tracker/CVE-2026-85091) currently labels trixie's 1.3.1 source vulnerable, while also documenting that support for nonblocking gz devices was introduced in [commit 81cc0beb](https://github.com/madler/zlib/commit/81cc0bebedd935daeb81b0b6e475d8786b51af3d) in 1.3.1.2. The [upstream fix](https://github.com/madler/zlib/commit/df84af25dc1942490e1d1c899a07619152a46148) fixes that later machinery. This is an explicit evidence conflict, not an accepted Debian adjudication.

Downloaded the [official Debian orig source archive](https://deb.debian.org/debian/pool/main/z/zlib/zlib_1.3.dfsg+really1.3.1.orig.tar.gz), SHA256 `60dd315c07f616887caa029408308a018ace66e3d142726a97db164b3b8f69fb`; `gzwrite.c` has neither `gz_vacate` nor `state->again`. The corresponding [Debian packaging archive](https://deb.debian.org/debian/pool/main/z/zlib/zlib_1.3.dfsg+really1.3.1-1.debian.tar.xz) has an empty patch series. Shipped amd64 libz SHA256 is `85590dd58edf5445e18bc7193e5ebc01ac5841f1ae187e97705a662e90c6421e`; arm64 is `6fe00acadf8119ff245855e5dcc7f26c6c0ffaf4bfa2baa88feab58f23727d65`. Source and image version evidence exclude the named later machinery, but do not establish that all older affected paths are absent. The public [upstream issue](https://github.com/madler/zlib/issues/1310) also records this range disagreement; the maintainer confirmed the later fix, without explicitly adjudicating Debian 1.3.1.

A later [Debian maintainer comment](https://github.com/madler/zlib/issues/1310) reports PoC-related behavior before the nonblocking feature and requests upstream clarification. There is still no definitive vendor adjudication for Debian 1.3.1. The correct disposition is **undetermined**, not a proven FP. Keep the alert open until vendor clarification or removal/upgrading of the package. If upgrading to 1.3.2 to fix Z2, require the nonblocking-writer fix as well (or use a later stable release containing it), because plain 1.3.2 introduces this affected path.

## Z2: TP fix and validation plan for alert 98

The [upstream CRC fix](https://github.com/madler/zlib/commit/ba829a458576d1ff0f26fc7230c6de816d1f6a77) adds checks for negative lengths in `crc32_combine64` and `crc32_combine_gen64`, released in 1.3.2. [Debian](https://security-tracker.debian.org/tracker/CVE-2026-27171) confirms trixie 1.3.1 remains affected; its no-DSA/minor classification is not a fix. Both shipped ELF libraries contain the unguarded arithmetic-shift loop. The CLI's lack of a dynamic libz dependency limits proven exploitation, not package validity.

1. Preferred minimal runtime: remove zlib and unused C++/OpenMP libraries by selecting a maintained compatible runtime image with only the CLI's actual ELF dependencies, CA certificates, and tini. Verify the full dependency closure before changing the base. A Rust/cargo-only update does not remediate this TP.
2. If retaining libz: use a supported stable vendor package containing the CRC fix **and** the Z1 nonblocking-writer fix, or backport both through a maintained package build. Do not mix Debian unstable packages into the stable image simply because Debian lists an unstable fixed revision.
3. Build both linux/amd64 and linux/arm64. Export the final filesystems, record package versions and image digests, and confirm libz is absent or prove both fixes in the shipped binaries/source provenance. For a retained library, run bounded regression calls to both CRC combine APIs with negative lengths and valid nonnegative controls; negative calls must terminate promptly. Run the gzip nonblocking writer regression for the retained version as well.
4. Run the CLI `--version`/`--help`, a configured authentication/TLS/API smoke test, and the image's updated Trivy scan with a current database on each architecture. Upload SARIF in the existing workflow/category. Consider alert 98 resolved only after the replacement artifact is scanned; this review does not claim an already-deployed fix.
