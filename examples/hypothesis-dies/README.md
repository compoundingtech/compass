# A hypothesis that dies

The purest case for the whole tool: a plan that is confidently wrong, and the
record of learning it was wrong.

## The situation

CI builds average 34 minutes and block every merge. The cache hit rate reads
12%, so the cache is the obvious suspect. The plan says: measure first, then fix
the cache.

The measurement comes back and kills the hypothesis. 26 of the 34 minutes are
artifact download from a single-homed mirror; cache misses cost about 90 seconds
in total. The cache was never the problem.

## What the lineage says

```
001  cos   The cache is the obvious suspect. Measure first, then fix it.
              + measure       Measure where the 34 minutes actually go
              + fixCache      Fix cache key instability

002  cos   The measurement kills the hypothesis. 26 minutes are artifact
           download from a single-homed mirror. Retiring the cache work.
              ~ fixCache      retired
              + mirror        Put build artifacts behind a regional mirror

003  cos   A mirror still leaves the cold-start case. Adding a warm path.
              + prewarm       Pre-warm the mirror on branch create
```

Read the `why` column on its own and it is an argument: *we thought it was the
cache → the measurement said it was the network → a mirror alone leaves cold
starts.* The plan at the tip — mirror plus pre-warm — is disposable. The reasoning
that arrived there is the artifact, and it survives the first version being
wrong.

## What to look at in the files

- **`002` retires `fixCache`, it does not delete it.** The abandoned hypothesis
  stays in the record with its acceptance criterion, marked retired. You can read
  what was tried and why it was dropped — which is exactly the thing a mutable
  todo list throws away.
- **`002` is a function of `001`.** It imports it and calls `.revise`. The
  `measure` step is never mentioned, and is carried forward untouched. There is
  no way the retarget could have dropped it.
- **The rationale is required.** A version that changed intent without saying why
  would be invalid — the reason is the point, not a courtesy.

## Files

This plan has no name. Its identity — its PlanId — is the content hash of its
origin, the first version (decision 0017), so the plan directory is
`634e2a7c458b`, the same hash the `001` file carries. To a person it is its goal,
"CI builds finish under 10 minutes"; the hash is only for exactness.

- [`001-634e2a7c458b.ts`](./catalog/plans/634e2a7c458b/versions/001-634e2a7c458b.ts) — the hypothesis
- [`002-68fdda593f4d.ts`](./catalog/plans/634e2a7c458b/versions/002-68fdda593f4d.ts) — it dies
- [`003-55ca61867911.ts`](./catalog/plans/634e2a7c458b/versions/003-55ca61867911.ts) — the cold-start follow-on
