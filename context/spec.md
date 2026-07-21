# Spec: Compass

Realizes [requirements.md](./requirements.md). Terms are in
[ontology.md](./ontology.md).

This document holds the authority model and how the layers compose. Each layer's
mechanism belongs to its node and is not restated here.

## Status

The contract is normative. The authoring API's exact spelling, command spelling,
and the acceptance vocabulary are unresolved; see
[open-questions.md](./open-questions.md).

## Layers

| Node | Owns | Depends on |
| --- | --- | --- |
| [01-data-model](./01-data-model/spec.md) | what a Plan is: lineage, identity, revision, divergence, progress, acceptance, readiness | nothing |
| [02-artifacts](./02-artifacts/spec.md) | one realization of the model as stored modules: layout, identity, admission, integrity, repair, index | the model |
| [03-surface](./03-surface/spec.md) | the sanctioned way to read and change a Plan | the model |
| [04-cli](./04-cli/spec.md) | the operator interface | the surface |
| [05-integrations](./05-integrations/spec.md) | contracts Compass consumes: catalog form, replication | the artifacts |
| [06-evals](./06-evals/spec.md) | executable scenarios that prove the claims | the CLI |

The dependency direction is the point. The model does not know how it is stored;
the store does not know a CLI exists. A different realization of the same model
changes 02 and 05 and leaves 01 and 03 intact. That separation is what the
layering buys, and it is worth more than the indirection costs.

The realization is less freely interchangeable than that phrasing suggests, and
the honest version is worth stating: intent is a program, so 02 stores modules
and reading them is running them. A realization that stored something inert
would not be a different encoding of the same model — it would be a model in
which a dependency is a string rather than a reference, which is a different
model. What 01 owes to no realization is its vocabulary: lineage, declared
identity, divergence, acceptance, readiness are stated without naming a
language.

## Execution

Reading a Plan evaluates it, and evaluating a version evaluates everything it
references, transitively. There is no second stored form to consult instead: two
immutable artifacts per version can disagree permanently, with nothing able to
adjudicate between them.

Under replication, some of what is evaluated was authored elsewhere. Compass
therefore runs modules that arrived from other machines. This is a consequence
of intent being code rather than an oversight, and interposing a generated shim
would not change it: the shim would be executed too, and an import that no
longer refers to what was written is worse than the honest form.

What makes it acceptable is not trust but the environment. A Plan evaluates
against a global the host constructs explicitly, so it holds nothing that was
not deliberately introduced — no clock, no filesystem, no network, no
randomness, nothing whose behaviour varies by platform. The capability is absent
rather than removed, which is why the boundary does not erode as the environment
grows. A hostile Plan can compute; it cannot reach anything.

Exhaustion is the exception, and it is handled separately: evaluation is bounded
in time and memory, and a Plan that does not terminate is stopped and reported.
What the bounds are, and whether they may differ between machines, is unresolved
(DQ09).

Because reading costs an evaluation, repeated reading is served from an index
keyed by version identity, which holds no authority and can be deleted at any
time; see [02-artifacts](./02-artifacts/spec.md).

## Authority

Compass is the sole authority for what a Plan says and whether work is done.
Every change passes through the Plan Surface; nothing writes Compass state
around it, because a second writer would be a second authority.

Other systems compose by reference. They may record operational facts citing a
Receipt, but such a fact never becomes Compass state, and its absence or failure
never changes a Compass result.

Compass owns two regimes, deliberately distinct: **intent**, which is immutable
and versioned, and **progress**, which is append-only and operational. Neither
creates the other. Owning both is what makes readiness computable, and readiness
is what justifies owning both.

## Availability

Compass takes availability over consistency (CMP-T01). A local write always
succeeds and converges afterwards, rather than being serialized.

The consequences are pervasive enough to state once, at the top:

- Concurrent revision produces divergence rather than a rejected write, and
  divergence is resolved by authorship rather than automatically.
- A reconciliation can itself diverge, because nothing serializes authorship.
- Completeness cannot be read from the data, so convergence is reported by the
  replication substrate rather than inferred.
- Nothing can be unwritten. Removal is unavailable, so admission is strict and
  repair proceeds by authoring rather than editing.

Each is a cost, not an incidental detail. A design that wanted rejected writes,
automatic merges, or deletion would be a different design, and
[decision 0003](./.decisions/0003-storage-is-a-catalog-replicated-by-an-external-union-sync.md)
records what that alternative looks like.

## Worked example

A Plan created, revised, diverged, and reconciled. The exact spelling of the
authoring API is illustrative; that intent is a module, that a Step is a named
declaration, and that a revision is a function of its predecessor are not.

**1 — created.** Each Step is a declaration, and its name is its identity.

```ts
import { plan, step, evidence } from "compass"

export const reproduce = step({
  work: "Reproduce with a failing test",
  accept: evidence.test({ name: "parser::nested_groups", status: "fail" }),
})

export const fix = step({
  work: "Fix the grammar",
  dependsOn: [reproduce],
  accept: evidence.test({ name: "parser::nested_groups", status: "pass" }),
})

export default plan({
  author: "cos",
  why: `Initial plan. Parser rejects nested groups; needed before the release
        branch cuts.`,
  goal: "Nested groups parse correctly",
  steps: [reproduce, fix],
})
```

`fix` depends on `reproduce` by naming the declaration, not by spelling a
reference. There is no identifier to mistype and none to invent: a dependency
that does not resolve is not a dangling edge discovered later, it is a name that
does not exist, and it fails where it is written.

**2 — revised.** The revision imports its predecessor and is a function of it.

```ts
import prior from "./001-9f3c….ts"

export default prior.revise({
  author: "cos",
  why: `Reproduction showed the tokenizer, not the grammar, drops the closing
        delimiter. Retargeting the fix; the grammar is not at fault.`,
  edit: [prior.steps.fix.with({ work: "Fix the tokenizer's delimiter handling" })],
})
```

`fix` keeps its identity and changes its work to name the tokenizer. Identity
survives because the intended work — make nested groups parse — did not change,
and because identity is the name it was declared under rather than anything
about its text. This is the property a byte-addressed history cannot provide.

Note what the revision does *not* mention: `reproduce`. It is carried forward
untouched. There is no parameter that could have dropped it — the revision takes
edits, additions, and retirements, and nothing else — so a Step cannot go
missing while a plan is being rewritten.

**3 — diverged.** Two machines revise before replication catches up.

```text
003-a1b2…  author="cos"  why "Adding a fuzz step; one case is not enough."
003-c3d4…  author="dev"  why "Splitting the fix: tokenizer, then grammar
                              guard, so each lands reviewable."
```

Both survive; both are reported with their authors and reasons. Nobody has to
reconstruct why the plan disagreed, because both sides said so at the time.

**4 — reconciled.** A reconciliation is an ordinary revision with more than one
predecessor, and a cross-plan reference is an ordinary import.

```ts
import { reconcile } from "compass"
import fuzzSide from "./003-a1b2….ts"
import splitSide from "./003-c3d4….ts"
import release from "../pl_release/versions/007-4d81….ts"

export default reconcile({
  revises: [fuzzSide, splitSide],
  author: "cos",
  why: `Both were right. Taking the two-step split from dev, and keeping the
        fuzz step from cos as a third, gated on the split landing.`,
  edit: [
    fuzzSide.steps.fuzz.with({
      dependsOn: [splitSide.steps.guard, release.steps.branchCut],
    }),
  ],
})
```

Every Step of every predecessor is carried forward, so the only thing this
version states is what actually changed: the fuzz step waits on the guard.
Neither side's work can be dropped by choosing the other, which was the one
operation capable of losing intent without leaving a trace of what it lost.

The import of `pl_release` is what makes the dependency on another Plan
checkable rather than spelled — and it is also why that Plan must be present to
read this one. A machine that has not received it cannot evaluate this version
at all, and says so.

The lineage now reads as an argument: what was thought, what was learned, where
it disagreed, and how it settled. The intent at the tip is disposable. The four
reasons are not.
