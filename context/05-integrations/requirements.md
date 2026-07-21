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
  discovery, path segments supplying defaults with content winning, and
  retirement as an authored flag are conventions Compass follows. It must not
  publish a competing definition of them.
  _refines: CMP-R06, CMP-R08._

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
