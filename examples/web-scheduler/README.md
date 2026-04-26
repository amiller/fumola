# scheduler-svc

A multi-party 1:1 meeting scheduler, written in fumola, packaged as a
self-contained container that deploys onto **any** tee-daemon CVM via
the existing `dockerfile` runtime — no platform changes required.

```
apps/scheduler-svc/
├── Dockerfile        ← multi-stage: builds fumola, bundles server + handler
├── server.py         ← aiohttp HTTP server; runs locally and in docker
├── handle.fumola     ← public func handle(method, path, body)
├── index.html        ← static preview UI (round picker + DCG tree)
├── project.json      ← runtime: "dockerfile", listen.port: 8080
└── README.md         ← this file
```

## Run locally (no docker)

```bash
# One-time: build fumola at the repo root
( cd ../.. && cargo build --release )

# Serve
PORT=8003 python3 server.py
```

Open `http://localhost:8003/` for the UI. The server walks up from its
own directory looking for `target/release/fumola`, so it picks up the
release build automatically.

## Run in docker

```bash
docker build -t scheduler-svc:local .
docker run --rm -p 8080:8080 scheduler-svc:local
```

Open `http://localhost:8080/`. The image bakes fumola in at
`/usr/local/bin/fumola`; nothing else is needed.

## Deploy to a tee-daemon CVM

This project uses tee-daemon's `dockerfile` runtime — **no fumola-specific
runtime exists or is needed in tee-daemon**. Push this directory to a
public git repo, then:

```bash
TOKEN=...
CVM=https://your-cvm.dstack.phala.network

curl -X POST $CVM/_api/projects \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"scheduler-svc","source":"https://github.com/<you>/scheduler-svc","ref":"main"}'
```

Or from a local tarball, no public repo needed:

```bash
tar czf app.tgz -C apps/scheduler-svc .
curl -X POST $CVM/_api/projects \
  -H "Authorization: Bearer $TOKEN" \
  -F 'manifest={"name":"scheduler-svc","runtime":"dockerfile"};type=application/json' \
  -F "files=@app.tgz"
```

Reach the running app at `$CVM/scheduler-svc/`. Promote to attested with
`$CVM/_api/projects/scheduler-svc/promote` when you're ready — the source
hash binds into the dstack quote, the audit log opens, and the verifier
endpoints become public.

## What the app does

Three principals — Alice, Bob, Carol — each in their own fumola space.
Alice and Bob each publish their own per-day acceptance policy as fumola
thunks; Carol composes them and runs a short-circuiting search for the
first feasible day. Four canonical rounds:

| Endpoint           | What changes                              | Result   |
|--------------------|-------------------------------------------|----------|
| `GET /schedule/v1` | standard cap-only policies on both sides  | `?\`tue` |
| `GET /schedule/v2` | Bob amends: refuses Tuesdays              | `?\`thu` |
| `GET /schedule/v3` | Alice's week fills up                     | `null`   |
| `GET /schedule/v4` | Alice raises her own cap                  | `?\`thu` |

## Why the DCG digest is the point

Every fumola response carries `X-Fumola-DCG-Digest` — `sha256(project ||
method || path || body || output)`. Because the program uses only
deterministic prims (no clock, no RNG, no I/O), the digest is reproducible:

```
$ for v in v1 v2 v3 v4; do
    curl -s -D - http://localhost:8003/schedule/$v -o /dev/null \
      | grep -i 'x-fumola-dcg'
  done
v1: 5071e1be...
v2: f50578ff...
v3: cb1ec657...
v4: a516c82f...
```

Same request, same digest across runs. Different rounds, different digests.
On an attested deployment, the dstack TEE quote signs over the digest;
any client can replay the request locally, hash, and verify by equality.

## Where this comes from

The handler is a slim adaptation of a longer-form scheduler demo that
explores diagnostics ("who refused which day?"), time-versioned negotiation
history (every round queryable as a fumola time tag), and cross-version
diffs ("what was the smallest amendment that worked?"). This example
exposes only the round-by-round outcomes; the deeper Adapton features
stay one level inside the source for whoever wants to look.
