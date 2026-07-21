# Compass owns execution progress, not only intent

Status: accepted

## Context

A planning tool can stop at structural intent and leave execution state to
whatever runs the work. That keeps the tool small and the boundary crisp.

The alternative is for Compass to own both: immutable versioned intent, and an
append-only record of progress against it.

## Evidence and Argument

Splitting them fails on a specific question: readiness. "What can be worked on
now" is a function of the dependency graph *and* accepted progress. If progress
lives in another authority, readiness either cannot be computed by the tool that
owns the graph, or is computed by importing another tool's schema — which
decision 0001 forbids. The split also makes acceptance ambiguous: an external
observation would be able to complete a Step that the Plan's own acceptance
criteria have not judged complete.

The failure is observable wherever the two are kept apart. When session
progress, handoff notes, and evidence live in a work-log beside the plan, every
meaningful unit of work straddles two domains, and "all work has a plan" becomes
unenforceable — because work can exist entirely in the log, referenced by no
plan, and nothing detects it. The log then accumulates the work that matters
least to reconstruct and the plan loses the work that matters most.

The two layers remain distinct in regime — versions are immutable and
structural, events are append-only and operational — so owning both does not
blur them. It is one authority with two shapes, not one shape doing two jobs.

## Options

| Option | Tradeoffs |
| --- | --- |
| Compass owns intent and progress | Readiness and acceptance are computable in one authority; larger tool |
| Intent only; progress elsewhere | Smallest tool; readiness is not computable without importing a foreign schema, and acceptance can be asserted from outside |
| Compass owns progress, another tool renders it | Single authority with a familiar presentation; keeps a second surface alive indefinitely |

## Decision

Compass owns both layers. Progress Events are append-only records against a Step
— start, update, handoff, completion, evidence — that never alter structural
intent and never create a Plan Version.

Completion is judged by Plan-owned acceptance criteria. An external observation
alone never completes a Step.

Separate work-log surfaces are superseded rather than wrapped.

## Consequences

- Readiness is a pure fold over the Step graph at head plus accepted Progress
  Events, computable entirely within Compass (see roadmap).
- Correction of a Progress Event is a further event, never an edit.
- Progress replicates with the Catalog and is therefore visible across machines.
- Compass is larger than a pure intent store, and its event volume grows without
  bound until compaction exists.
