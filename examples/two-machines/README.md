# Two machines, one plan

Concurrent revision is the normal case, not an edge case. Two agents on two
machines revise the same plan before replication catches up. Neither is wrong.
This example shows the divergence that produces, and the reconciliation that
resolves it.

## The situation

A plan to fix nested-group parsing sits at version `001`. Two agents pick it up
at the same time, each from `001`, neither having seen the other's work:

- **cos, on machine A**, thinks one reproduction is not enough and adds a fuzz
  step.
- **dev, on machine B**, wants the fix split so each part lands reviewable, and
  adds a grammar guard for unterminated groups.

When the two catalogs replicate together, both `002` versions arrive. They share
predecessor `001`, so they are a **divergence** — not a sequence, a disagreement.
Both survive; nothing is silently dropped.

```
                     001  cos   Fix the tokenizer, gated on a failing test.
                       ┌──────────────┴──────────────┐
   002 (machine A) cos │                              │ dev  002 (machine B)
   + fuzz               │                              │  + guard
        "one repro is not enough"      "split so each part lands reviewable"
                       └──────────────┬──────────────┘
                     003  cos   Both were right. Keep both; gate fuzz on the guard.
```

## The reconciliation

`003` is an ordinary revision with **two** predecessors. Every step of both sides
is carried forward — the fuzz step from A and the guard from B — so nothing is
lost by "choosing a side", because there is no way to choose a side. The only
thing `003` states is what actually changed: the fuzz run now depends on the
guard, so it exercises the guarded parser.

This is the operation that, done wrong, silently loses intent. Here it cannot:
carrying-forward is the default and dropping is unrepresentable, so a
reconciliation can only *add* an edge, never quietly delete a step someone else
added.

## What to look at in the files

- **Two `002-` files with different hashes**, both importing `001`. That is a
  divergence on disk — same predecessor, two contents, both admitted. Union
  replication keeps both; neither overwrites the other.
- **`003` imports both sides** and lists them in `revises`. A reconciliation is a
  revision that names more than one predecessor; there is nothing else special
  about it.
- **The reconciliation edits one step** (`fuzz.with({ dependsOn: [...] })`) and
  mentions nothing else. Everything unmentioned is carried forward. A reader sees
  exactly the decision that was made and no noise.

## Files

This plan has no name. Its identity — its PlanId — is the content hash of its
origin, the shared base `001` (decision 0017), so the plan directory is
`8e528ff9bc56`, the same hash the `001` file carries. Both `002` sides descend
from that origin, so they are the same Plan. To a person it is its goal, "Nested
groups parse correctly"; the hash is only for exactness.

- [`001-8e528ff9bc56.ts`](./catalog/plans/8e528ff9bc56/versions/001-8e528ff9bc56.ts) — the shared base
- [`002-2c922cb978de.ts`](./catalog/plans/8e528ff9bc56/versions/002-2c922cb978de.ts) — machine A: fuzz
- [`002-e5d27ee538bf.ts`](./catalog/plans/8e528ff9bc56/versions/002-e5d27ee538bf.ts) — machine B: guard
- [`003-037d5ddb9db7.ts`](./catalog/plans/8e528ff9bc56/versions/003-037d5ddb9db7.ts) — reconciliation
