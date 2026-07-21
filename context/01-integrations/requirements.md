# Requirements: Integrations

> **Role.** The boundary where Compass *consumes* contracts it does not own: the
> catalog form published by [agent-spec](https://github.com/compoundingtech/agent-spec)
> and the replication guarantees published by
> [fabric](https://github.com/compoundingtech/fabric). Everything Compass
> defines for itself lives in the [parent spec](../spec.md); this node covers
> only what an external system imposes. Each requirement `refines:` a `CMP-R*`.

## Assumptions

- **CMP.INT-A01 The substrate is versioned independently.** agent-spec and
  fabric evolve on their own schedules. Compass pins expectations to observable
  behaviour, not to a release.

## Requirements

- **CMP.INT-R01 Adopt the catalog form, do not restate it.** Content-based
  discovery, path segments supplying defaults with content winning, and
  `retired` as an authored flag are agent-spec's conventions. Compass must
  follow them and must not publish a competing definition.
  _refines: CMP-R15, CMP-R19._

- **CMP.INT-R02 Define only the document.** Compass owns what a Plan Version and
  a Progress Event *are* — the chain, the hashing rule, the required Rationale,
  the field set. No existing contract covers these, and Compass must not push
  them into one.
  _refines: CMP-R02, CMP-R23._

- **CMP.INT-R03 Declare, do not implement, replication.** Compass must declare
  its catalog directory to the external sync mechanism and must implement no
  transport, peer discovery, or merge algorithm of its own.
  _refines: CMP-R16._

- **CMP.INT-R04 Declare the whole subtree.** The declaration must cover the
  catalog recursively. A path-glob filter that does not cross directory
  separators silently propagates nothing for a nested layout, which is
  indistinguishable from a working sync. Compass must not narrow its declaration
  without proving the filter matches its actual depth.
  _refines: CMP-R17._

- **CMP.INT-R05 Probe, degrade, report.** Absence of the sync mechanism must be
  detected and reported, never fatal. Compass must remain fully usable on one
  machine, and must say plainly that replication is not active rather than
  appearing to be synced.
  _refines: CMP-R16, CMP-R17._

- **CMP.INT-R06 Source convergence from the substrate.** Compass must obtain its
  converged-or-receiving signal from the replication mechanism's own state
  rather than inferring it from the catalog, which cannot express completeness.
  _refines: CMP-R17._

- **CMP.INT-R07 Verify the policy, not just the presence.** Compass must confirm
  the declared sync carries no-delete semantics. A sync that propagates deletes
  violates CMP-A01 and must be reported as a misconfiguration.
  _refines: CMP-R18._

- **CMP.INT-R08 Survive foreign file handling.** The sync mechanism materialises
  files and may not preserve modes. Compass must not depend on a permission bit
  surviving replication, and must treat any received file as untrusted until its
  content-addressed name is checked.
  _refines: CMP-R20, CMP-R22._
