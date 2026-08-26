# The API, for Render (or any container host).
#
# Only `airlock-api` ships. The Reader stays out deliberately: on a hosted
# instance the API runs in stub mode, because a free-tier Reader that spins
# down on idle would make every novel-recipient transfer fail closed and hold
# — correct behaviour that looks exactly like a broken deployment. The
# fail-closed beat belongs on the laptop, where a real process dies in front
# of people.

# Pinned to the toolchain the workspace is actually built and tested against.
# A floating tag would move under us between deploys, and an older pin would
# be a build failure discovered on Render rather than here.
FROM rust:1.98-slim AS builder
WORKDIR /build

# Manifests first, so a source-only change doesn't refetch the registry.
COPY Cargo.toml Cargo.lock ./
COPY crates/core/Cargo.toml     crates/core/
COPY crates/policy/Cargo.toml   crates/policy/
COPY crates/agents/Cargo.toml   crates/agents/
COPY crates/runtime/Cargo.toml  crates/runtime/
COPY crates/api/Cargo.toml      crates/api/
COPY crates/reader/Cargo.toml   crates/reader/
COPY evals/Cargo.toml           evals/

# Stub sources so the manifests resolve on their own and the dependency build
# lands in its own cached layer.
RUN mkdir -p crates/core/src crates/policy/src crates/agents/src \
             crates/runtime/src crates/api/src crates/reader/src evals/src \
 && echo "" | tee crates/core/src/lib.rs crates/policy/src/lib.rs \
        crates/agents/src/lib.rs crates/runtime/src/lib.rs \
        crates/api/src/lib.rs evals/src/lib.rs > /dev/null \
 && echo "fn main() {}" | tee crates/api/src/main.rs \
        crates/reader/src/main.rs > /dev/null \
 && cargo build --release -p airlock-api \
 && rm -rf crates evals

COPY crates crates
COPY evals evals

# Touch so cargo doesn't reuse the stub fingerprints above.
RUN find crates evals -name '*.rs' -exec touch {} + \
 && cargo build --release -p airlock-api

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Nothing here needs root.
RUN useradd --system --create-home --uid 10001 airlock
USER airlock

COPY --from=builder /build/target/release/airlock-api /usr/local/bin/airlock-api

# Render provides PORT and routes to it; BIND_ALL opts into 0.0.0.0, which
# main.rs will not do on its own.
ENV BIND_ALL=1 PORT=10000 RUST_LOG=info
EXPOSE 10000

CMD ["airlock-api"]
