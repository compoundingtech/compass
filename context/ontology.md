# Ontology: Compass

**Compass**:
The tool. It owns durable planning intent and the accepted execution record for
that intent. It does not own coordination identity, messaging, presence, process
supervision, or operational accounting.
_Avoid_: planner, task runner, issue tracker

**Plan**:
Durable authored intent for one goal: an acceptance contract plus a dependency
graph of Steps. A Plan is referenced by an opaque `PlanRef`. A Plan is never
edited; it is revised, which produces a new Plan Version.
_Avoid_: ticket, issue, epic, backlog, board

**Plan Version**:
An immutable, content-addressed snapshot of a Plan's structural intent. Every
Plan Version carries a Rationale and the content hash of its predecessor,
forming a chain. Versions are created for structural change to intent, never for
operational facts.
_Avoid_: revision row, draft, resourceVersion

**Rationale**:
The required prose on every Plan Version explaining why intent changed. It is
the durable planning record: the artifact is the plan, the value is the
Rationale chain.
_Avoid_: changelog entry, commit message, note

**Head**:
The current Plan Version of a Plan, derived by walking the chain. Head is
computed, never stored, so there is no mutable file for concurrent writers to
contend on.
_Avoid_: current pointer, HEAD file, latest symlink

**Fork**:
Two or more Plan Versions sharing the same predecessor — the observable result
of concurrent revision on different machines. A Fork is a legitimate state, not
an error: both versions survive replication and both are visible.
_Avoid_: conflict, collision, divergence error

**Merge Version**:
A Plan Version naming more than one predecessor, resolving a Fork by stating the
reconciled intent and why. It is an ordinary Plan Version in every other
respect.
_Avoid_: rebase, conflict resolution, fixup

**Step**:
A stable unit of intended work within a Plan, carrying dependencies, acceptance
criteria, and lifecycle. Referenced by a `StepRef`.
_Avoid_: task row, checklist item, ephemeral list index

**StepRef**:
A minted stable opaque reference to a Step. It is minted at Step creation, not
derived from Step content, so it survives a title or metadata revision while the
intended work is unchanged. It is never reused after the Step is retired.
_Avoid_: content hash, array index, title slug

**PlanRef**:
A stable opaque reference to a Plan. It encodes no filesystem, database,
transport, or host location.
_Avoid_: plan path, catalog path, file name

**Catalog**:
The on-disk tree of Plans. Discovery is content-based: the tree is walked and
files that are Plan Versions are processed, regardless of their path. Path
segments may supply defaults, but content wins.
_Avoid_: database, index, registry

**Retired**:
A declared flag marking a Plan or Step as decommissioned. Retirement is always
an authored edit, never a file deletion, because the Catalog replicates as a
union with no deletes.
_Avoid_: delete, archive, remove

**Progress Event**:
An append-only record of execution against a Step: start, update, handoff,
completion, evidence. Progress Events never alter structural intent and never
create a Plan Version.
_Avoid_: status field, state column, mutable progress

**PlanPort**:
The transport-neutral boundary for Compass queries and mutations. It applies a
mutation exactly once and returns a stable PlanReceipt.
_Avoid_: event emitter, shared-files adapter

**PlanReceipt**:
The stable result of an accepted mutation, bound to its affected opaque
references and resulting Plan Version.
_Avoid_: log acknowledgement, observation id

**Observation**:
An operational fact emitted by a surrounding system after a Compass mutation
succeeds. It may reference a PlanReceipt, PlanRef, or StepRef, but never becomes
Compass state.
_Avoid_: progress authority, completion record
