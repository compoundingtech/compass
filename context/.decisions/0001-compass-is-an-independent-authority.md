# Compass is an independent authority for planning intent

Status: accepted

## Context

Planning capability for coding agents tends to accrete inside whichever tool
already has a CLI — a coordination bus, an operations tool, an issue tracker.
Each of those owns something else first, so plan state ends up shaped by a
foreign storage layout and lifecycle. Extraction then requires migrating
identity and data, not just transport.

Compass is being defined before it holds any authoritative state, which is the
one moment when the boundary is free.

## Evidence and Argument

Consider the shape this takes in practice: plan state placed inside a
coordination bus's per-agent directory tree, with plan lifecycle derived by
folding that bus's event log. It works, and it makes the plan's identity,
storage path, and lifecycle semantics dependent on a tool that owns messaging,
not planning. Every later question — where plans live, how they replicate, what
happens when the bus changes shape — inherits that coupling, and none of them
can be answered on planning's own terms.

Defining opaque references, an idempotent port, and stable receipts before the
first authoritative write costs boundary work now and avoids an identity
migration later. Because no live plan state exists, that cost is at its minimum.

## Options

| Option | Tradeoffs |
| --- | --- |
| Independent authority from the first write | Boundary work up front; no later identity or data migration; the tool is portable by construction |
| Host plan state inside an existing tool, extract later | No boundary work now; requires migrating identity, storage, and lifecycle when extracted, at exactly the point where real data exists |
| Persist no local plan state; rely on an external tracker | Simplest; loses offline, structured, machine-local planning and makes every query a network call |

## Decision

Compass is a standalone authority. Its core owns goals, Steps, dependencies,
acceptance, revisions, and accepted progress. It depends on no other tool's
paths, schemas, event envelopes, or storage layouts.

Other systems compose with Compass through opaque references, mutations,
queries, and receipts. They may record operational facts referencing a
PlanReceipt, but such facts never become Compass state. Compass exposes no
subcommand inside another tool's CLI namespace; a facade would make the
namespace imply authority.

## Consequences

- The Catalog root is configuration, not a compiled-in path.
- Composition is one-directional: Compass never reads another tool to
  reconstruct its own state.
- Surrounding systems store opaque refs and receipts only.
- Moving Compass behind a different transport or process changes packaging, not
  identifiers or persisted semantics.
