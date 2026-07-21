# Requirements: Data Model

> **Role.** What a Plan *is*, independent of how it is stored. This node defines
> lineage, identity, divergence, progress, acceptance, and readiness as logical
> structure; [02-artifacts](../02-artifacts/requirements.md) realizes it as
> files. Each requirement `refines:` a `CMP-R*`.

## Requirements

### Lineage

- **CMP.DM-R01 A Plan is a lineage of versions.** Each version records its
  predecessors, so the history of intent is reconstructible from the versions
  alone. _refines: CMP-R02._

- **CMP.DM-R02 Every version carries a Rationale.** The reason for the revision
  is part of the version, not metadata about it. _refines: CMP-R03._

- **CMP.DM-R03 Head is derived.** The frontier of a Plan is computed from the
  lineage rather than recorded, so no stored value can disagree with the
  versions. Head is a set; ordinarily it has one member.
  _refines: CMP-R02, CMP-R04._

- **CMP.DM-R04 Divergence is a state, not an error.** Versions sharing a
  predecessor are both valid and both reported. _refines: CMP-R04._

- **CMP.DM-R05 Divergence resolves by authorship.** Reconciliation is an
  ordinary version naming every predecessor it reconciles, with its own
  Rationale. Nothing reconciles automatically, and a reconciliation may itself
  diverge. _refines: CMP-R03, CMP-R04._

- **CMP.DM-R06 An absent predecessor is not divergence.** A version whose
  predecessor is unknown must be distinguished from one that disagrees. The
  first ordinarily means state is still arriving; treating it as the second
  writes permanent intent to resolve a transient condition.
  _refines: CMP-R04, CMP-R05._

- **CMP.DM-R07 Versions are attributable.** Each version records its author and
  a logical time ordered against other versions of the same Plan. Reconciling
  divergence requires knowing who wrote each side and in what order.
  _refines: CMP-R03, CMP-R04._

### Identity

- **CMP.DM-R08 References are minted, not derived.** A Plan reference and a Step
  reference are minted at creation and are independent of content, so revising
  text preserves identity and a reader can follow one unit of work across the
  whole lineage. _refines: CMP-R02._

- **CMP.DM-R09 References survive concurrent minting.** Two machines minting at
  the same time, without a coordinator, must not produce the same reference.
  _refines: CMP-R04._

- **CMP.DM-R10 Retired references are final.** A reference is never reused after
  retirement, and identity changes — split, merge, replacement — are stated
  explicitly rather than inherited silently. _refines: CMP-R02._

### Progress and acceptance

- **CMP.DM-R11 Progress is append-only.** Progress records never alter intent
  and never create a version. Correction is a further record, never an edit.
  _refines: CMP-R02._

- **CMP.DM-R12 Acceptance is evaluable.** A Step's acceptance criterion is
  expressible in a form Compass can evaluate against recorded evidence. Prose
  may accompany it but is not the criterion, because readiness cannot fold over
  prose. _refines: CMP-R01._

- **CMP.DM-R13 Only Compass judges completion.** An external record never
  completes a Step; completion follows from the Step's own acceptance criterion.
  _refines: CMP-R01._

### Readiness

- **CMP.DM-R14 Readiness is derived.** What can be worked on now follows from
  the Step graph at head, accepted progress, and gates. It is part of the model,
  not a projection over it. _refines: CMP-R01._

- **CMP.DM-R15 Readiness explains itself.** Every answer names the unsatisfied
  dependencies and gates. An answer that cannot say why is neither trustworthy
  nor debuggable, and this constrains what an acceptance criterion may express.
  _refines: CMP-R01._

- **CMP.DM-R16 Readiness is defined under divergence.** With more than one head
  member, readiness is reported per member and labelled. It never selects a side
  and never merges graphs, which would assert intent nobody authored. The normal
  state of the system must not be an undefined state of its primary query.
  _refines: CMP-R04._
