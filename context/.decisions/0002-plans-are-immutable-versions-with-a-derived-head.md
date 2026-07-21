# Plans are immutable content-addressed versions with a derived head

Status: accepted

## Context

"Plans are useless, planning is invaluable." A tool built on that premise has to
decide which half it stores. A mutable plan document stores only the useless
half — the current state — and discards the valuable half, the reasoning that
moved it there. Git history recovers some of it, but only if the plan lives in a
repository, and only as a diff without a stated reason.

Compass must also survive concurrent revision on multiple machines replicating
through a union file sync with no deletes.

## Evidence and Argument

These two requirements have a common solution.

If a plan is a chain of immutable, content-addressed versions, each naming its
predecessor and carrying a required Rationale, then the history *is* the
planning record — diffable, attributable, and readable without a version control
system.

The concurrency property follows from refusing to store a head. A mutable
"current version" pointer is a single cell two machines can write, and union
sync must then arbitrate — silently losing one write. If head is instead derived
by walking the chain, there is no cell to contend on. Two concurrent revisions
write two files with the same parent and different content hashes; union sync
preserves both; the divergence becomes a visible Fork that a Merge Version
resolves with a stated reason.

Notably, the concurrency safety is not an added mechanism. It falls out of
immutability plus content addressing plus the absence of a head file — which is
the same reason distributed version control works.

Filesystem permissions were considered as the enforcement mechanism and rejected
as insufficient alone: on a filesystem the writer owns, writes cannot be
prevented, and OS-level immutability flags are root-only and not portable. Mode
`0444` is retained as accident-prevention — it stops a careless or automated
in-place edit — while the hash chain is what makes any violation detectable.
The two differ in kind and neither substitutes for the other.

## Options

| Option | Tradeoffs |
| --- | --- |
| Immutable content-addressed versions, derived head | History is first-class and forks are visible; costs file proliferation and a chain walk to resolve head |
| Mutable plan file, history via version control | Cheapest; requires the plan to live in a repository, loses stated rationale, and gives no answer for non-repo replication |
| Immutable versions with a stored head pointer | Simple head resolution; reintroduces the one mutable cell that union sync cannot safely merge |
| Event-sourced plan with no versions | Uniform append-only model; structural intent becomes a fold rather than a readable document, and a human can no longer read "the plan" |

## Decision

A Plan is a chain of immutable Plan Versions. Each version is content-addressed,
written mode `0444`, names its predecessor by content hash, and carries a
required Rationale explaining why intent changed.

Head is derived by walking the chain and is never stored. Concurrent revision
produces a Fork, which is a legitimate observable state resolved by an ordinary
Plan Version naming multiple parents.

Versions are created for structural change to intent only. Operational facts —
progress, evidence, handoff, review — are Progress Events and never create a
version.

## Consequences

- The Rationale chain is the durable planning record and the reason the tool
  exists; a version without a Rationale is invalid.
- Nothing is ever silently overwritten, including across machines.
- Readers must walk the chain; `compass verify` validates it and reports breaks.
- Long-lived plans accumulate versions and events without bound. Compaction is
  deferred (see roadmap) rather than designed speculatively.
- Tamper-evidence is a property of the chain, not of file permissions.
