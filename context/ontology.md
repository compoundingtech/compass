# Ontology: Compass

## The tool

**Compass**:
The tool. It owns durable planning intent and the accepted execution record for
that intent. It does not own coordination identity, messaging, presence, process
supervision, or operational accounting.
_Avoid_: planner, task runner, issue tracker

## Intent

**Plan**:
Durable authored intent for one goal: an acceptance contract plus a dependency
graph of Steps. A Plan is identified by a `PlanId`. A Plan is never edited; it
is revised, which produces a new Plan Version.
_Avoid_: ticket, issue, epic, backlog, board

**PlanId**:
A Plan's identity: the content hash of its origin — the single parent-less
version. It is derived, never declared and never minted, and encodes no
filesystem, database, transport, or host location. Two versions are the same
Plan when they share an origin; the origin version's own identity and the PlanId
are the same hash. It is machine-facing; the human handle for a Plan is its
`goal`. It is an identity, not a pointer — a cross-Plan reference imports the
other Plan's version rather than spelling this.
_Avoid_: plan path, catalog path, file name, plan name, declared id, PlanRef, reference

**Goal**:
The one outcome a Plan pursues, stated in every version's `goal` field and
required on each. It is the Plan's human handle: where a person reads or
references a Plan, Compass shows the goal, and reserves the PlanId for where
exactness is needed. A version that changes neither a Step nor the goal is
refused, because it would assert a structural change it did not make. There is
no separate title or description; the goal is the whole human-facing name.
_Avoid_: title, name, description, summary

**Plan Version**:
An immutable snapshot of a Plan's structural intent, authored as a module and
stored exactly as authored. It carries a Rationale, its author, and imports each
parent — none for the first version, one ordinarily, several when reconciling a
Divergence. Versions are created for structural change to intent, never for
operational facts, and a version that changes neither a Step nor the goal is
refused.

Identity is the hash of the version's source bytes, with nothing excluded and
nothing normalized, so the name always determines the content and any alteration
of a committed version is visible. Two versions with identical source are one
version: repeating a Commit therefore cannot repeat its effect, and no
caller-supplied token is involved.
_Avoid_: revision row, draft, resourceVersion, logical clock, rendering

**Revision**:
The act that produces a Plan Version from its parent, expressed as a function of
that parent. It may edit a Step, add one, or retire one. It has no way to remove
one: every Step of the parent is carried forward, so dropping a Step is not
something Compass refuses but something a revision cannot say.
_Avoid_: patch, diff, regeneration, overwrite

**Rationale**:
The required statement on every Plan Version explaining why intent changed,
authored as the version's `why` field. It is the durable planning record: the
artifact is the plan, the value is the Rationale chain. It is close kin to a
commit message, and differs in one respect that matters — it is attached to a
document whose Steps have identity, so a reason can be tied to a unit of work
rather than to a range of bytes.
_Avoid_: changelog entry, status note

**Step**:
A stable unit of intended work within a Plan, carrying dependencies, an
Acceptance criterion, and lifecycle. **Its identity is the name it is declared
under**, qualified by its Plan: authored rather than minted, and independent of
the Step's content, so it survives a rewording of the same intended work. The
name is not opaque and not a separate handle — depending on a Step *names the
declaration* (a language reference), so there is no identifier to invent or
mistype. A name is never reused after the Step is retired, and a Step declared
without a name has no identity and is refused.
_Avoid_: task row, checklist item, ephemeral list index, StepRef, minted id, opaque token

## Committing

**Commit**:
The act that stores authored intent as a Plan Version, and the only way a Plan
changes — there is no second writer. A Commit reads a module, evaluates it, and
stores it exactly as authored; it is an origin-creation, a Revision, or a
Reconciliation according to how many parents the module names. It is idempotent
by content: committing bytes that already landed repeats no effect, because
identical source has identical identity, and a Commit that would change neither
a Step nor the goal is refused. A rejected Commit writes nothing. Recording
Progress against a Step is not a Commit — it appends a Progress Event and
produces no version.
_Avoid_: mutation, save, apply, publish, push, plan surface, receipt

## Lineage shapes

**Head**:
The frontier of a Plan: the set of Plan Versions with no successor, derived by
walking the chain. Ordinarily this set has one member and Head reads as "the
current version." When a Plan has diverged it has several, and every query
defined over Head must have a meaning for that case. Head is computed, never
stored.
_Avoid_: current pointer, HEAD file, latest symlink

**Divergence**:
Two or more Plan Versions sharing the same parent — the observable result of
concurrent revision on different machines. It is git-style forking of one
lineage, not a rewrite collision: both versions are real and both survive.
Divergence is a legitimate state, not an error: both versions survive
replication and both are visible.

A Divergence is **open** while its sides have no common descendant, and
**settled** once a Reconciliation descends from all of them. The distinction is
load-bearing rather than cosmetic: a Divergence is a permanent fact of the
lineage and can never be removed, so a tool that does not distinguish the two
reports every historical disagreement as an outstanding problem forever, and
operators learn to ignore the report. Only an open Divergence asks anything of
anyone.
_Avoid_: conflict, collision, fork, divergent change

**Reconciliation**:
A Plan Version naming more than one parent, resolving a Divergence by stating
the reconciled intent and why. It is an ordinary Plan Version in every other
respect, and is itself capable of diverging.
_Avoid_: rebase, conflict resolution, merge commit, fixup

## Incomplete replication

**Orphan**:
A Plan Version whose parent is not present locally. Distinct from Divergence,
which it superficially resembles: divergent versions share a parent, an orphan
is missing one. An orphan ordinarily means replication is incomplete rather than
that intent disagreed, and it is repaired by waiting, not by reconciling.
_Avoid_: fork, broken chain, corruption

**Unresolved**:
A Plan that cannot be evaluated because a module it imports is not present
locally. Distinct from an Orphan, and more severe: an Orphan can be read, and
only its lineage is incomplete, whereas an Unresolved Plan cannot be read at all.
Ordinarily it means replication has not delivered the import yet, and it is
repaired by waiting; it is permanent if the import was never committed.
_Avoid_: orphan, broken plan, missing parent

## Reading, storage, and replication

**Evaluation**:
Running a Plan Version, and transitively everything it imports, to obtain what
the Plan says. Reading is evaluation — there is no second stored form to consult
instead — so reading a replicated Plan runs code authored on another machine.
Evaluation holds no capability it was not explicitly given, and is bounded in
time and memory.
_Avoid_: parsing, loading, rendering, interpretation

**Catalog**:
The on-disk tree of Plans. Discovery is content-based: the tree is walked and
files that are Plan Versions are processed, regardless of their path. Path
segments may supply defaults, but content wins.
_Avoid_: database, index, registry

**Index**:
A machine-local cache holding the evaluated form of a version, keyed by that
version's content hash. It exists because reading is evaluation and evaluation
is expensive to repeat. It has no authority: an entry is a memo of a pure
function over immutable input, so it can be deleted at any time and rebuilt on
demand, it is never replicated, and there is nothing to invalidate — a changed
module is a different hash and therefore a different key.
_Avoid_: database, source of truth, projection, materialized view

**Retired**:
A declared state marking a Plan or Step as decommissioned. Retirement is always
authored content carried forward by every later version, never a file deletion
and never an omission, because the Catalog replicates as a union with no deletes
and because a revision has no way to omit a Step in the first place.
_Avoid_: delete, archive, remove

**Convergence**:
Whether the local catalog has received everything its peers have sent. It is a
property of the replication substrate, not of the catalog: no file states how
many versions a Plan should have, so completeness cannot be read from the data.
A query answered before convergence may be answered from stale intent.
_Avoid_: sync status, freshness, consistency

## Execution record

**Progress Event**:
An append-only record of execution against a Step: start, update, handoff,
completion, evidence. Progress Events never alter structural intent and never
create a Plan Version. Unlike a version, a Progress Event is inert data: it is
read without being evaluated, and nothing in the progress layer executes.
_Avoid_: status field, state column, mutable progress

**Evidence**:
A typed fact a Progress Event records for an Acceptance criterion to read — a
test result, a measurement, a waiver — carrying whichever attributes its own
constructor names. Its vocabulary is supplied by whoever writes the Plan and is
defined nowhere in Compass, so a Plan for writing or research records evidence
on the same terms as one for software. A predicate binds the fields Compass
itself records, never attributes the payload merely claims, so a piece of
evidence cannot assert its own author.
_Avoid_: proof, result, claim-as-fact, log line

**Acceptance**:
A Step's criterion for being done: a predicate over recorded Evidence, authored
as part of the Step. It answers whether the Step is complete from what has
actually been observed, without asking a judge. Compass fixes the structure of a
criterion — combinators over atoms — and never its vocabulary. Because it is the
only thing that makes a Step done, it is also what gates the Steps that depend on
it: Compass has no separate gate concept, the acceptance predicate *is* the gate.
_Avoid_: gate, check, approval, sign-off, definition-of-done

**Readiness**:
The Plan-derived answer to what work is available now, computed from the Step
graph at Head, accepted progress, and each Step's Acceptance, together with an
explanation of which dependencies or unmet criteria stand in the way. An answer
without its explanation is not Readiness.
_Avoid_: queue, backlog, todo list, next action
