# syntax=docker/dockerfile:1
FROM rust:1.98-bookworm@sha256:93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e AS builder
RUN rustup component add rustfmt clippy
RUN apt-get update && apt-get install -y --no-install-recommends python3 poppler-utils \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /workspace/src/backend_api
COPY src/backend_api/Cargo.toml src/backend_api/Cargo.lock ./
RUN --mount=type=cache,id=longlivehealth-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=longlivehealth-cargo-git,target=/usr/local/cargo/git \
    cargo fetch --locked
COPY src/archive/ /workspace/src/archive/
COPY src/frontend/ /workspace/src/frontend/
COPY src/shared/Core/Resources/InterfaceChinese.json /workspace/src/shared/Core/Resources/InterfaceChinese.json
COPY src/backend_api/ /workspace/src/backend_api/
COPY tests/backend_api/ /workspace/tests/backend_api/
COPY dist/loinc/Loinc.zip /app/loinc/Loinc.zip
# BuildKit keeps dependency compilation and registry downloads across source edits.
# Export the binary outside the cache mount so the final image can copy it.
RUN --mount=type=cache,id=longlivehealth-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=longlivehealth-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=longlivehealth-cargo-target,target=/workspace/build/backend_api/target \
    cargo fmt --all --check \
    && cargo clippy --locked --release --all-targets -- -D warnings \
    && cargo test --locked --release --all-targets \
    && cargo build --locked --release --bin longlivehealth \
    && mkdir -p /workspace/dist/backend_api \
    && cp /workspace/build/backend_api/target/release/longlivehealth /workspace/dist/backend_api/longlivehealth

FROM debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates tzdata gosu poppler-utils \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /workspace/dist/backend_api/longlivehealth /app/longlivehealth
COPY --from=builder /app/loinc/Loinc.zip /app/loinc/Loinc.zip
COPY dist/android/longlivehealth.apk /app/downloads/longlivehealth.apk
COPY --chmod=755 docker/entrypoint.sh /app/entrypoint.sh
RUN ldd /app/longlivehealth > /tmp/linked-libraries \
    && ! grep -q 'not found' /tmp/linked-libraries \
    && rm /tmp/linked-libraries \
    && /app/longlivehealth --version \
    && /app/longlivehealth --data-dir /data check-loinc \
    && pdftoppm -v
ENV PUID=1000 PGID=1000 UMASK=077 TZ=UTC
EXPOSE 8080
ENTRYPOINT ["/app/entrypoint.sh", "/app/longlivehealth"]
CMD ["--data-dir", "/data", "serve"]
