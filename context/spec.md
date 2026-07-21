# Spec: Compass

Realizes [requirements.md](./requirements.md). Terms are in
[ontology.md](./ontology.md).

This document holds the authority model and how the layers compose. Each layer's
mechanism belongs to its node and is not restated here.

## Status

The contract is normative. Serialization syntax, command spelling, and the
repetition model are unresolved; see [open-questions.md](./open-questions.md).

## Layers

| Node | Owns | Depends on |
| --- | --- | --- |
| [01-data-model](./01-data-model/spec.md) | what a Plan is: lineage, identity, divergence, progress, acceptance, readiness | nothing |
| [02-artifacts](./02-artifacts/spec.md) | one realization of the model as files: layout, identity, admission, integrity, repair | the model |
| [03-surface](./03-surface/spec.md) | the sanctioned way to read and change a Plan | the model |
| [04-cli](./04-cli/spec.md) | the operator interface | the surface |
| [05-integrations](./05-integrations/spec.md) | contracts Compass consumes: catalog form, replication | the artifacts |

The dependency direction is the point. The model does not know it is stored in
files; the files do not know a CLI exists. A different realization of the same
model — a different serialization, a different store — changes 02 and 05 and
leaves 01 and 03 intact. That separation is what the layering buys, and it is
worth more than the indirection costs.

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

A Plan created, revised, diverged, and reconciled. Syntax is illustrative and
unresolved (DQ02); the shape is not.

**1 — created.**

```kdl
plan "pl_7Kq2" author="cos" {
  why "Initial plan. Parser rejects nested groups; needed before the release
       branch cuts."
  goal "Nested groups parse correctly"
  step "st_a1" { work "Reproduce with a failing test"
                 accept { evidence "test" name="parser::nested_groups" status="fail" } }
  step "st_b2" { work "Fix the grammar"  depends_on "st_a1"
                 accept { evidence "test" name="parser::nested_groups" status="pass" } }
}
```

**2 — revised.**

```kdl
why "Reproduction showed the tokenizer, not the grammar, drops the closing
     delimiter. Retargeting the fix; the grammar is not at fault."
```

`st_b2` keeps its reference and changes its work to name the tokenizer.
Identity survives because the intended work — make nested groups parse — did
not change. This is the property a byte-addressed history cannot provide.

**3 — diverged.** Two machines revise before replication catches up.

```text
003-a1b2…  author="cos"  why "Adding a fuzz step; one case is not enough."
003-c3d4…  author="dev"  why "Splitting the fix: tokenizer, then grammar
                                    guard, so each lands reviewable."
```

Both survive; both are reported with their authors and reasons. Nobody has to
reconstruct why the plan disagreed, because both sides said so at the time.

**4 — reconciled.**

```kdl
why "Both were right. Taking the two-step split from dev, and keeping the fuzz
     step from cos as a third, gated on the split landing."
```

The lineage now reads as an argument: what was thought, what was learned, where
it disagreed, and how it settled. The intent at the tip is disposable. The four
reasons are not.
