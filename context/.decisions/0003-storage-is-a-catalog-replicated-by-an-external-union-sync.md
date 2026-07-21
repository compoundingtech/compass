# Storage is a catalog replicated by an external union sync

Status: accepted

## Context

Compass state must be available to agents working across several machines and
across many repositories, and must converge without a server, a daemon of its
own, or a database requiring coordinated schema migration.

Comparable tools have answered this with a version-controlled SQL database and
cell-level three-way merge. That solves the general case of merging mutable
rows, at the cost of a substantial dependency and an operational surface —
concurrent-writer modes, migration coordination across clones, corruption modes.
The recurring criticism of that class of tool is that the idea is right and the
machinery is heavy.

## Evidence and Argument

Compass does not need the general case. Cell-level merge exists to reconcile
concurrent edits to mutable rows; after decision 0002 there are no mutable rows.
Both layers are append-only and content-addressed, so union is not an
approximation of the correct merge — it *is* the correct merge, and a genuine
concurrent revision surfaces as a Fork rather than requiring arbitration.

That reduces the storage requirement to a directory replicated with union,
newer-wins, and no deletes. A general-purpose file-sync mechanism with a catalog
policy already provides exactly this, and an adjacent tool already replicates its
declarative catalog that way. Reusing that path adds no dependency to Compass
and no third dataset to any other tool's replication contract.

The catalog form itself is borrowed from the published agent spec: content-based
discovery, path segments supplying defaults with content winning on mismatch,
environment-variable references instead of absolute paths so one tree is valid
on machines with different layouts, and `retired` as an authored flag. That last
point is forced rather than stylistic — under no-delete replication a deleted
file returns on the next sync, so retirement must be content.

Committing the catalog to each repository was considered. It makes plan history
reviewable in a pull request and gives worktree isolation for free, but binds a
plan to one repository, prevents plans from following an agent across
checkouts, and does not survive worktree teardown.

## Options

| Option | Tradeoffs |
| --- | --- |
| Catalog directory, external union file sync | No database, no daemon, no schema migration; relies on an external sync being present, and offers no history beyond the chain itself |
| Version-controlled SQL database with cell-level merge | Handles mutable rows and scales; heavy dependency and operational surface, most of which is unnecessary once state is append-only |
| Catalog committed per repository, synced by version control | Plan history is pull-request reviewable and isolation is free; plans cannot span repositories or outlive a worktree |
| Register as a dataset of an existing tool's replication | Reuses a working transport; couples Compass to that tool's schema and lifecycle, contradicting decision 0001 |

## Decision

Compass state is a Catalog: a directory tree of Plan Version and Progress Event
files, discovered by content rather than by path. Compass replicates nothing
itself. The Catalog is declared to an external file-sync mechanism configured
for union, newer-wins, and no deletes.

The Catalog root is configuration. Compass depends on no other tool's paths or
schemas, and is not registered as another tool's dataset.

Decommissioning is the `retired` flag. Files are never deleted.

## Consequences

- Compass runs single-machine with no sync configured; replication is additive.
- Union replication is correct precisely because 0002 removed every mutable
  cell; the two decisions are load-bearing for each other.
- Plans follow the agent across repositories and outlive any worktree.
- Plan history is not reviewable in a pull request. If that is later wanted, it
  is a projection, not a change of authority.
- Authored content must use environment-variable references rather than absolute
  paths, or the Catalog stops being machine-agnostic.

## Amendment 1 — a version-control repository as the catalog was never considered

The options table found room for a version-controlled SQL database but omitted
the simplest alternative: **a standalone version control repository as the
catalog.** That row defeats both objections raised against the per-repository
option — a dedicated repository is bound to no project and outlives every
worktree — and supplies history, review, signing, and replication without a
separate sync mechanism. Its absence, next to a heavier database option, made
the chosen option look uncontested.

Stated properly, the choice is availability against consistency.

A version control repository serializes: the push is a compare-and-swap, so
concurrent writers are *rejected* rather than diverging, and a clone is complete
or absent rather than partially arrived. That directly answers both problems
recorded in 0002 Amendment 1. The cost is that a local write can fail. An agent
mid-revision must then reconcile before its work is durable, and the failure
arrives at exactly the moment the agent is least able to handle it.

The union sync takes the opposite side: a local write always succeeds and
converges afterwards, at the price of hand-resolved divergence and a
convergence signal that must be obtained from the substrate rather than the
data. That is the trade this decision makes, recorded as CMP-T01.

Both are defensible. Only one was written down.

## Amendment 2 — the replication assumption must be verified, not assumed

This decision states that union replication is correct precisely because 0002
removed every mutable cell. True — and it makes an external, unenforced
guarantee load-bearing for correctness. A sync configured to propagate deletes
removes history mid-chain, and the result is indistinguishable from a plan that
was simply shorter: the failure is silent and permanent.

An assumption this critical must be checkable. Corrected by CMP-R05 and
CMP.INT-R07.

## Amendment 3 — this decision constrains compaction

Compaction is often treated as an independent concern. It is not: this decision
makes its obvious implementation unavailable. Under no-delete replication,
deleted files return on the next sync, so compaction cannot reclaim anything by
removal. A tombstone that readers honour still leaves files on disk and in the
scan path, which is the actual cost, since discovery parses every file to
identify it.

Recorded as CMP-T03: growth is unbounded until compaction is *designed*, and
its design is constrained here rather than free.
