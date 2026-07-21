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
An immutable, content-addressed snapshot of a Plan's structural intent. It
carries a Rationale, its author, a logical time, and the content hash of each
predecessor — none for the first version, one ordinarily, several when
reconciling a Divergence. Versions are created for structural change to intent,
never for operational facts.
_Avoid_: revision row, draft, resourceVersion

**Rationale**:
The required statement on every Plan Version explaining why intent changed. It
is the durable planning record: the artifact is the plan, the value is the
Rationale chain. It is close kin to a commit message, and differs in one
respect that matters — it is attached to a document whose Steps have identity,
so a reason can be tied to a unit of work rather than to a range of bytes.
_Avoid_: changelog entry, status note

**Head**:
The frontier of a Plan: the set of Plan Versions with no successor, derived by
walking the chain. Ordinarily this set has one member and Head reads as "the
current version." When a Plan has diverged it has several, and every query
defined over Head must have a meaning for that case. Head is computed, never
stored.
_Avoid_: current pointer, HEAD file, latest symlink

**Divergence**:
Two or more Plan Versions sharing the same predecessor — the observable result
of concurrent revision on different machines. Divergence is a legitimate state,
not an error: both versions survive replication and both are visible.
_Avoid_: conflict, collision, fork

**Orphan**:
A Plan Version whose predecessor is not present locally. Distinct from
Divergence, which it superficially resembles: divergent versions share a
predecessor, an orphan is missing one. An orphan ordinarily means replication is
incomplete rather than that intent disagreed, and it is repaired by waiting, not
by reconciling.
_Avoid_: fork, broken chain, corruption

**Reconciliation**:
A Plan Version naming more than one predecessor, resolving a Divergence by
stating the reconciled intent and why. It is an ordinary Plan Version in every
other respect, and is itself capable of diverging.
_Avoid_: rebase, conflict resolution, merge commit, fixup

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

**Plan Surface**:
The transport-neutral boundary for Compass queries and mutations, and the only
sanctioned way to change a Plan. It applies a mutation and returns a stable
Receipt. What makes a repeated mutation the *same* mutation — and therefore
whether application is exactly-once — is unresolved; see DQ01.
_Avoid_: port, API, event emitter, shared-files adapter

**Convergence**:
Whether the local catalog has received everything its peers have sent. It is a
property of the replication substrate, not of the catalog: no file states how
many versions a Plan should have, so completeness cannot be read from the data.
A query answered before convergence may be answered from stale intent.
_Avoid_: sync status, freshness, consistency

**Readiness**:
The Plan-derived answer to what work is available now, computed from the Step
graph at Head, accepted progress, and gates, together with an explanation of
which dependencies and gates are unsatisfied. An answer without its explanation
is not Readiness.
_Avoid_: queue, backlog, todo list, next action

**Receipt**:
The stable result of an accepted mutation, bound to its affected opaque
references and resulting Plan Version.
_Avoid_: log acknowledgement, observation id

**Observation**:
An operational fact emitted by a surrounding system after a Compass mutation
succeeds. It may reference a Receipt, PlanRef, or StepRef, but never becomes
Compass state.
_Avoid_: progress authority, completion record
