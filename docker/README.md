# Docker images

Container images for the binaries shipped by this repository. One Dockerfile per
image; the build context is always the repository root. Images are built locally
(or on any host with Docker) from this repository; registry publishing is not
wired up.

| Image | Dockerfile | Binary | Build features |
|-------|------------|--------|----------------|
| `bgpkit/pg-inserter` | `docker/pg-inserter.Dockerfile` | `pg_inserter` | `pg-inserter-cli` |

## Credential contract

The images hold no credentials and no `.env` file. Everything secret is supplied
at run time through environment variables, so the same image runs against any
database:

| Variable | Required | Purpose |
|----------|----------|---------|
| `DATABASE_URL` | yes | PostgreSQL connection string for the target database |
| `PEERINGDB_API_KEY` | for `peeringdb` | PeeringDB API key; without it the full mirror is rate limited |

Passing them:

```sh
# env file, mode 600, never committed (.gitignore covers docker/*.env)
docker run --rm --env-file pg-inserter.env bgpkit/pg-inserter:dev asndata

# single variable (keep secrets out of shell history where possible)
docker run --rm -e DATABASE_URL="$DATABASE_URL" bgpkit/pg-inserter:dev irr

# orchestrator: use the platform's secret store and pass the same names
```

A `.env` file can also be mounted read-only at the working directory; the binary
loads it via `dotenvy`:

```sh
docker run --rm -v /etc/bgpkit/pg-inserter.env:/data/.env:ro bgpkit/pg-inserter:dev asndata
```

Never bake a connection string or API key into the image, and prefer
`--env-file` over `-e` for anything that contains a password.

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
