# Roadmap: Compass

Non-normative. Directions outside the contract, recorded so their absence is
deliberate rather than overlooked. Nothing here binds the design.

## Contention between concurrent workers

Two agents taking the same Step is outside the contract. The plausible direction
is a serializing owner rather than claim leases: leases require a clock and a
reaper, and produce ambiguous states when a holder dies mid-work.

Divergence does not cover this. Divergence reconciles concurrent revision of
*intent*, and the availability tradeoff (CMP-T01) that makes it tolerable does
not obviously transfer — two agents doing the same work twice is waste, not a
disagreement with two valid sides.

## Automatic reconciliation

Divergence resolves by authorship, and a reconciliation can itself diverge,
since the model has no serialization point. Two directions are plausible.

A deterministic reconciliation function for the common cases — disjoint step
additions being the dominant one — would let two machines independently produce
byte-identical results, converging without coordination and preserving CMP-T01.
An elected serializer per Plan would also work, at the cost of that tradeoff.

The first is preferable, and depends on distinguishing genuine disagreement from
staleness: a machine that revised from an older version only because replication
lagged did not actually disagree, and that case is mechanically reconcilable.

## Compaction

Constrained by [decision 0003](./.decisions/0003-storage-is-a-catalog-replicated-by-an-external-union-sync.md)
Amendment 3: no-delete replication means removal is unavailable, so compaction
cannot work by deleting, and a tombstone leaves files in the scan path.

The plausible direction is a summary version subsuming a range of lineage,
paired with a derived index so discovery need not parse every file. Both
interact with CMP-A03 and with DQ06.

## Projection to external trackers

Rendering a Plan into an external issue tracker for human or cross-organization
visibility. Compass remains the authority and the projection is one-way.
Bidirectional sync is explicitly not a direction: it reintroduces the
dual-authority problem the design exists to avoid.

## Import

There is no path from existing trackers or checklists. A Plan's value here is
its rationale chain, and imported work has none, so an import yields a Plan
whose lineage begins with a single uninformative entry. Honest, but thin enough
that it is worth doing only if adoption depends on it.
