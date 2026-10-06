# ThreatFlux Rust Dockerfile
# Multi-stage build for single-crate or workspace-based applications.
#
# Follows ThreatFlux/rust-cicd-template's canonical Dockerfile: a Debian 13
# (trixie) Rust builder and a distroless Debian 13 runtime with no shell,
# package manager or coreutils. The vertex CLI needs none of them: TLS uses
# rustls (or OpenSSL vendored into the binary with --all-features), and
# distroless/cc supplies glibc, libgcc, OpenSSL and the CA certificates.
# Repo-specific values are build ARGs; .github/workflows/docker.yml supplies
# them from repository variables.
#
# Base images are pinned by digest for reproducibility (Scorecard
# Pinned-Dependencies). Refresh with:
#   docker buildx imagetools inspect <image> | awk '/^Digest:/{print $2}'
# Dependabot refreshes the Rust builder; refresh the runtime digest with the
# command above when it is updated.

# rust 1.99.0 on Debian 13 (trixie); multi-arch index digest
FROM rust:1.99.0-trixie@sha256:15ad267e7a4cb2dce5905c90c76765adb6714945c5ea6d7c82673897a5e4067b AS rust-base

ARG VERSION=0.0.0
ARG BUILD_DATE=unknown
ARG VCS_REF=unknown
ARG BINARY_NAME=vertex
ARG BINARY_PACKAGE=threatflux-vertex-rust-sdk
ARG CLI_NAME=vertex
ARG SBOM_MANIFEST_PATH=Cargo.toml
ARG OCI_IMAGE_TITLE="ThreatFlux Vertex Rust SDK CLI"
ARG OCI_IMAGE_DESCRIPTION="ThreatFlux Vertex AI Rust SDK command-line interface"
ARG OCI_IMAGE_VENDOR=ThreatFlux
ARG OCI_IMAGE_SOURCE=https://github.com/ThreatFlux/vertex_rust_sdk

# tini is installed here so the runtime stage can copy it out: distroless ships
# no init, and PID 1 must reap zombies and forward signals. Package revisions
# follow the pinned base image's Debian 13 repositories.
# hadolint ignore=DL3008
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    pkg-config \
    libssl-dev \
    tini \
    && rm -rf /var/lib/apt/lists/*

FROM rust-base AS builder

RUN useradd -m -u 1000 builder
USER builder
WORKDIR /build

ENV CARGO_HOME=/home/builder/.cargo
ENV PATH="/home/builder/.cargo/bin:${PATH}"

COPY --chown=builder:builder . .

RUN rustc --version --verbose && cargo --version && \
    if [ -n "${BINARY_PACKAGE}" ]; then \
      cargo build --locked --release -p "${BINARY_PACKAGE}" --bin "${BINARY_NAME}" --all-features; \
    else \
      cargo build --locked --release --bin "${BINARY_NAME}" --all-features || cargo build --locked --release --all-features; \
    fi

# cargo-cyclonedx writes the SBOM beside the manifest it was handed, which in a
# workspace is not necessarily /build. The find normalizes the output location.
RUN cargo install cargo-cyclonedx --locked --version 0.5.9 && \
    cargo cyclonedx \
      --manifest-path "${SBOM_MANIFEST_PATH}" \
      --all-features \
      --format json \
      --spec-version 1.5 \
      --override-filename "${BINARY_NAME}-sbom" && \
    find /build -name "${BINARY_NAME}-sbom.json" -exec cp {} /build/sbom.cdx.json \; -quit && \
    test -s /build/sbom.cdx.json

# Stage the runtime filesystem under a fixed layout.
#
# The binary is staged as `app` at a FIXED path: exec-form ENTRYPOINT and
# HEALTHCHECK do not expand build ARGs. A symlink keeps the CLI's own name
# (`vertex`) available on PATH.
#
# Writable directories are created here because the distroless runtime has no
# shell to mkdir with; ownership is applied on the way in via COPY --chown.
RUN mkdir -p /home/builder/out/bin /home/builder/out/doc \
             /home/builder/runtime-skel/data \
             /home/builder/runtime-skel/config \
             /home/builder/runtime-skel/output && \
    cp "target/release/${BINARY_NAME}" /home/builder/out/bin/app && \
    if [ "${CLI_NAME}" != "app" ]; then \
      ln -s app "/home/builder/out/bin/${CLI_NAME}"; \
    fi && \
    cp /build/sbom.cdx.json /home/builder/out/doc/sbom.cdx.json

# distroless cc on Debian 13, nonroot tag (uid/gid 65532); multi-arch index digest
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:e792ab3d241a468a4fd7519ddbbebe66b49b5f365771716ea688ad40b6c6f1c2 AS runtime

ARG VERSION=0.0.0
ARG BUILD_DATE=unknown
ARG VCS_REF=unknown
ARG OCI_IMAGE_TITLE="ThreatFlux Vertex Rust SDK CLI"
ARG OCI_IMAGE_DESCRIPTION="ThreatFlux Vertex AI Rust SDK command-line interface"
ARG OCI_IMAGE_VENDOR=ThreatFlux
ARG OCI_IMAGE_SOURCE=https://github.com/ThreatFlux/vertex_rust_sdk
ARG OCI_IMAGE_LICENSES=MIT
ARG OCI_IMAGE_DOCUMENTATION=https://github.com/ThreatFlux/vertex_rust_sdk/blob/main/CLI.md

LABEL org.opencontainers.image.title="${OCI_IMAGE_TITLE}" \
      org.opencontainers.image.description="${OCI_IMAGE_DESCRIPTION}" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.revision="${VCS_REF}" \
      org.opencontainers.image.vendor="${OCI_IMAGE_VENDOR}" \
      org.opencontainers.image.source="${OCI_IMAGE_SOURCE}" \
      org.opencontainers.image.licenses="${OCI_IMAGE_LICENSES}" \
      org.opencontainers.image.documentation="${OCI_IMAGE_DOCUMENTATION}"

COPY --from=builder /usr/bin/tini /usr/bin/tini

# The binary and SBOM stay root-owned so the runtime user cannot modify them;
# only the working directories belong to the nonroot user.
COPY --from=builder --chown=0:0 /home/builder/out/bin/ /usr/local/bin/
COPY --from=builder --chown=0:0 /home/builder/out/doc/ /usr/share/doc/app/
COPY --from=builder --chown=65532:65532 /home/builder/runtime-skel/data /data
COPY --from=builder --chown=65532:65532 /home/builder/runtime-skel/config /config
COPY --from=builder --chown=65532:65532 /home/builder/runtime-skel/output /output

USER 65532:65532
WORKDIR /data

# Exec form (there is no shell in distroless); a nonzero exit means unhealthy.
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/app", "--version"]

# Arguments go straight to the CLI: `docker run <image> --help`.
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/app"]
