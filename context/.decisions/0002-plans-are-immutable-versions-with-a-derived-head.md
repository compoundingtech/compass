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
  outside this decision; see [0003](./0003-storage-is-a-catalog-replicated-by-an-external-union-sync.md)
  Amendment 3 for the constraint that shapes it.
- Tamper-evidence is a property of the chain, not of file permissions.

## Amendment 1 — the concurrency claim was overstated

The Evidence section above claims the derived head makes replication safe, and
that "the concurrency safety is not an added mechanism." That is a trade
presented as a dominance. Two corrections:

**Removing the head also removes the only completeness signal.** Nothing in the
catalog states how many versions a plan should have. A reader that has received
versions 1, 2, and 4 — replication gives no ordering guarantee — either resolves
head to 2, serving superseded intent as authoritative with no indication, or
reports `{2, 4}` as divergence, which it is not: those versions do not share a
predecessor. A stored head would have made this detectable ("head names 4, its
parent is absent, wait"). The property was real; the claim that it dominated was
not. This is why convergence must come from the replication substrate
(CMP-R17), not from the chain.

**There is no compare-and-swap, so reconciliation can itself diverge.** The
comparison to distributed version control was misapplied: that system's safety
comes from an atomic reference update at the remote — precisely the mutable cell
this decision removes — not from content addressing. Without a serialization
point, two machines observing the same divergence can each author a
reconciliation with the same predecessors and different bytes, producing fresh
divergence at the next position, and again after that. Convergence is not
guaranteed by this design; it is achieved by an operator noticing.

Both corrections are accepted as the cost of CMP-T01 (availability over
consistency). The mechanism stands; the argument for it was wrong.

## Amendment 2 — the options table was stacked

Two rows were misstated, and one was missing.

The "mutable plan file, history via version control" row was dismissed with
"loses stated rationale." That is false — commit messages *are* stated
rationale, as this decision's own Context concedes. The honest objection is
narrower: a commit message describes a change to a file, and cannot address a
step whose identity survives rewording, because that system addresses bytes and
has no stable sub-document identity.

The absent row is **an immutable versioned plan document inside a version
control repository** — which would take most of this decision's benefits, and
adds a genuine compare-and-swap on push. It loses on three grounds, none of them
about rationale: a plan is bound to a repository and a checkout, so it cannot
span repositories or outlive a worktree; a rejected push makes a local write
*fail*, which contradicts CMP-T01; and the machinery is far larger than an
append-only file set requires.

That row belonged in the table when it was written. Its absence made the chosen
option look better than it was.

## Amendment 3 — versions must carry author and logical time

The field set omitted both, while this decision claims the history is
"attributable." It was not. Reconciling divergence begins with who wrote each
side and in what order, and the catalog could answer neither — offering strictly
less than the version-control history this decision declines to use. Corrected
by CMP-R09.

