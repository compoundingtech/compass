# Spec: Compass

Builds on [requirements.md](./requirements.md). Terms are defined in
[ontology.md](./ontology.md).

## Status

Draft. The logical contract is normative. Serialization details, CLI spelling,
and the idempotency model are open where noted.

## Shape

Compass owns two layers with different regimes:

```text
catalog/plans/<plan>/versions/<seq>-<hash>.kdl    immutable, mode 0444
catalog/plans/<plan>/events/<ts>-<id>.json        append-only
```

Structural intent lives in versions. Execution lives in events. A Progress Event
never creates a Plan Version; a Plan Version never records progress. Everything
a reader needs beyond these two layers is derived.

## Plan Versions

A Plan Version is an authored declarative document. It carries:

| Field | Meaning |
| --- | --- |
| `plan` | the `PlanRef` this version belongs to |
| `seq` | monotonic sequence within the Plan, for ordering and legibility |
| `parent` | content hash(es) of the predecessor version(s); empty for the first |
| `why` | required Rationale for this revision |
| `goal` | the intent being pursued |
| `acceptance` | how the Plan is judged complete |
| `step` | zero or more Step blocks |
| `retired` | optional decommission flag |

A Step block carries a minted `ref`, its intended work, `depends_on` edges,
acceptance criteria, and an optional `supersedes` naming the StepRef it
replaces.

The file name embeds the content hash of the version body. Files are written
mode `0444`.

### Identity rules

`StepRef` is **minted at Step creation and never derived from Step content.** A
title or metadata revision therefore preserves the StepRef, and a reader can
follow the same intended work across the whole version chain. Content hashes
address versions; they never address Steps. A retired StepRef is never reused.

`PlanRef` is likewise minted and opaque. Neither ref encodes a path, host, or
transport.

### The chain

Each version names its predecessor by content hash. This makes the history
tamper-evident: editing any historical version changes its hash and breaks every
descendant's `parent` link, which `compass verify` reports.

Mode `0444` is accident-prevention, not enforcement. On a filesystem the writer
owns, writes cannot be prevented; the chain is what makes violations *detectable*.
The two mechanisms are deliberately different in kind and neither is redundant.

### Head, forks, and merges

Head is **derived** by walking the chain, never stored. There is no head file, so
concurrent writers have nothing to contend on.

When two machines revise the same Plan concurrently, each writes a version with
the same `parent` and a different content hash. Under union replication both
files survive:

```text
versions/003-a1b2….kdl   parent = 002-…
versions/003-c3d4….kdl   parent = 002-…     ← a Fork, both present
versions/004-e5f6….kdl   parent = [a1b2…, c3d4…]   ← Merge Version
```

A Fork is a legitimate, visible state. `compass status` reports it; `compass
merge` resolves it by authoring an ordinary Plan Version with multiple parents
and a Rationale. Nothing is lost and nothing is silently overwritten — the
property the derived head buys.

## Progress Events

Progress Events are append-only files naming a `PlanRef`, a `StepRef`, the
version they were observed against, an actor, and a payload. They cover start,
update, handoff, completion, and evidence.

Events are additive only. Correction is a further event, never an edit.

## Catalog and discovery

Discovery is content-based: the Catalog tree is walked and every file that *is*
a Plan Version or Progress Event is processed, whatever its path. Path segments
may supply defaults; content wins on mismatch, and a mismatch is a warning
rather than an error.

Paths inside authored content use environment-variable references rather than
absolute paths, so a Catalog is machine-agnostic and replicates cleanly to hosts
with different layouts.

Decommissioning is always the `retired` flag. Files are never deleted, because
the Catalog replicates as a union with no deletes — a deletion would simply
return on the next sync.

## Replication

Compass does not replicate anything itself. The Catalog is a directory declared
to an external file-sync mechanism with union / newer-wins / no-delete
semantics.

This works because both layers are append-only and content-addressed: union is
the correct merge, "newer wins" never has to arbitrate a mutable cell, and a
genuine concurrent revision surfaces as a Fork rather than a lost write.

Compass depends on no other tool's paths, schemas, or storage layouts. The
Catalog root is configuration.

## PlanPort

The logical surface is transport-neutral:

```text
getPlan(PlanRef)   -> PlanView | NotFound
mutate(mutation)   -> PlanReceipt | PlanError
```

`PlanMutation` covers creation, structural revision, acceptance changes, Step
progress, retirement, and merge. A successful mutation returns a stable
`PlanReceipt` naming the affected refs and the resulting version hash.

Whether mutation dedup is keyed by a caller-supplied idempotency key, by content
address, or by both at different layers is **open** — see
[open-questions.md](./open-questions.md).

## Composition

A surrounding system may append an Observation referencing a PlanReceipt, but
only after the mutation succeeds. Observation failure never rolls back, blocks,
or alters the Compass result. A failed mutation produces no receipt and no
success Observation.

Integrations exchange opaque refs, mutations, queries, and receipts. They do not
share mutable files and do not mutate Compass state directly.
