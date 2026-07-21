# Roadmap: Compass

Non-normative. Direction, not commitment.

## Ready work

The query "what can be worked on now" is a pure fold over material Compass
already owns: the Step graph at head, plus accepted Progress Events, plus gates.
It needs no schema change, which is why v1 ships the substrate without it rather
than designing around its absence.

Deferring it is a deliberate scope call, not an oversight. It is also the single
feature most responsible for the appeal of graph-based trackers, so shipping the
substrate without it means v1 supersedes their *data model* before it matches
their *ergonomics*.

## Contention between concurrent workers

Two agents taking the same Step is unhandled in v1. The intended direction is a
serializing owner rather than claim leases with timeouts: leases require a clock
and a reaper, and produce ambiguous states when a holder dies mid-work.

Deferred rather than dropped. The Fork mechanism handles concurrent *revision*;
concurrent *execution* is a separate problem and v1 does not pretend to solve it.

## Compaction

See DQ04. Some form of history compaction that preserves the Rationale chain
while shedding event volume is likely required for long-lived Plans.

## Projection to external trackers

Rendering a Plan into an external issue tracker for human or cross-organization
visibility. Compass would remain the authority and the projection would be
one-way; bidirectional sync is explicitly not the direction, since it
reintroduces the dual-authority problem the design exists to avoid.
