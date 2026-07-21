# Roadmap: Compass

Non-normative. Direction, not commitment.

Readiness was previously listed here as deferred. It is not: it ships in v1 per
[decision 0006](./.decisions/0006-readiness-ships-in-v1-and-acceptance-is-machine-checkable.md),
because deferring it would leave the progress layer with no consumer.

## Contention between concurrent workers

Two agents taking the same Step is unhandled. The intended direction is a
serializing owner rather than claim leases with timeouts: leases require a clock
and a reaper, and produce ambiguous states when a holder dies mid-work.

Note what this does *not* cover. Divergence handles concurrent *revision* of
intent. Concurrent *execution* of the same Step is a separate problem, and the
availability tradeoff (CMP-T01) that makes divergence tolerable does not
obviously transfer — two agents doing the same work twice is waste, not a
disagreement to reconcile.

## Automatic reconciliation

Divergence is resolved by hand today, and reconciliations can themselves diverge
because there is no serialization point. Two directions are plausible: a
deterministic reconciliation function for the common cases (disjoint step
additions, which is the dominant case) so that two machines independently
produce byte-identical results and converge without coordination; or an elected
serializer per plan. The first preserves the availability tradeoff and is
preferred.

Distinguishing genuine disagreement from mere staleness matters here — a machine
that revised from an older version only because replication lagged did not
actually disagree, and that case should reconcile automatically.

## Compaction

Constrained by [decision 0003](./.decisions/0003-storage-is-a-catalog-replicated-by-an-external-union-sync.md)
Amendment 3: under no-delete replication, removal is unavailable, so compaction
cannot work by deleting. A tombstone readers honour still leaves files on disk
and in the scan path, which is the actual cost.

The plausible direction is a periodically rewritten summary version that
subsumes a range of history, plus an index so that discovery does not parse
every file. Both interact with CMP-A03 — when the catalog stops being walkable
in interactive time, an index is required, and that is a change of assumption
rather than an optimization.

## Projection to external trackers

Rendering a Plan into an external issue tracker for human or cross-organization
visibility. Compass would remain the authority and the projection would be
one-way; bidirectional sync is explicitly not the direction, since it
reintroduces the dual-authority problem the design exists to avoid.

## Import

There is no migration path from existing trackers or markdown checklists. A
plan's value here is its rationale chain, and imported work has none — so an
import produces a plan whose history begins with "imported," which is honest but
thin. Worth doing anyway if adoption ever depends on it.
