# ThreatFlux Rust Dockerfile
# Multi-stage build for single-crate or workspace-based applications.

FROM rust:1.98.0-bookworm AS rust-base

ARG VERSION=0.0.0
ARG BUILD_DATE=unknown
ARG VCS_REF=unknown
ARG BINARY_NAME=vertex
ARG BINARY_PACKAGE=threatflux-vertex-rust-sdk
ARG SBOM_MANIFEST_PATH=Cargo.toml
ARG OCI_IMAGE_TITLE="ThreatFlux Vertex Rust SDK CLI"
ARG OCI_IMAGE_DESCRIPTION="ThreatFlux Vertex AI Rust SDK command-line interface"
ARG OCI_IMAGE_VENDOR=ThreatFlux
ARG OCI_IMAGE_SOURCE=https://github.com/ThreatFlux/vertex_rust_sdk

RUN apt-get update && apt-get install -y \
    ca-certificates \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

FROM rust-base AS builder

RUN useradd -m -u 1000 builder
USER builder
WORKDIR /build

COPY --chown=builder:builder . .

RUN if [ -n "${BINARY_PACKAGE}" ]; then \
      cargo build --release -p "${BINARY_PACKAGE}" --bin "${BINARY_NAME}" --all-features; \
    else \
      cargo build --release --bin "${BINARY_NAME}" --all-features || cargo build --release --all-features; \
    fi

RUN cargo install cargo-cyclonedx --locked --version 0.5.8 && \
    cargo cyclonedx \
      --manifest-path "${SBOM_MANIFEST_PATH}" \
      --all-features \
      --format json \
      --spec-version 1.5 \
      --override-filename "${BINARY_NAME}-sbom"

FROM debian:bookworm-slim AS runtime

ARG VERSION=0.0.0
ARG BUILD_DATE=unknown
ARG VCS_REF=unknown
ARG BINARY_NAME=vertex
ARG OCI_IMAGE_TITLE="ThreatFlux Vertex Rust SDK CLI"
ARG OCI_IMAGE_DESCRIPTION="ThreatFlux Vertex AI Rust SDK command-line interface"
ARG OCI_IMAGE_VENDOR=ThreatFlux
ARG OCI_IMAGE_SOURCE=https://github.com/ThreatFlux/vertex_rust_sdk

LABEL org.opencontainers.image.title="${OCI_IMAGE_TITLE}" \
      org.opencontainers.image.description="${OCI_IMAGE_DESCRIPTION}" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.revision="${VCS_REF}" \
      org.opencontainers.image.vendor="${OCI_IMAGE_VENDOR}" \
      org.opencontainers.image.source="${OCI_IMAGE_SOURCE}"

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    tini \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /usr/share/doc/app \
    && useradd -m -u 1000 app

COPY --from=builder /build/target/release/${BINARY_NAME} /usr/local/bin/app
COPY --from=builder /build/${BINARY_NAME}-sbom.json /usr/share/doc/app/sbom.cdx.json

RUN chown -R app:app /usr/local/bin/app /usr/share/doc/app

USER app
WORKDIR /home/app

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/app", "--version"]

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/usr/local/bin/app"]
