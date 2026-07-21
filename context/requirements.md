# Requirements: Compass

Realizes [vision.md](./vision.md). Terms are defined in
[ontology.md](./ontology.md); mechanism in [spec.md](./spec.md).

## Assumptions

- **CMP-A01 Replication is union-shaped.** The external sync mechanism provides
  union, newer-wins, and no-delete semantics. Compass correctness depends on
  this and Compass cannot impose it, so it must be verifiable rather than
  assumed (see CMP-R17).
- **CMP-A02 Concurrent writers are normal.** Several agents on several machines
  revise plans at once. Concurrency is the design centre, not an edge case.
- **CMP-A03 A catalog is walkable.** The plan set for one operator fits in
  memory and can be scanned in interactive time. When this stops holding, an
  index becomes necessary and is a change to this assumption, not a workaround.

## Acceptable Tradeoffs

- **CMP-T01 Availability over consistency.** A local write never fails and
  converges later, rather than being serialized by a lock or a
  compare-and-swap. The price is real: divergence must be resolved by hand, and
  Compass cannot offer the "your write was rejected, rebase" signal that a
  consistent store gives for free.
- **CMP-T02 Corruption-evidence, not tamper-proofing.** The chain detects
  accidental and mechanical corruption. It does not resist a determined writer,
  who owns the files and can recompute every hash. Nothing here is a security
  boundary.
- **CMP-T03 Growth is unbounded until compaction exists.** Under no-delete
  replication, removal is not available as a reclamation strategy, so
  compaction must be designed rather than assumed.
- **CMP-T04 Plan history is not pull-request reviewable.** The catalog is not a
  repository. Review of intent, if wanted, is a projection.

## Requirements

### Intent and identity

- **CMP-R01 Sole authority.** Compass must be the sole authority for goals,
  steps, dependencies, acceptance, revisions, and accepted progress.
- **CMP-R02 Versions are immutable.** A Plan Version, once written, must never
  change. Revision must produce a new version rather than edit an existing one.
- **CMP-R03 Rationale is required.** Every Plan Version must carry a stated
  reason for the revision. A version without one is invalid.
- **CMP-R04 Head is derived.** The current version must be computed from the
  chain rather than stored, so that no mutable cell exists for concurrent
  writers to contend on.
- **CMP-R05 Divergence is legitimate.** Two versions sharing a predecessor must
  both survive replication and both be reported. Divergence must never be
  silently resolved by discarding one side.
- **CMP-R06 Divergence resolves by authorship.** Reconciliation must be an
  ordinary Plan Version naming every predecessor it reconciles, with its own
  Rationale.
- **CMP-R07 References are minted.** PlanRef and StepRef must be minted, opaque,
  and independent of content, so that revising text preserves identity. They
  must encode no path, host, or transport, and must not collide when minted
  concurrently on different machines.
- **CMP-R08 Retired references are final.** A reference must never be reused
  after retirement.
- **CMP-R09 Versions are attributable.** Every Plan Version must record its
  author and a logical time. Reconciling divergence requires knowing who wrote
  each side and in what order; without this, Compass offers less than the
  version-control history it declines to use.

### Progress, acceptance, and readiness

- **CMP-R10 Progress is append-only.** Progress Events must never alter
  structural intent and never create a Plan Version. Correction must be a
  further event, never an edit.
- **CMP-R11 Acceptance is machine-checkable.** A Step's acceptance criteria must
  be expressible in a form Compass can evaluate against recorded evidence. Prose
  alone is insufficient, because readiness cannot fold over prose.
- **CMP-R12 Readiness is derived and shipped.** Compass must compute what work is
  available from the step graph at head, accepted progress, and gates. Readiness
  is not deferrable: it is the query the model exists to answer, and without it
  the progress layer has no consumer.
- **CMP-R13 Readiness is defined under divergence.** When a plan has diverged,
  readiness must produce a defined, explained result rather than an arbitrary
  one. The normal state of the system must not be an undefined state of its
  primary query.
- **CMP-R14 Acceptance is Compass-judged.** An external observation must never
  complete a Step.

### Catalog and convergence

- **CMP-R15 The catalog root is configuration.** Compass must not compile in a
  storage location.
- **CMP-R16 Compass replicates nothing.** Replication must be delegated to an
  external mechanism. Compass must remain fully functional on a single machine
  with no sync configured.
- **CMP-R17 Convergence must be observable.** A reader must be able to tell
  whether the catalog it is reading is converged or still receiving. Serving a
  stale head as authoritative, with no indication, is a defect — not an
  acceptable consequence of asynchronous replication.
- **CMP-R18 The sync contract must be checkable.** Compass must be able to
  detect that its replication assumption (CMP-A01) is violated. A sync
  configured with delete propagation must produce a diagnosable failure rather
  than silent history loss.
- **CMP-R19 Retirement is content.** Decommissioning must be an authored flag.
  Deletion must never be the mechanism, because a deleted file returns on the
  next sync.

### Integrity

- **CMP-R20 The chain is verifiable.** Compass must detect a broken or
  inconsistent chain, including a file whose content does not match its
  content-addressed name.
- **CMP-R21 Corruption has a repair path.** Detecting damage is insufficient. A
  damaged chain must have a defined, non-destructive recovery that preserves the
  surviving Rationale record. A plan that can become permanently unreadable and
  unrepairable violates the reason the tool exists.
- **CMP-R22 Stray files must not become state.** A file that merely parses must
  not be silently adopted as a Plan Version. Ingestion must be deliberate, since
  under no-delete replication a wrongly-adopted file is permanent.

### Boundary

- **CMP-R23 No foreign schema dependency.** The Compass core must not depend on
  another tool's paths, storage layouts, event envelopes, or private schemas.
- **CMP-R24 Composition is by reference.** Integrations must exchange opaque
  refs, mutations, queries, and receipts. They must not share mutable files or
  mutate Compass state directly.
- **CMP-R25 Observation follows success.** An external observation may be emitted
  only after a mutation succeeds. Its failure must never roll back, block, or
  alter the Compass result, and a failed mutation must emit no success
  observation.
