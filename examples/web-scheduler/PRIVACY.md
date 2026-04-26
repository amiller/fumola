# Reasoning about private preferences with Adapton

A short exploration: where does Adapton's specific machinery — named cells,
DCG receipts, demand-driven evaluation — actually help when parties want to
keep their preferences private?

## The minimum demonstration in this example

The handler exposes two parallel endpoint families:

| Endpoint        | Body                                       | Size (round v1) |
|-----------------|--------------------------------------------|-----------------|
| `/schedule/v1`  | `{ version, chosen, dcg }` — full receipt  | 913 bytes       |
| `/private/v1`   | `{ version, chosen }` — verdict only       | 68 bytes        |

Both invoke the same `runRound` internally: the TEE computes the full DCG,
then the private endpoint discards everything but the verdict before
returning. **The TEE saw alice's and bob's accept-thunks; the external
caller sees only the chosen day.** That's the simplest privacy primitive
the language already supports — *selective publication* of a structured
result whose components are typed.

Verification still works because each party can re-derive the same DCG in
their own browser (the wasm path) using their own copy of the policies.
Match the chosen day → match the underlying reasoning.

## Why the DCG matters here

If the receipt were just a string ("Tuesday wins"), there'd be no privacy
work to do — but also no audit. Adapton lets you have it both ways:

- The DCG is a **value**, not a side-channel log. Inside the TEE you can
  walk it, classify it, redact it, hash it, sign over portions, decide
  what to publish.
- Each edge names the **cell it read** and the **value it observed**.
  That two-part structure is exactly the privacy unit: which input was
  consulted, what conclusion did it produce.
- **Demand-driven** evaluation means the DCG already has minimum-
  disclosure baked into the search itself. `first_feasible` short-circuits
  on the first true day; if Tuesday wins, Wednesday onward never appears
  in the receipt at all. Privacy isn't a separate pass — it's what the
  language already does.

## Four axes worth thinking about

These are research-shaped, not implemented. Listing them makes the
language's relationship to each clear.

### 1. Selective disclosure (today's `/private/*` endpoint)

The TEE owns the DCG. The published response contains only the verdict.
A digest over the full DCG can serve as a commitment: a verifier with
the same inputs can re-derive locally and confirm by hash equality.

In our current code the `X-Fumola-DCG-Digest` header hashes the
*response body* (which differs between full and private), not a
canonical DCG. A real implementation would compute one canonical hash
over the full DCG inside the TEE and expose it on both endpoints, so
the digest is the same regardless of disclosure level. ~30 lines of
work.

### 2. Per-cell sensitivity labels

The party publishing a cell labels it `#public | #private | #sensitive`.
A redaction pass walks the DCG: edges whose target sits in a `#private`
cell get their `value` field replaced with `#redacted`. The receipt
still shows the *shape* of the search — which cells were consulted, in
what order — but not the boolean each one returned.

This is the classic "info flow" pattern. Adapton makes it cheap because
the DCG is already a graph indexed by cell; the redaction pass is a
graph traversal, not a custom annotation system bolted on.

### 3. Cross-party data, party-local TEEs

In our deployment, alice's policy lives in the same fumola eval as
carol's. In a federated setting, alice's policy would live in alice's
own TEE, and carol's TEE would invoke it via attested API. Each
`force(a.accepts_<day>)` becomes an attested round-trip; the value
returned travels in the clear, but alice's underlying cells never
leave her TEE.

The DCG's role: the receipt records "a.accepts_mon was forced and
returned false," NOT "a.mon = 7". The minimum thing alice has to
disclose to negotiate is per-day booleans, not hours.

### 4. Counterfactual queries as privacy oracles

"Would v3 have succeeded if alice's mon=7 instead of 8?" — runRound
with a perturbed input. The diff demo (`runDiffDemo` in `demo/`) is
the public-private version of this. A privacy-aware variant: alice's
TEE answers the counterfactual without revealing what mon currently
is; the response is just `{counterfactual_chosen, original_chosen}`,
two booleans-or-symbols that don't leak the values.

The Adapton angle: counterfactual computation reuses cached sub-DCGs
where the perturbation didn't reach. So the marginal cost of asking
"what if?" is bounded by the perturbation's reach in the DCG, not the
size of the policy. Privacy-cheap.

## What this maps onto in oauth3 / capability-style negotiation

Each axis above has a familiar shape in the auth-protocol world:

- **Selective disclosure** = the access token's claims contain the
  decision but not the policy that produced it.
- **Per-cell sensitivity** = the AS's verifier API returns the verdict
  plus a list of *which* policy clauses fired, not their values.
- **Cross-party + per-party TEEs** = federated trust: each principal
  hosts their own policy endpoint, the AS composes via attested calls.
- **Counterfactual** = "what permission would be granted under THIS
  scope?" without committing to it.

None of these require new fumola features. The first one is in this
repo today (the `/private/*` endpoints); the others are 30-100 line
extensions of `handle.fumola`.

## What's NOT a fit (worth saying)

- **Zero-knowledge proofs over the DCG.** Adapton's data-dependent
  control flow doesn't compile cleanly into arithmetic circuits. If
  the goal is "prove the verdict without revealing the DCG to a
  non-TEE verifier," zkVMs (RISC Zero, SP1) are the right substrate;
  Adapton organizes the computation, but the verifier toolchain is
  separate.
- **Multi-party computation.** Same reason. Adapton helps if you're
  willing to run inside a TEE and use attested cross-party calls; it
  doesn't replace MPC where you don't trust any single TEE.
- **Differential privacy over numeric outputs.** The DCG records what
  was read, but DP needs noise added to outputs in a calibrated way;
  Adapton is the substrate you build over, not the noise mechanism.

The right summary: **Adapton is a privacy-aware substrate, not a
privacy mechanism.** It gives you the structure to reason about what
got disclosed. The actual disclosure rules and the cryptographic
primitives that implement them sit on top.
