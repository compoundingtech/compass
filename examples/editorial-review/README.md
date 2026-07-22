# A plan that is not code

Nothing in Compass mentions software. This plan is for writing a comparison
piece, and it works exactly like the engineering one — which is the point: the
model is domain-neutral, and this example exists so that claim is tested rather
than asserted.

## The situation

Three agent-memory tools claim the same ground and nobody has compared them on
one workload. The piece is worth writing only if the comparison is real: run all
three on one dataset, draft from it, and get an editor's sign-off before
publishing.

Then one tool changes its storage engine mid-benchmark, so the dataset is no
longer comparable across all three. The plan narrows to the two that held still,
and says so.

## What the lineage says

```
001  writer   Worth writing only if the comparison is real.
                 + benchmark   Run all three tools on one identical workload
                 + draft       Draft the comparison from the dataset
                 + publish     Editorial sign-off before publishing

002  writer   One tool changed engines mid-benchmark; the dataset is not
              comparable. Narrowing to the two that held still, and saying so.
                 ~ benchmark   Run the two stable tools on one identical workload
                 ~ draft       Draft the comparison, two tools, name the third's instability
```

## What to look at in the files

- **Acceptance is not about tests.** `benchmark` accepts on a reviewed dataset;
  `publish` accepts on `any(review(verdict=approved, actor=editor), waiver(actor=editor))`
  — an editor's approval, or an editor's explicit waiver. A human judgement is as
  valid a criterion as a passing test, and reads naturally.

- **`actor=editor` is bound to the recorded author, not a typed attribute.**
  Compass records who made an evidence claim; it does not adjudicate whether the
  claim is true. `review(actor=editor)` is satisfied by an event Compass recorded
  as *authored by* the editor — not by anyone who types `editor` into an
  attribute. Trust is "who claimed this", visible, and no stronger than that.

- **The vocabulary — `dataset`, `artifact`, `review`, `waiver`, and their
  attributes — is this use case's own.** Compass ships none of these words. They
  are typed constructors the author defines, so a misspelled attribute is a
  compile error, and Compass validates the shape of the criterion without ever
  knowing what "waiver" means.

## Files

- [`001-cfe4f8d721d2.ts`](./catalog/plans/pl_agent_memory_piece/versions/001-cfe4f8d721d2.ts) — the plan
- [`002-043517240262.ts`](./catalog/plans/pl_agent_memory_piece/versions/002-043517240262.ts) — narrowed to two tools
