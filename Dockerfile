# GitHub action dockerfile

ARG CURL_IMAGE=docker.io/curlimages/curl:7.81.0
ARG RUST_IMAGE=docker.io/rust:1.60-bullseye
ARG RUNTIME_IMAGE=docker.io/rust:1.60-slim-bullseye

FROM $CURL_IMAGE as cargo-deny
ARG CARGO_DENY_VERSION=0.11.1
WORKDIR /tmp
RUN curl --proto '=https' --tlsv1.3 --retry 2 -vsSfL "https://github.com/EmbarkStudios/cargo-deny/releases/download/${CARGO_DENY_VERSION}/cargo-deny-${CARGO_DENY_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
    | tar zvxf - --strip-components=1 "cargo-deny-${CARGO_DENY_VERSION}-x86_64-unknown-linux-musl/cargo-deny"

FROM $RUST_IMAGE as build
WORKDIR /build
COPY . .
RUN cargo build --release && mv target/release/rustsecbot /tmp

FROM $RUNTIME_IMAGE
COPY --from=cargo-deny /tmp/cargo-deny /
COPY --from=build /tmp/rustsecbot /
ENV CARGO_DENY_PATH=/cargo-deny
ENTRYPOINT ["/rustsecbot"]
