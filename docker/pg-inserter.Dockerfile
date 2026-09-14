# syntax=docker/dockerfile:1
#
# pg_inserter image: bulk-load BGP reference data into PostgreSQL.
#
# Build with the repository root as context:
#   docker build -f docker/pg-inserter.Dockerfile -t bgpkit/pg-inserter .
#
# Credentials are supplied as environment variables at run time and are never
# baked into the image:
#   DATABASE_URL       PostgreSQL connection string (required)
#   PEERINGDB_API_KEY  PeeringDB API key (required for the full peeringdb mirror)
#
# See docker/README.md for run examples, the credential contract, and the
# plaintext-connection (NoTls) limitation.

FROM rust:1.97 AS build
WORKDIR /build

# Copy the manifests and source needed to build the pg_inserter bin. The
# crate has no committed Cargo.lock (see .gitignore), so dependencies resolve
# at build time.
COPY Cargo.toml ./
COPY src ./src
COPY examples ./examples
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --no-default-features --features pg-inserter-cli --bin pg_inserter

FROM debian:trixie-slim

# ca-certificates is required: every data source (RIR delegated stats, IRR
# dumps, PeeringDB API) is fetched over HTTPS.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# The loader needs no privileges: it binds no port and writes nothing outside
# /tmp. Fixed uid/gid so a mounted `.env` can be made readable with chown.
RUN groupadd --gid 10001 loader \
    && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin loader \
    && install -d -o loader -g loader /data

COPY --from=build /build/target/release/pg_inserter /usr/local/bin/pg_inserter

LABEL org.opencontainers.image.title="pg_inserter" \
      org.opencontainers.image.description="Load BGP reference data from bgpkit-commons into PostgreSQL" \
      org.opencontainers.image.source="https://github.com/bgpkit/bgpkit-commons" \
      org.opencontainers.image.licenses="MIT"

# Working directory for a mounted `.env` (dotenvy reads it from the working
# directory). Environment variables remain the primary credential source.
USER loader
WORKDIR /data

ENTRYPOINT ["/usr/local/bin/pg_inserter"]
CMD ["--help"]
