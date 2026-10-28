FROM rust:latest AS build
WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY src ./src
ARG BIN_NAME=camera-fetcher
RUN cargo build --locked --release \
    && install -D "target/release/${BIN_NAME}" /out/app

FROM debian:stable-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home app

WORKDIR /app
RUN mkdir -p config images && chown -R app:app /app
COPY --from=build /out/app /usr/local/bin/app

USER app
ENTRYPOINT ["/usr/local/bin/app"]