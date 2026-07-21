# Requirements: Integrations

> **Role.** The boundary where Compass *consumes* contracts it does not own: the
> catalog form published by [agent-spec](https://github.com/compoundingtech/agent-spec)
> and the replication guarantees published by
> [fabric](https://github.com/compoundingtech/fabric). What Compass defines for
> itself belongs to [01-data-model](../01-data-model/requirements.md) and
> [02-artifacts](../02-artifacts/requirements.md). Each requirement `refines:` a
> `CMP-R*`.

## Assumptions

- **CMP.INT-A01 The substrate versions independently.** The consumed contracts
  evolve on their own schedules, so expectations are pinned to observable
  behaviour rather than to a release.

## Requirements

- **CMP.INT-R01 Adopt the catalog form, do not restate it.** Content-based
  discovery, path segments supplying defaults, and retirement as an authored
  flag are conventions Compass follows. It must not publish a competing
  definition of them.
  _refines: CMP-R06, CMP-R08._

- **CMP.INT-R01a Identity is not a default.** The borrowed convention that
  content wins over path applies to values a path may supply when content omits
  them. It does not extend to identity: a file whose content disagrees with the
  location or name that identify it is rejected, not reinterpreted. Compass
  diverges from the convention here deliberately, because under no-delete
  replication a misfiled file cannot be removed once admitted.
  _refines: CMP-R07._

- **CMP.INT-R02 Define only the document.** What a version and a progress record
  contain — lineage, identity, the required Rationale, the field set — is
  Compass's own. No existing contract covers it, and Compass must not push it
  into one. _refines: CMP-R01, CMP-R08._

- **CMP.INT-R03 Declare, do not implement, replication.** Compass declares its
  catalog to the external mechanism and implements no transport, peer discovery,
  or merge algorithm. _refines: CMP-R06._

- **CMP.INT-R04 Declare the whole subtree.** The declaration covers the catalog
  recursively. A path filter that does not cross directory separators propagates
  nothing for a nested layout while reporting itself healthy — a silent failure
  indistinguishable from having nothing to sync. Compass must not narrow its
  declaration without proving the filter matches its actual depth.
  _refines: CMP-R05, CMP-R06._

- **CMP.INT-R05 Probe, degrade, report.** Absence of the mechanism is detected
  and reported, never fatal. Compass stays fully usable on one machine and says
  plainly that replication is inactive, rather than being silent in a way that
  resembles a healthy sync. _refines: CMP-R05, CMP-R06._

- **CMP.INT-R06 Source convergence from the substrate.** The
  converged-or-arriving signal comes from the replication mechanism's own state.
  It cannot be inferred from the catalog, which has no way to express
  completeness. _refines: CMP-R05._

- **CMP.INT-R07 Verify the policy, not just the presence.** Compass confirms the
  declared replication carries no-delete semantics. A mechanism that propagates
  deletes violates CMP-A01 and destroys history irrecoverably, so its
  misconfiguration must be diagnosable rather than silent.
  _refines: CMP-R05, CMP-R07._

- **CMP.INT-R08 Survive foreign file handling.** The mechanism materialises files
  under its own semantics and may not preserve modes. Compass must not depend on
  a permission bit surviving replication, and treats every received file as
  untrusted until its identity is checked.
  _refines: CMP-R07._

- **CMP.INT-R09 Replicated versions are executed.** What arrives is not data to
  be parsed but intent to be run: reading a replicated Plan evaluates modules
  authored on another machine. This must be stated at the boundary rather than
  discovered from the model, because it is the point at which the evaluation
  environment stops protecting against local accident and starts being the only
  thing standing between a peer's Plan and this machine.
  _refines: CMP-R12, CMP-T05._

- **CMP.INT-R10 An undelivered reference makes a Plan unreadable, not merely
  incomplete.** Replication that has not yet delivered something a Plan
  references leaves that Plan unresolved: it answers nothing at all, rather than
  answering with a short lineage. This is more severe than a missing predecessor
  and must be reported as its own condition, because an operator who reads it as
  an ordinary gap will wait for a Plan to fill in when what is actually missing
  is the thing without which nothing can be read. _refines: CMP-R05, CMP-R07._

- **CMP.INT-R11 Only committed content replicates.** What Compass derives to make
  reading affordable stays on the machine that derived it and is never declared
  to the replication mechanism. Shipping a derived value gives two machines
  something to disagree about that neither authored. _refines: CMP-R06, CMP-R14._
