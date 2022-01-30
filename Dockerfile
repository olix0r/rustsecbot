# GitHub action dockerfile

ARG CURL_IMAGE=docker.io/curlimages/curl:7.81.0
ARG RUST_IMAGE=docker.io/rust:1.58.0-bullseye
ARG RUNTIME_IMAGE=gcr.io/distroless/cc:nonroot

FROM $CURL_IMAGE as cargo-deny
ARG CARGO_DENY_VERSION=0.11.1
WORKDIR /out
RUN curl --proto '=https' --tlsv1.3 -vsSfL "https://github.com/EmbarkStudios/cargo-deny/releases/download/${CARGO_DENY_VERSION}/cargo-deny-${CARGO_DENY_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
    | tar zvxf - --strip-components=1 "cargo-deny-${CARGO_DENY_VERSION}-x86_64-unknown-linux-musl/cargo-deny"

FROM $RUST_IMAGE as build
WORKDIR /build
RUN mkdir /out
COPY . .
RUN --mount=type=cache,from=docker.io/rust:1.58.0-bullseye,source=/usr/local/cargo,target=/usr/local/cargo \
    --mount=type=cache,target=target \
    cargo build --release && mv target/release/rustsecbot /out

FROM $RUNTIME_IMAGE
COPY --from=cargo-deny /out/cargo-deny /
COPY --from=build /out/rustsecbot /
ENV CARGO_DENY_PATH=/cargo-deny
ENTRYPOINT ["/rustsecbot"]
