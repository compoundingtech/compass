# Spec: Authoring API

Realizes [requirements.md](./requirements.md). The data model it constructs is in
[01-data-model](../01-data-model/spec.md); the evaluation of what it produces is
in [0011](../.decisions/0011-the-javascript-runtime-is-embedded-not-invoked.md)
and [0014](../.decisions/0014-a-version-is-a-module-and-peer-code-is-executed.md).

The exact spelling below is illustrative. What is normative is the shape: a Step
is a named binding, a dependency is a reference to a binding, a revision is an
operation on a predecessor value, and everything is pure construction.

## The module a plan imports

A plan imports one library. The library provides construction only — there is no
call in it that reads the clock, the filesystem, the network, or the
environment, so a plan written with it is pure by construction and not merely by
policy.

```ts
import { plan, step, evidence } from "compass"
```

## A first version

Each Step is a binding. Its name is its identity, so there is no separate
identifier to assign and none to mistype. A dependency refers to the binding, so
a dependency that does not resolve is a name that does not exist, and it fails
where it is written rather than as a dangling edge found later.

```ts
export const reproduce = step({
  work: "Reproduce with a failing test",
  accept: evidence.test({ name: "parser::nested_groups", status: "fail" }),
})

export const fix = step({
  work: "Fix the tokenizer's delimiter handling",
  dependsOn: [reproduce],
  accept: evidence.test({ name: "parser::nested_groups", status: "pass" }),
})

export default plan({
  author: "cos",
  goal: "Nested groups parse correctly",
  why: "Reproduction showed the tokenizer drops the closing delimiter.",
  steps: [reproduce, fix],
})
```

A Step declared inline in the `steps` array without a binding has no identity and
is refused: identity must be a name a reader and a successor can refer to.

`plan({...})` declares no identity of its own. A Plan's identity is the content
hash of this origin version, derived at commit (decision 0017) — there is no `id`
to write and none to get wrong. `goal` is required and is the Plan's human
handle, shown wherever a Plan is listed or referenced in place of the hash.

## A revision

A revision imports its predecessor and is an operation on it. It carries every
Step of the predecessor forward, and offers only edits, additions, and
retirements — there is no parameter that removes a Step, so a Step cannot go
missing while a plan is rewritten.

```ts
import prior from "./001-9f3c1ae4.ts"

// An added Step is a named binding too, for the same reason a first-version Step
// is: its name is its identity, and a later version refers to it by that name. An
// anonymous `step({...})` in `add` has no identity and is refused.
export const tripleCase = step({
  work: "Add a regression case for triple nesting",
  dependsOn: [prior.steps.fix],
  accept: evidence.test({ name: "parser::triple", status: "pass" }),
})

export default prior.revise({
  author: "cos",
  why: "Reproduction retargeted the fix at the tokenizer, not the grammar.",
  edit: [prior.steps.fix.with({ work: "Fix the tokenizer's delimiter handling" })],
  add: [tripleCase],
  retire: [prior.steps.someObsoleteStep],
})
```

`prior.steps.fix` refers to the carried-forward Step through the predecessor, so
an edit cannot target a Step that is not there or invent a new one. `.with(...)`
changes work, acceptance, or dependencies; it cannot change identity, because
identity is the binding and the binding is unchanged. An *added* Step is declared
the same way a first-version Step is — a named export — because it acquires
identity by the same rule, and a successor that carries it forward refers to it
by that name.

`retire` marks a Step decommissioned. It is carried forward like any other Step,
retired, because retirement is content and a revision could not omit it in any
case.

## A reconciliation

A reconciliation is a revision with more than one predecessor. Every Step of
every predecessor is carried forward, so nothing is lost by choosing a side; the
only thing the version states is what changed.

```ts
import { reconcile } from "compass"
import fuzzSide from "./003-a1b2c3d4.ts"
import splitSide from "./003-c3d4e5f6.ts"

export default reconcile({
  revises: [fuzzSide, splitSide],
  author: "cos",
  why: "Both were right. Keeping the guard and the fuzz step; gating fuzz on the guard.",
  edit: [
    fuzzSide.steps.fuzz.with({
      dependsOn: [fuzzSide.steps.fix, splitSide.steps.guard],
    }),
  ],
})
```

A Step present on one side and absent on another is carried forward from the
side that has it; the reconciliation cannot drop it. A Step both sides edited is
an open question (DQ08).

## Cross-plan references

Referring to another Plan's version is importing it. The import path reaches
across the catalog's plan-then-versions layout, so a sibling plan is two levels
up — pop `versions/`, then the plan segment. That segment is the other Plan's
PlanId, its origin hash (decision 0017), not a chosen name:

```ts
import release from "../../4d81f0a2b3c1/versions/007-4d81f0a2b3c1.ts"
```

The import is what makes the reference checkable rather than spelled — and it is
why the other Plan must be present to evaluate this one. A machine that has not
received that Plan (the one whose goal is "cut the release branch") cannot read
this one at all, and reports it as Unresolved rather than showing an incomplete
graph. The imported version is a reference,
not a predecessor: it does not become a parent of the importing version, and its
absence-of-admission there is an error, not an uncommitted-predecessor.

A Step **depending on** a Step in another Plan — `dependsOn:
[release.steps.branchCut]` — is a different and larger thing, and is **not yet
supported**. A dependency is validated against the importing version's own
Steps, and readiness folds within one Plan; a dependency edge that crosses Plans
raises questions neither answers — whether the other Plan's Step gates this one,
how readiness folds across Plans, what an out-of-Plan retirement does here. Those
are open (DQ11). A cross-plan *reference* resolves today; a cross-plan
*dependency edge* does not.

## The evidence vocabulary

`evidence` is not a fixed set the library defines. A use case declares its own
constructors, and their types are what make a mistyped attribute or an
out-of-range value a compile-time error while authoring rather than a criterion
that is valid and never matches:

```ts
// a use case's own vocabulary — the library ships none of these words
export const test = (a: { name: string; status: "pass" | "fail" }) =>
  atom("test", a)
export const measurement = (a: { of: string; below?: string; above?: string }) =>
  atom("measurement", a)
export const waiver = (a: { actor: string }) => atom("waiver", a)
```

The library provides the combinators — `all`, `any`, `not` — and the `atom`
primitive, and nothing that names a domain. This is what keeps the model
domain-neutral: Compass validates the *structure* of a criterion and never its
vocabulary. See [0013](../.decisions/0013-the-evidence-vocabulary-stays-undeclared.md).

## Compatibility

The library changes over time, and a committed version is read forever. A
version is evaluable under the API it was authored against, so the library
carries a compatibility identity and a stored version records which it used. A
version authored against an incompatible API is reported as such rather than
evaluated under the wrong one — the same distinct-failure discipline the CLI
applies to an unresolved import.

The exact mechanism — a field, an import specifier that encodes the version, or a
manifest — is open (DQ10).
