# compass

Durable planning intent for coding agents.

**A plan is immutable. Planning is continuous.**

Compass stores plans as a chain of immutable, content-addressed versions. You
never edit a plan — you revise it, which appends a new version naming its
predecessor and stating *why* intent changed. The chain of rationales is the
point: the plan at the tip is disposable, the reasoning that produced it is not.

Status: **design**. The contract lives in [`context/`](./context/); a Rust
implementation follows. Expect it to change.

---

## Why not a mutable issue graph

Graph-based trackers for agents store the current state and let history fall to
the version control system, if there is one. That stores the disposable half. It
also forces a hard problem at replication time: two agents editing the same
mutable row on two machines need real merge machinery to reconcile — the reason
that class of tool reaches for a version-controlled database with cell-level
merge.

Compass has no mutable rows, so the problem does not arise. Both layers are
append-only and content-addressed, which makes union the *correct* merge rather
than an approximation. Concurrent revision on two machines produces two versions
sharing a parent; both survive replication, the divergence is visible as a fork,
and a merge version resolves it with a stated reason. Nothing is silently
overwritten, and no database is involved.

The trade is deliberate: less machinery, and forks you resolve by hand instead
of merges that resolve themselves.

## Shape

```
catalog/plans/<plan>/versions/<seq>-<hash>.kdl    immutable, mode 0444
catalog/plans/<plan>/events/<ts>-<id>.json        append-only
```

Structural intent lives in versions. Execution lives in events. Neither creates
the other.

There is no head file. The current version is derived by walking the chain — so
there is no cell for concurrent writers to contend on. That single omission is
what makes replication safe.

Discovery is content-based: the catalog is walked and files that *are* plan
versions are processed, whatever their path. Paths use environment-variable
references rather than absolute paths, so one catalog is valid on machines with
different layouts. Decommissioning is a `retired` flag, never a deletion —
under no-delete replication a deleted file simply returns.

The catalog form follows [agent-spec](https://github.com/compoundingtech/agent-spec).

## Replication

Compass replicates nothing itself. Point a file-sync mechanism with union /
newer-wins / no-delete semantics at the catalog directory. Without one, Compass
runs single-machine and nothing else changes.

## Not in v1

**Ready work** — "what can be worked on now" is a pure fold over the step graph
and accepted progress. It needs no schema change, which is why the substrate
ships first. It is also the main reason people like graph trackers, so v1
supersedes their data model before it matches their ergonomics.

**Contention** — two agents claiming the same step is unhandled. Forks cover
concurrent *revision*; concurrent *execution* is a different problem and v1 does
not pretend otherwise.

**Compaction** — versions and events accumulate without bound.

See [`context/roadmap.md`](./context/roadmap.md).

## Prior art

[Beads](https://github.com/gastownhall/beads) established that agents need a
dependency-aware work graph rather than a markdown checklist, and that the
valuable query is "what is ready." Compass takes the graph and the query, and
disagrees about mutability and about how much machinery replication requires.
