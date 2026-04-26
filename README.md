# scheduler-svc — fumola in your browser

Live demo of a multi-party 1:1 meeting scheduler, written in fumola
([Adapton](https://github.com/Adapton/fumola)) and running entirely
client-side via WebAssembly. No server.

**Open `index.html`** (or visit the GitHub Pages URL).

The page lazy-loads `wasm-pkg/fumola_wasm.js` (~5 MB), registers
`handle.fumola` as a fumola module, and evaluates `H.handle("GET",
"/schedule/v1", "")` directly in the browser for each round you click.
The DCG receipt — the structured record of which days the scheduler
inspected and which party's policy refused which — renders alongside
the verdict.

## Files at the root of this branch

| File                               | What it is                                              |
|------------------------------------|---------------------------------------------------------|
| `index.html`                       | The page                                                |
| `handle.fumola`                    | The scheduler source (loaded as a fumola module)        |
| `wasm-pkg/fumola_wasm.js`          | wasm-bindgen JS glue                                    |
| `wasm-pkg/fumola_wasm_bg.wasm`     | The fumola interpreter compiled to WebAssembly          |

## Source

Everything else — the Rust crate that compiles fumola to wasm, the
fumola handler, the python aiohttp server for tee-daemon deployment,
and the design docs — lives on the `examples/web-scheduler` branch:

  https://github.com/amiller/fumola/tree/examples/web-scheduler

The Pages build is a static export of the same example, with the
server-mode toggle auto-disabled because no fumola server is reachable
when hosted statically.

## What the demo shows

Three principals — Alice, Bob, Carol — each in their own fumola space.
Alice and Bob each publish their own per-day acceptance policy as
fumola thunks; Carol composes them and runs a short-circuiting search
for the first feasible day. Four rounds:

| Round | What changes                              | Result   |
|-------|-------------------------------------------|----------|
| v1    | standard cap-only policies on both sides  | `?\`tue` |
| v2    | Bob amends: refuses Tuesdays              | `?\`thu` |
| v3    | Alice's week fills up                     | `null`   |
| v4    | Alice raises her own cap                  | `?\`thu` |

The DCG receipt for each round names exactly which days the scheduler
inspected and what each `fits_<day>` thunk returned. That receipt is
the artifact a TEE-attested deployment would sign over.

For the full design discussion (handler contract, dockerfile-runtime
deploy onto a Phala dstack CVM via tee-daemon, privacy axes), see the
`examples/web-scheduler/` directory on the development branch.
