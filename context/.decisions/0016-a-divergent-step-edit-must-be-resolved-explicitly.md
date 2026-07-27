# A divergent Step edit must be resolved explicitly

Status: accepted

Resolves DQ08.

## Context

A reconciliation carries every Step of every predecessor forward, so no Step is
lost by omission. That leaves one case the carry-forward rule does not settle:
two predecessors that both define the *same* Step with *different* content — a
different description, a different acceptance criterion, a different dependency
set. One reconciled version must present one Step; which intent survives?

## Evidence and Argument

An implementation of reconciliation resolved this silently: it kept the first
predecessor's definition, ordered by the `revises` list, and dropped the other.
That is the precise failure the divergence model exists to prevent. A
reconciliation reported as settled would then assert intent nobody authored, and
would discard one author's work according to nothing more than argument order —
invisibly, with no record that a choice was even made.

The alternative is to refuse. If two sides disagree on a Step, the reconciliation
must state the surviving intent explicitly, with an edit for that Step, and is
refused until it does. This costs the author a keystroke and buys the guarantee
that no reconciliation ever silently chooses between two authored intents.

The two are not close. Silent resolution is fast and wrong in exactly the way
the whole design is built to avoid; explicit resolution is the same principle
divergence already follows — it is resolved by authorship, never by inference.

A Step only one side carries is not in conflict and is carried forward. A Step
both sides left identical is not in conflict either. Only a genuine disagreement
requires a choice, and only that choice must be authored.

## Options

| Option | Tradeoffs |
| --- | --- |
| Refuse a divergent same-Step edit absent an explicit resolution | No reconciliation ever silently drops an authored intent; costs an explicit edit per genuine conflict |
| Keep one side by `revises` order | No keystroke; discards one author's intent invisibly, by argument order, while reporting success |
| Merge the two definitions automatically | No keystroke; invents a third intent neither author wrote, which is worse than choosing one |

## Decision

When two predecessors of a reconciliation define the same-identity Step with
different content, the reconciliation must carry an explicit edit for that Step
stating the surviving intent. Absent it, the reconciliation is refused, naming
the Step and both differing sides.

Difference is judged over the whole Step — work, dependencies, acceptance,
supersession, retirement — not a subset, because any of them is intent a silent
choice could discard.

## Consequences

- A reconciliation never silently chooses between two authored intents; the
  choice is always in the record, authored.
- The common case — sides that touched different Steps — needs no explicit edit
  and reconciles directly.
- This is the same shape as the rest of the model: divergence resolves by
  authorship, and here the authorship is a required edit rather than an inferred
  default.
