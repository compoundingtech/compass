# Requirements: Compass

Realizes [vision.md](./vision.md). Terms are defined in
[ontology.md](./ontology.md); the authority model is in [spec.md](./spec.md).

This document holds the mechanism-agnostic contract. Each child node refines it
for one layer:

| Node | Owns |
| --- | --- |
| [01-data-model](./01-data-model/requirements.md) | what a Plan is, independent of how it is stored |
| [02-artifacts](./02-artifacts/requirements.md) | one realization of that model as stored modules, and the derived index over them |
| [03-surface](./03-surface/requirements.md) | the logical query and mutation surface |
| [04-cli](./04-cli/requirements.md) | the operator surface over that port |
| [05-integrations](./05-integrations/requirements.md) | contracts Compass consumes rather than defines |
| [06-evals](./06-evals/requirements.md) | executable scenarios proving the claims above hold |

## Assumptions

- **CMP-A01 Replication is union-shaped.** The external sync mechanism provides
  union, newer-wins, and no-delete semantics. Compass correctness depends on
  this and Compass cannot impose it, so it must be verifiable rather than
  assumed.
- **CMP-A02 Concurrent writers are normal.** Several agents on several machines
  revise plans at once. Concurrency is the design centre, not an edge case.
- **CMP-A03 Evaluation is reproducible.** Authored intent evaluates to the same
  result on every machine and across engine versions. Agreement about what a
  Plan says — its Steps, their identities, their acceptance — rests on this, so
  it must be tested rather than assumed, and any widening of what evaluation can
  reach is a change to this assumption.

## Acceptable Tradeoffs

- **CMP-T01 Availability over consistency.** A local write never fails and
  converges later, rather than being serialized by a lock or a
  compare-and-swap. The price is real: divergence must be resolved by hand, and
  Compass cannot offer the "your write was rejected, reconcile first" signal
  that a consistent store gives for free.
- **CMP-T02 Corruption-evidence, not tamper-proofing.** Storage and integrity
  mechanisms detect accidental and mechanical damage. They do not resist a
  determined writer, who owns the files and can recompute any derived value. No
  storage mechanism here is a security boundary. This is a statement about
  storage only; the environment intent is evaluated in is a correctness boundary
  and is not covered by this tradeoff (CMP-R12).
- **CMP-T03 Growth is unbounded until compaction exists.** Under no-delete
  replication, removal is unavailable as a reclamation strategy, so compaction
  must be designed rather than assumed.
- **CMP-T04 Plan history is not pull-request reviewable.** The catalog is not a
  repository. Review of intent, if wanted, is a projection.

- **CMP-T05 Reading executes code authored elsewhere.** Intent is a program, so
  reading a Plan evaluates it and everything it imports — including modules that
  arrived by replication from another machine. This is accepted rather than
  worked around, because the alternatives cost either the reference model that
  makes intent authorable or a second artifact per version that can disagree
  with the first. What makes it acceptable is the evaluation environment
  (CMP-R12) and the bound on evaluation (CMP-R13), not trust in the peer.

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

- **CMP-R09 Composition is by reference.** Integrations must exchange stable
  references, mutations, queries, and receipts. They must not share mutable
  files or mutate Compass state directly. A reference is stable — it survives
  revision and names one thing forever — but it is not required to be
  meaningless: a Step is referenced by the name it was declared under, which a
  reader can read.

- **CMP-R10 Prefer derived values to asserted ones.** Where a value could be
  computed from existing state or supplied by a caller, it must be computed. A
  value a caller sets is a value a caller can set wrongly, and in an append-only
  store a wrong value is permanent. This constrains version identity, ordering,
  and deduplication in particular: none of them may rest on something an actor
  chooses.

  The objection is to values an actor supplies and cannot check, not to values
  an actor writes deliberately. A name a Step is declared under is authored, but
  every use of it is checked where it is used, so getting it wrong fails loudly
  at the point of the mistake. A value with that property is not what this
  requirement forbids; an opaque token that must be carried correctly by hand
  is.

- **CMP-R11 Starting a plan must be trivial.** Beginning a plan must cost one
  command and produce something immediately workable, with nothing to import,
  configure, or look up. Ceremony must scale with a plan's size, never with the
  act of starting one.

  This is a requirement rather than an ergonomic preference because the vision
  claims nothing about the tool should tempt anyone to keep plans elsewhere, and
  a hand-written checklist is always available and always cheap. A tool that is
  heavier at the first step loses to the checklist before any of its guarantees
  can matter. Whether it holds is observable — an agent given a small planning
  task either reaches for Compass or does not — so it is testable rather than
  asserted.

- **CMP-R12 Evaluating intent grants no capability.** Reading a Plan runs it, so
  evaluation must happen in an environment that holds nothing it was not
  explicitly given — no clock, no filesystem, no network, no source of
  randomness, nothing platform-dependent. The requirement is that a capability
  be *absent* rather than removed: a boundary built by taking things away widens
  silently every time the environment gains something. This is what makes
  CMP-T05 tolerable, so it is a correctness boundary and not a convenience.

- **CMP-R13 Evaluation is bounded.** Intent that does not terminate, or that
  allocates without limit, must be stopped and reported rather than allowed to
  consume the reader. Once reading runs code that arrived from elsewhere,
  exhaustion is a reachable state rather than a theoretical one, and it is the
  one failure CMP-R12 does not address.

- **CMP-R14 Derived state carries no authority.** Anything Compass computes and
  keeps in order to make reading affordable must be discardable at any moment
  with nothing lost, and must never be consultable as a second account of what a
  Plan says. Derived state that can become load-bearing is a second source of
  truth that nobody declared.
