# Spec: Data Model

Realizes [requirements.md](./requirements.md). Storage is specified in
[02-artifacts](../02-artifacts/spec.md) and is deliberately absent here.

## Plan

A Plan is a lineage of Versions plus the Progress recorded against them. It is
named by a `PlanRef`; what determines that reference is unresolved (DQ04).

## Version

A Version is a program that states one Plan's structural intent. Reading it
means running it, so the fields below are what a version *declares* rather than
fields of a record:

| Declared | Meaning |
| --- | --- |
| plan | the Plan this version belongs to |
| predecessors | each version it revises — none for the first, one ordinarily, several for a reconciliation |
| author | who authored the revision |
| rationale | why intent changed — required |
| goal | the intent being pursued |
| steps | zero or more named Step declarations |
| retired | whether the Plan itself is decommissioned |

A predecessor is not a name copied into the version; it is the predecessor
itself, referenced. That is what lets a revision be written as a function of
what came before, and it is why a retry cannot drift: the base a revision was
written against is part of the revision, not something read when it is applied.

There is no ordering field. A counter cannot order divergent siblings — neither
observed the other, so both would carry the same value — and where one version
precedes another, the lineage already records it. Order is derived; only
authorship is stated.

A version is identified by its content. Two versions with identical content are
the same version; any difference makes a different one. This is what makes
divergence observable rather than a lost write — see
[02-artifacts](../02-artifacts/spec.md) for how identity is computed.

The author is authored, not observed. Compass stores a version exactly as
written and therefore cannot stamp it, so authorship of a version has the
standing of a claim, unlike the actor of a Progress record, which Compass writes
itself. Whether that gap should be closed is unresolved (DQ07).

## Revision

A revision is written against its predecessor and may:

- **edit** a Step — change its work, dependencies, or acceptance,
- **add** a Step — a new declaration, whose name becomes its identity,
- **retire** a Step — mark it decommissioned while it stays in the Plan.

There is no fourth operation. Every Step of the predecessor is carried forward
unless it is edited or retired, so a revision has no way to say "and this one is
gone." Losing a Step silently is not caught by a check; it has no spelling.

Retirement is therefore visible in the lineage: a retired Step appears in every
later version, marked, rather than ceasing to appear.

A reconciliation is a revision with more than one predecessor and behaves the
same way: every Step of every predecessor is carried forward, and the version
states only what it resolves. This is what makes reconciliation derivable
against each side rather than a choice of one side whose rejected half vanishes
unrecorded. Where two predecessors disagree about the same Step, the
reconciliation must say which intent survives; how that is stated, and what
happens if it does not, is unresolved (DQ08).

## Step

A Step is a named declaration carrying the intended work, its dependencies, an
optional `supersedes` naming the Step it replaces, and an acceptance criterion.

Its identity is that name, qualified by its Plan. A dependency is a reference to
the declaration, not a reference written out by hand, so it resolves or it fails
where it is written — there is no dependency on a Step that does not exist and
no dependency on a Step the author meant to name but did not.

Identity therefore survives rewording, because rewording the work does not touch
the name it is declared under. It does not survive renaming, and it should not:
a rename in already-committed content is not an edit but an alteration of what
was committed, and it is detectable as one.

Version identity is derived from content, because a version's identity *should*
change whenever its content does. Step identity is declared, because a Step's
identity must *not* change when its wording does. The two layers want opposite
properties and therefore use different mechanisms; this asymmetry is deliberate.

## Lineage, head, divergence

Head is the set of versions with no successor, computed by walking the lineage.

- **Divergence** — two or more versions share a predecessor. Both are valid.
- **Reconciliation** — an ordinary version naming several predecessors.
- **Orphan** — a version whose predecessor is unknown locally.
- **Unresolved** — a Plan that cannot be evaluated, because something it
  references is not present locally.

Divergence and orphan are superficially alike and must not be conflated: the
first is a disagreement about intent, the second is ordinarily incomplete
replication. Reconciling an orphan writes permanent intent to paper over a
transient gap.

Unresolved is a third state and the most severe of the three. An orphan still
says what it intends; only its lineage is short. An unresolved Plan says
nothing: no goal, no Steps, no readiness, because the answer to every question
about it is produced by evaluating it and the evaluation cannot complete. Like
an orphan it is ordinarily repaired by waiting, and unlike an orphan there is
nothing partial to report in the meantime. It is permanent if what it references
was never committed anywhere.

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

The words inside a criterion — the kinds of evidence and their attributes — are
supplied by whoever writes the Plan and are defined nowhere. Compass fixes the
structure and stays out of the vocabulary, which is why a Plan for writing or
research is expressible on exactly the same terms as one for software.

The cost of an undeclared vocabulary is a criterion and a record that name the
same thing differently, leaving the criterion valid and permanently
unsatisfiable. That is caught where it is made rather than by a declaration:
when a record is written, the criteria it could contribute to are already in the
version being written against, so the record is checked against them, and the
domain of each attribute is derived from the values those criteria bind — the
Plan supplies its own enumeration without declaring one.

The check reports and never refuses, and its scope is deliberately narrow:

- Only positively-stated criteria may suggest a correction. Negation is
  idiomatic — "no known failure" — so a record matching the inner term of a
  negated criterion is an outcome to report, not a mistake to fix.
- A near-match is not evidence of a mistake. A genuine alternative outcome
  differs from the expected one by exactly as much as a misspelling does.
- A value no criterion mentions is accepted in silence. Matching is a subset
  relation, and evidence may carry more context than any criterion demands.

The residue is a legitimate value that no criterion happens to name, which is
irreducible and is the reason this reports rather than refuses.

A criterion that contradicts itself is refused when it is written. Whether a
criterion will *ever* be satisfied is a different and undecidable question,
since satisfaction may depend on whether a named actor ever acts.

The predicate vocabulary itself is unresolved (DQ03). Its shape is constrained
by CMP.DM-R15: a predicate that cannot report which of its parts failed makes
readiness unexplainable, so expressive power that costs explainability is not a
good trade here.

A narrowing revision can leave previously-recorded evidence inert under every
later criterion. Nothing prevents that: evidence is evaluated against the
criterion at the current frontier, and the version a record cites is provenance
rather than the scope it is judged in.

## Readiness

A Step is ready when it is not retired, its acceptance criterion is not yet
satisfied, and every Step it depends on has a satisfied criterion.

Every answer carries its reasons — which dependency or gate is unsatisfied.

Under divergence, readiness is computed per head member and labelled with it.
Merging the graphs would produce a plan nobody wrote; picking a side would hide
a disagreement the model exists to surface.
