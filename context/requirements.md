# Requirements: Compass

Realizes [vision.md](./vision.md). Terms are defined in
[ontology.md](./ontology.md); the authority model is in [spec.md](./spec.md).

This document holds the mechanism-agnostic contract. Each child node refines it
for one layer:

| Node | Owns |
| --- | --- |
| [01-data-model](./01-data-model/requirements.md) | what a Plan is, independent of how it is stored |
| [02-artifacts](./02-artifacts/requirements.md) | one realization of that model as files |
| [03-surface](./03-surface/requirements.md) | the logical query and mutation surface |
| [04-cli](./04-cli/requirements.md) | the operator surface over that port |
| [05-integrations](./05-integrations/requirements.md) | contracts Compass consumes rather than defines |

## Assumptions

- **CMP-A01 Replication is union-shaped.** The external sync mechanism provides
  union, newer-wins, and no-delete semantics. Compass correctness depends on
  this and Compass cannot impose it, so it must be verifiable rather than
  assumed.
- **CMP-A02 Concurrent writers are normal.** Several agents on several machines
  revise plans at once. Concurrency is the design centre, not an edge case.
- **CMP-A03 A catalog is walkable.** The plan set for one operator fits in
  memory and can be scanned in interactive time. When this stops holding, an
  index becomes necessary, and that is a change to this assumption rather than
  an optimization.

## Acceptable Tradeoffs

- **CMP-T01 Availability over consistency.** A local write never fails and
  converges later, rather than being serialized by a lock or a
  compare-and-swap. The price is real: divergence must be resolved by hand, and
  Compass cannot offer the "your write was rejected, reconcile first" signal
  that a consistent store gives for free.
- **CMP-T02 Corruption-evidence, not tamper-proofing.** Integrity mechanisms
  detect accidental and mechanical damage. They do not resist a determined
  writer, who owns the files and can recompute any derived value. Nothing here
  is a security boundary.
- **CMP-T03 Growth is unbounded until compaction exists.** Under no-delete
  replication, removal is unavailable as a reclamation strategy, so compaction
  must be designed rather than assumed.
- **CMP-T04 Plan history is not pull-request reviewable.** The catalog is not a
  repository. Review of intent, if wanted, is a projection.

## Requirements

- **CMP-R01 Sole authority.** Compass must be the sole authority for goals,
  steps, dependencies, acceptance, revisions, and accepted progress. No other
  system may determine what a Plan says or whether work is done.

- **CMP-R02 Intent is immutable.** Recorded intent must never change. Revision
  must produce new intent that supersedes the old, never an edit of it.

- **CMP-R03 Every revision states its reason.** A revision without a stated
  reason is invalid. The record of reasons is the durable output of the system;
  intent without it is a guess with no provenance.

- **CMP-R04 Divergence is legitimate.** Concurrent revision must produce a
  visible, reportable disagreement. Compass must never silently discard one
  side, and must never present a partial view as authoritative.

- **CMP-R05 Convergence must be observable.** A reader must be able to tell
  whether what it is reading is complete or still arriving. Serving incomplete
  state as authoritative, with no indication, is a defect rather than an
  acceptable consequence of asynchronous replication.

- **CMP-R06 Compass replicates nothing.** Replication must be delegated. Compass
  must remain fully functional on a single machine with no sync configured.

- **CMP-R07 Damage is detectable and recoverable.** Compass must detect
  corrupted or incomplete state, and must offer a recovery that preserves the
  surviving record of reasons. State that can become permanently unreadable and
  unrepairable defeats the purpose of keeping it.

- **CMP-R08 No foreign schema dependency.** The Compass core must not depend on
  another tool's paths, storage layouts, event envelopes, or private schemas.

- **CMP-R09 Composition is by reference.** Integrations must exchange opaque
  references, mutations, queries, and receipts. They must not share mutable
  files or mutate Compass state directly.

- **CMP-R10 Prefer derived values to asserted ones.** Where a value could be
  computed from existing state or supplied by a caller, it must be computed. A
  value a caller sets is a value a caller can set wrongly, and in an append-only
  store a wrong value is permanent. This constrains identity, ordering, and
  deduplication in particular: none of them may rest on something an actor
  chooses.
