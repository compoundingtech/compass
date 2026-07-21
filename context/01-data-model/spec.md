# Spec: Data Model

Realizes [requirements.md](./requirements.md). Storage is specified in
[02-artifacts](../02-artifacts/spec.md) and is deliberately absent here.

## Plan

A Plan is a lineage of Versions plus the Progress recorded against them. It is
named by a minted `PlanRef`.

## Version

| Field | Meaning |
| --- | --- |
| `plan` | the Plan this version belongs to |
| `parent` | each predecessor; none for the first, several for a reconciliation |
| `author` | who authored the revision |
| `at` | logical time, ordered against other versions of this Plan |
| `why` | the Rationale — required |
| `goal` | the intent being pursued |
| `step` | zero or more Steps |
| `retired` | optional decommission flag |

`at` is a logical counter, not wall-clock time: machines disagree about clocks
and agree about causality, and ordering two sides of a divergence is a causal
question.

A version is identified by its content. Two versions with identical content are
the same version; any difference makes a different one. This is what makes
divergence observable rather than a lost write — see
[02-artifacts](../02-artifacts/spec.md) for how identity is computed.

## Step

A Step carries a minted `StepRef`, the intended work, `depends_on` edges, an
optional `supersedes` naming the Step it replaces, and an acceptance criterion.

Step identity is minted rather than derived from content, because a Step's
reference must survive rewording of the same intended work. Version identity is
derived from content, because a version's identity *should* change whenever its
bytes do. The two layers want opposite properties and therefore use different
mechanisms; this asymmetry is deliberate.

## Lineage, head, divergence

Head is the set of versions with no successor, computed by walking the lineage.

- **Divergence** — two or more versions share a predecessor. Both are valid.
- **Reconciliation** — an ordinary version naming several predecessors.
- **Orphan** — a version whose predecessor is unknown locally.

Divergence and orphan are superficially alike and must not be conflated: the
first is a disagreement about intent, the second is ordinarily incomplete
replication. Reconciling an orphan writes permanent intent to paper over a
transient gap.

A reconciliation can itself diverge, because nothing serializes authorship. Two
machines observing one divergence may each reconcile it differently, producing a
new divergence between the reconciliations. Compass reports this; it does not
resolve it. This is the cost of CMP-T01, and it is why convergence is a reported
condition rather than an assumed one.

## Progress

A Progress record names a Plan, a Step, the version it was observed against, an
actor, and a payload: start, update, handoff, completion, evidence.

Records are additive. A record against a superseded Step is attributed forward
through `supersedes`; a record against a retired Step is retained but does not
contribute to readiness.

## Acceptance

An acceptance criterion is a predicate over recorded evidence. It answers
whether a Step is done, from what has actually been observed, without asking a
judge.

The predicate vocabulary is unresolved (DQ03). Its shape is constrained by
CMP.DM-R15: a predicate that cannot report which of its parts failed makes
readiness unexplainable, so expressive power that costs explainability is not a
good trade here.

## Readiness

A Step is ready when it is not retired, its acceptance criterion is not yet
satisfied, and every Step it depends on has a satisfied criterion.

Every answer carries its reasons — which dependency or gate is unsatisfied.

Under divergence, readiness is computed per head member and labelled with it.
Merging the graphs would produce a plan nobody wrote; picking a side would hide
a disagreement the model exists to surface.
