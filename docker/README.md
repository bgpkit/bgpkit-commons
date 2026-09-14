# Docker images

Container images for the binaries shipped by this repository. One Dockerfile per
image; the build context is always the repository root. Images are built locally
(or on any host with Docker) from this repository; registry publishing is not
wired up.

| Image | Dockerfile | Binary | Build features |
|-------|------------|--------|----------------|
| `bgpkit/pg-inserter` | `docker/pg-inserter.Dockerfile` | `pg_inserter` | `pg-inserter-cli` |

Images run as an unprivileged user (uid/gid 10001).

## Credential contract

The images hold no credentials and no `.env` file. Everything secret is supplied
at run time through environment variables:

| Variable | Required | Purpose |
|----------|----------|---------|
| `DATABASE_URL` | yes | PostgreSQL connection string for the target database |
| `PEERINGDB_API_KEY` | for `peeringdb` | PeeringDB API key; without it the full mirror is rate limited |

Passing them:

```sh
# env file in the ignored location, mode 600, never committed
docker run --rm --env-file docker/pg-inserter.env bgpkit/pg-inserter:dev asndata

# single variable (keep secrets out of shell history where possible)
docker run --rm -e DATABASE_URL="$DATABASE_URL" bgpkit/pg-inserter:dev irr

# orchestrator: use the platform's secret store and pass the same names
```

Copy `docker/pg-inserter.env.example` to `docker/pg-inserter.env` first: that
path is covered by `.gitignore` (`docker/*.env`) and by `.dockerignore`.

A `.env` file can also be mounted read-only at the working directory, where
`dotenvy` picks it up. The file must be readable by uid 10001:

```sh
chown 10001 docker/pg-inserter.env && chmod 600 docker/pg-inserter.env
docker run --rm -v "$PWD/docker/pg-inserter.env:/data/.env:ro" bgpkit/pg-inserter:dev asndata
```

Prefer `--env-file` over `-e` for anything containing a password, and never bake
a connection string or API key into the image.

## Connection security

TLS follows PostgreSQL's `sslmode` parameter in `DATABASE_URL`, so the same
image covers a loopback/bridge connection and a remote one:

| `sslmode` | Behavior |
|-----------|----------|
| `disable` | never negotiate TLS |
| `prefer` (default) | use TLS when the server offers it, plaintext otherwise; certificates are not verified |
| `require` | TLS mandatory; certificates are not verified unless `sslrootcert` is given, in which case the chain is verified |
| `verify-ca` | TLS mandatory; chain verified against `sslrootcert` (or the system roots), hostname not checked |
| `verify-full` | TLS mandatory; chain and hostname verified |

`sslrootcert=/path/to/ca.pem` supplies the trust anchors for the verifying
modes; without it they use the image's system trust store. `sslmode=allow` is
rejected (use `prefer`). `prefer` and `require` encrypt without authenticating
the peer, which protects against passive observers only; use `verify-ca` or
`verify-full` when the server certificate has to be proven.

## Build and smoke test

```sh
docker build -f docker/pg-inserter.Dockerfile -t bgpkit/pg-inserter:dev .

docker run --rm bgpkit/pg-inserter:dev --help
docker run --rm bgpkit/pg-inserter:dev asnames   # exits 10: DATABASE_URL is not set
```

## Adding another image

1. Add `docker/<name>.Dockerfile` building the binary with its own feature flag.
2. Tag the result under its own image name and add a row to the catalog table
   above.
