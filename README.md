# compass

Durable planning intent for coding agents.

**A plan is immutable. Planning is continuous.**

Compass stores plans as a chain of immutable versions. You never edit a plan —
you revise it, which appends a new version naming its predecessor and stating
*why* intent changed. The plan at the tip is a disposable guess; the chain of
reasons that produced it is what compounds.

Status: **design**. No code yet. The reasoning lives in [`context/`](./context/)
— three of its load-bearing choices are still open, so treat it as a rationale
record rather than a contract you could build against today.

---

## "Isn't this just git?"

It is very nearly git's object model, and the design says so out loud: immutable
snapshots naming their predecessors by hash, a required message per change,
divergence as a legitimate state, reconciliation with multiple parents. If you
are thinking *why not a `plan.md` and good commit messages* — that gets you most
of this, plus blame, bisect, signing, review, and a UI everyone already has.

Two things it doesn't get you.

**Plans outlive repositories and checkouts.** A plan in git is bound to one
repository and one worktree. Agent work spans repositories and survives the
worktree being torn down. The plan has to live somewhere that isn't a project.

**Git addresses bytes, not units of work.** Rename a step and git sees a changed
line; there is no stable sub-document identity. Compass mints a reference per
step that survives rewording, so a reason attaches to *a piece of work* rather
than to a range of characters — and so progress events and readiness can point
at something that stays put across ten revisions. This is the load-bearing idea,
and it is the one git structurally cannot provide.

What git gives that Compass gives up: `push` is a compare-and-swap, so
concurrent writers get rejected instead of diverging, and a clone is complete or
absent rather than partially arrived. Compass takes the other side — a local
write always succeeds and converges later — and pays for it in divergence you
resolve by hand. See
[decision 0003](./context/.decisions/0003-storage-is-a-catalog-replicated-by-an-external-union-sync.md),
which records that trade rather than pretending it away.

## Shape

```
catalog/plans/<plan>/versions/<seq>-<hash>.kdl    immutable, mode 0444
catalog/plans/<plan>/events/<ts>-<id>.json        append-only
```

Structural intent lives in versions. Execution lives in events. Neither creates
the other, and everything else — the current version, readiness, lineage — is
derived.

There is no head file. That removes the cell concurrent writers would contend
on, and it also removes the only signal that would tell you replication is still
in flight — so convergence has to come from the sync layer instead. Both halves
of that are real; [an earlier draft claimed only the first](./context/.decisions/0002-plans-are-immutable-versions-with-a-derived-head.md).

Point a file-sync mechanism with union / newer-wins / no-delete semantics at the
catalog directory. Without one, Compass runs single-machine and nothing else
changes. The catalog form follows
[agent-spec](https://github.com/compoundingtech/agent-spec).

## What it looks like

The [worked example](./context/spec.md#worked-example) walks a plan through
creation, revision, divergence across two machines, and reconciliation — with
the four rationales visible in sequence. That example is the product; the rest
of the design exists to make it durable.

## Not in v1

**Contention** — two agents claiming the same step is unhandled. Divergence
covers concurrent *revision*; concurrent *execution* is a different problem.

**Automatic reconciliation** — divergence is resolved by hand, and a
reconciliation can itself diverge, because there is no serialization point
anywhere. Convergence is achieved by an operator noticing.

**Compaction** — versions and events accumulate without bound, and no-delete
replication means removal isn't available as the mechanism.

See [`context/roadmap.md`](./context/roadmap.md).

## Prior art

[Beads](https://github.com/gastownhall/beads) established that agents need a
dependency-aware work graph rather than a markdown checklist, and that the
question worth answering is *what is ready*. Compass adopts both, and disagrees
about mutability and about how much machinery replication requires — Beads
reconciles concurrent edits with a version-controlled SQL database and
cell-level merge; Compass has no mutable rows to reconcile, and hands you the
residual by hand.

That's a different bet, not a proven better one. Beads has 25k stars and a
working binary; this has neither.

## License

MIT — see [LICENSE](./LICENSE).
