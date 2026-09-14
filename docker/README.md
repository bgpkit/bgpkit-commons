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

`pg_inserter` connects with `tokio_postgres` over plain TCP (`NoTls`), the same
as the other BGPKIT data jobs: the password is SCRAM-authenticated but the link
is not encrypted. Run the image on the trusted network that reaches the database
(loopback, the Docker bridge, or the tailnet) and do not expose the database to
untrusted networks. TLS support is not implemented.

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
