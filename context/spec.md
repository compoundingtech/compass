# Spec: Compass

Realizes [requirements.md](./requirements.md). Terms are defined in
[ontology.md](./ontology.md). The boundary with external systems is specified in
[01-integrations/](./01-integrations/spec.md).

## Status

Draft. The logical contract is normative. Serialization syntax, CLI spelling,
and the idempotency model are open where noted.

## Shape

Compass owns two layers with different regimes:

```text
catalog/plans/<plan>/versions/<seq>-<hash>.kdl    immutable, mode 0444
catalog/plans/<plan>/events/<ts>-<id>.json        append-only
```

Structural intent lives in versions. Execution lives in events. A Progress Event
never creates a Plan Version; a Plan Version never records progress. Everything
else — Head, Readiness, lineage — is derived.

## Plan Versions

| Field | Meaning |
| --- | --- |
| `plan` | the `PlanRef` this version belongs to |
| `seq` | position along this version's own lineage; ordering aid, not identity |
| `parent` | content hash of each predecessor; none for the first, several for a Reconciliation |
| `author` | who authored the revision |
| `at` | logical time, ordered against other versions of the same Plan |
| `why` | required Rationale for this revision |
| `goal` | the intent being pursued |
| `step` | zero or more Step blocks |
| `retired` | optional decommission flag |

`author` and `at` exist because reconciling a Divergence begins with knowing who
wrote each side and in what order. Without them Compass would offer less than
the version-control history it declines to use.

`seq` is an aid to reading, not a key. Divergent versions may share a `seq`, and
the value after a Reconciliation of unequal-length lineages follows the longest
predecessor. Nothing resolves on `seq`.

### Steps and acceptance

A Step carries a minted `ref`, its intended work, `depends_on` edges, an
optional `supersedes` naming the StepRef it replaces, and `accept`.

`accept` is machine-checkable: a predicate over recorded evidence, not prose.
Prose may accompany it, but Readiness must be computable without a judge, and
prose cannot be folded. The predicate vocabulary is unspecified here and must
land before v1.

### Identity

`StepRef` is minted at Step creation and never derived from Step content, so a
rewording preserves it and a reader can follow the same intended work across the
whole lineage. Content hashes address versions; they never address Steps. A
retired StepRef is never reused.

Refs are minted so that two machines minting concurrently cannot collide. The
minting scheme is unspecified here; independence from a coordinator is the
requirement.

`PlanRef` follows the same rules.

## The chain

Each version names its predecessors by content hash, making corruption
detectable: altering a version changes its hash and breaks every descendant's
link.

A file whose content does not match its content-addressed name is **rejected**,
not warned about. Under no-delete replication a wrongly-admitted file is
permanent, so ingestion is deliberate: a file becomes a version only in the
expected location with a name matching its content. Merely parsing is not
enough.

Mode `0444` is accident-prevention — it converts an agent's in-place edit into a
visible permission error. It is not enforcement: on a filesystem the writer
owns, writes cannot be prevented, and a determined writer can recompute every
hash. The chain gives corruption-evidence, not tamper-proofing.

### Head, Divergence, Orphans

Head is the set of versions with no successor, derived by walking the chain.
Ordinarily it has one member.

When two machines revise concurrently, each writes a version with the same
`parent` and a different content hash. Under union replication both survive:

```text
versions/003-a1b2….kdl   parent = 002-…
versions/003-c3d4….kdl   parent = 002-…     ← Divergence: same predecessor
versions/004-e5f6….kdl   parent = [a1b2…, c3d4…]   ← Reconciliation
```

Divergence is legitimate and visible. Reconciliation is an ordinary version
naming both predecessors with its own Rationale.

**A Reconciliation can itself diverge.** Two machines observing the same
Divergence may each author a reconciliation with the same predecessors and
different bytes. Nothing in the model prevents this, because there is no
serialization point anywhere — the cost of CMP-T01. Compass detects the case
and reports it as what it is; it does not resolve automatically.

An **Orphan** — a version whose predecessor is absent locally — is not
Divergence, though it resembles one. It ordinarily means replication is
incomplete. Compass distinguishes the two and never offers reconciliation as the
repair for an orphan, because reconciling around a version that is merely
in-flight writes permanent intent to fix a transient condition.

### Convergence gates authoritative reads

Because completeness cannot be read from the catalog, any query reporting
authoritative state first establishes convergence from the replication
substrate. When the catalog is still receiving, Compass says so rather than
serving a Head that a pending file would supersede. An unknown convergence state
is reported as unknown, never assumed converged. See
[01-integrations/](./01-integrations/spec.md).

### Repair

Detection without repair is insufficient, and under no-delete replication every
write is permanent — including mistakes. Deleting a damaged version returns it
on the next sync; rewriting it cascades through every descendant.

Repair therefore never edits history. A damaged or unverifiable version is
marked as such by a new authored version that records the damage, states what
is known of the lost intent, and continues the lineage from the last verifiable
predecessor. The surviving Rationale record is preserved, which is the property
that matters; the damaged bytes remain on disk and are excluded from
interpretation.

This is also the only available response to content that should never have been
written — a credential pasted into a step description cannot be recalled from
replicas. Compass makes such content inert and flags it; it cannot make it
absent. Treat the catalog as append-only in the strongest sense: do not write
what must later be unwritten.

## Progress Events

Progress Events are append-only files naming a `PlanRef`, a `StepRef`, the
version observed against, an `actor`, and a payload: start, update, handoff,
completion, evidence. Correction is a further event, never an edit.

Events reference the version they were observed against, so an event recorded
against a Step later superseded is attributed to the superseding Step through
the `supersedes` edge, and an event against a retired Step is retained but does
not contribute to Readiness.

## Readiness

Readiness is derived from the Step graph at Head, accepted progress, and gates.
A Step is ready when its dependencies are satisfied, its gates are open, and it
is neither accepted nor retired. Acceptance is evaluated from `accept` against
recorded evidence.

Readiness always explains itself — which dependency or gate is unsatisfied. An
unexplained answer cannot be trusted or debugged.

**Under Divergence, Readiness is reported per Head member, labelled.** It does
not silently pick a side, and it does not merge the graphs, which would invent
intent nobody authored. Divergence is a normal state; the primary query has a
defined meaning there, and that meaning is "here are the two answers, and why
they differ."

## Catalog and discovery

Discovery walks the catalog and admits files in their expected locations whose
content matches their names. Path segments may supply defaults; content wins on
mismatch.

Paths inside authored content use environment-variable references rather than
absolute paths, so a catalog is machine-agnostic. A reference that does not
resolve is an error — resolving it to an empty or plausible-but-wrong path is
worse than failing.

Decommissioning is the `retired` flag. Files are never deleted, because a
deletion returns on the next sync.

## PlanPort

```text
getPlan(PlanRef)   -> PlanView | NotFound
getReady(PlanRef)  -> Readiness
mutate(mutation)   -> PlanReceipt | PlanError
```

`PlanMutation` covers creation, structural revision, acceptance changes, Step
progress, retirement, and reconciliation. A successful mutation returns a stable
`PlanReceipt` naming the affected refs and the resulting version hash.

Whether mutation dedup is keyed by a caller-supplied idempotency key, by content
address, or by both at different layers is **open** — see
[open-questions.md](./open-questions.md).

## CLI

The CLI is the operator surface, addressed to an agent or a human reading a
terminal. Its spelling is open; its contract is not.

Every command reports convergence state alongside its answer, so a reader can
never mistake a pre-convergence answer for an authoritative one. Every command
that reports Head handles a Head set of more than one without erroring, and
labels the members. Machine-readable output is available for every command that
has a human rendering, and carries the same fields.

The surface covers: creating a plan, revising it, recording progress, querying
readiness, showing lineage and rationale, reconciling a divergence, and
verifying the chain. Verification and repair are distinct commands, because one
is safe to run anywhere and the other authors permanent content.

## Composition

An external system may append an Observation referencing a PlanReceipt, but only
after the mutation succeeds. Observation failure never rolls back, blocks, or
alters the result. A failed mutation produces no receipt and no success
observation.

Integrations exchange opaque refs, mutations, queries, and receipts. They do not
share mutable files and do not mutate Compass state directly.

## Worked example

A plan created, revised, diverged, and reconciled. This is the whole product in
one page; the shape is illustrative and the syntax is not yet fixed (DQ02).

**1 — created.** `001-4f2a…kdl`

```kdl
plan "pl_7Kq2" seq=1 author="cos" at=1 {
  why "Initial plan. Parser rejects nested groups; we need it fixed before the
       release branch cuts."
  goal "Nested groups parse correctly"
  step "st_a1" { work "Reproduce with a failing test"
                 accept { evidence "test" name="parser::nested_groups" status="fail" } }
  step "st_b2" { work "Fix the grammar"  depends_on "st_a1"
                 accept { evidence "test" name="parser::nested_groups" status="pass" } }
}
```

**2 — revised.** `002-9c81…kdl`, `parent = 4f2a…`

```kdl
why "Reproduction showed the tokenizer, not the grammar, drops the closing
     delimiter. Retargeting the fix; the grammar is not at fault."
```

`st_b2` keeps its ref and changes its `work` to name the tokenizer. Identity
survives because the intended work — make nested groups parse — did not change.

**3 — diverged.** Two machines revise from `002` before replication catches up.

```text
003-a1b2…  author="cos"   at=3  why "Adding a fuzz step; one case is not enough."
003-c3d4…  author="dev"   at=3  why "Splitting the fix: tokenizer, then grammar
                                     guard, so each lands reviewable."
```

Both survive. `compass status` reports two Head members with their authors and
rationales — and this is the moment the design earns itself: nobody has to
reconstruct why the plan disagreed, because both sides said so at the time.

**4 — reconciled.** `004-e5f6…kdl`, `parent = [a1b2…, c3d4…]`

```kdl
why "Both were right. Taking the two-step split from dev, and keeping the fuzz
     step from cos as a third, gated on the split landing."
```

The chain now reads as an argument: what we thought, what we learned, where we
disagreed, and how it settled. The plan at the tip is disposable. The four
rationales are not.
