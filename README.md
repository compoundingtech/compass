# compass

Durable planning intent for coding agents.

**A plan is immutable. Planning is continuous.**

Compass stores plans as a chain of immutable versions. You never edit a plan —
you revise it, which appends a new version naming its predecessor and stating
*why* intent changed. The plan at the tip is a disposable guess; the chain of
reasons that produced it is what compounds.

A plan is a TypeScript module. A step is a named declaration, a dependency is a
reference to another step, and a revision is a function of its predecessor. See
[`examples/`](./examples/) for three worked plans, and
[`context/`](./context/) for why it is shaped this way.

Nothing in the model mentions code. A plan for a piece of writing works the same
way as one for a parser — a human sign-off is as valid a criterion as a passing
test. Coding agents are simply who it is built for.

Status: **early** and changing. The design is settled in `context/`; the
implementation is being rebuilt to match it. Don't point it at anything you'd
mind losing.

---

## What a plan looks like

```ts
import { plan, step, evidence } from "compass"

export const measure = step({
  work: "Measure where the 34 minutes actually go",
  accept: evidence.measurement({ of: "build-phases" }),
})

export const mirror = step({
  work: "Put build artifacts behind a regional mirror",
  dependsOn: [measure],
  accept: evidence.measurement({ of: "artifact-download", below: "120" }),
})

export default plan({
  author: "cos",
  goal: "CI builds finish under 10 minutes",
  why: "Builds average 34 minutes and block every merge.",
  steps: [measure, mirror],
})
```

A revision imports the version before it and is a function of it. It can edit,
add, and retire — it has **no way to remove** a step, because every step is
carried forward by the revision itself. A step you no longer want is retired, and
stays in the record marked as such.

```ts
import { step, evidence } from "compass"
import prior from "./001-634e2a7c.ts"

export default prior.revise({
  author: "cos",
  why: "The measurement kills the hypothesis: it's the network, not the cache.",
  retire: [prior.steps.fixCache],
  add: [
    step({
      work: "Put build artifacts behind a regional mirror",
      dependsOn: [prior.steps.measure],
      accept: evidence.measurement({ of: "artifact-download", below: "120" }),
    }),
  ],
})
```

## Why code

Because a reference to a prior step is then a **variable**, not a string — and
that removes an entire class of failure. You cannot invent a step id, mistype
one, or forget one: a dependency that doesn't resolve is a name that doesn't
exist, and it fails where you wrote it. A step's identity is the name you
declared it under, so rewording its description doesn't change what it is, and
renaming it breaks every reference loudly.

It also means the words inside `accept(...)` are typed. `test`, `measurement`,
`review`, `waiver` and their attributes are **your** vocabulary, not Compass's —
declared as typed constructors, so a misspelled attribute is a compile error
instead of a criterion that is valid and silently never matches. Compass
validates the *structure* of a criterion and stays neutral about what it means.

## "Isn't this just git?"

Its object model is close to git's: immutable snapshots naming their
predecessors, a required message per change, divergence as a legitimate state,
reconciliation with multiple parents. If you're thinking *why not a `plan.md` and
good commit messages* — that gets you a lot of this.

Two things it doesn't get you.

**Plans outlive repositories and checkouts.** A plan in git is bound to one repo
and one worktree. Agent work spans repositories and outlives the worktree. The
plan has to live somewhere that isn't a project.

**Git addresses bytes, not units of work.** Rename a step in a markdown file and
git sees a changed line; there is no stable sub-document identity. A Compass step
keeps its identity across rewording, so a reason, a progress event, and a
readiness check all point at the same unit of work ten revisions later.

What git gives that Compass gives up: `push` is a compare-and-swap, so concurrent
writers are rejected rather than diverging. Compass takes the other side — a
local write always succeeds and converges later — and pays for it in divergence
you resolve by hand. That trade is recorded, not hidden:
[decision 0003](./context/.decisions/0003-storage-is-a-catalog-replicated-by-an-external-union-sync.md).

## How it is stored

```
catalog/plans/<plan>/versions/<seq>-<hash>.ts     immutable, mode 0444
catalog/plans/<plan>/events/<ts>-<id>...          append-only
```

A committed version *is* the module you wrote, stored unchanged and named by the
hash of its bytes. There is no separate rendered form, so nothing can drift from
what you authored, and altering a committed version changes its hash — which is
how tampering is caught. A revision imports its predecessor by that hashed name,
so the lineage is a real module graph.

There is no head file. That removes the cell concurrent writers would contend
on; it also removes the signal that would tell you replication is still in
flight, so convergence comes from the sync layer instead. Point a file-sync
mechanism with union / newer-wins / no-delete semantics at the catalog. Without
one, Compass runs single-machine.

Reading a plan evaluates it — and, through its imports, its predecessors and any
plans it references. That evaluation runs against a locked-down engine with no
clock, no filesystem, no network, and no way to run code the plan didn't declare;
it has to be, because under replication those modules were authored on another
machine. Reads are served from a content-hash-keyed cache that can be deleted at
any time and never has authority. See
[decisions 0011–0015](./context/.decisions/).

## Not in scope

**Contention** — two agents claiming the same step is unhandled. Divergence
covers concurrent *revision*, not concurrent *execution*.

**Automatic reconciliation** — divergence is resolved by hand; a reconciliation
can itself diverge, since there is no serialization point.

**Compaction** — versions and events accumulate; no-delete replication means
removal isn't the mechanism.

See [`context/roadmap.md`](./context/roadmap.md).

## Prior art

[Beads](https://github.com/gastownhall/beads) established that agents need a
dependency-aware work graph rather than a markdown checklist, and that the
question worth answering is *what is ready*. Compass adopts both, and disagrees
about mutability and about how much machinery replication needs — Beads
reconciles concurrent edits with a version-controlled SQL database; Compass has
no mutable rows to reconcile, and hands you the residual by hand. A different
bet, not a proven better one.

## License

MIT — see [LICENSE](./LICENSE).
