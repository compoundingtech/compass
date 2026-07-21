# Spec: Evals

Realizes [requirements.md](./requirements.md).

## Shape

An eval is one directory holding a scenario and its expectations:

```text
evals/<scenario>/
  README.md      the situation in prose, then the steps, then what must hold
  run            the scenario, driving the CLI
  expect         the claims, as observable properties
```

The prose is not decoration. A scenario that cannot be read is a test; a
scenario that cannot be run is documentation; this is both because either alone
rots — an example drifts from the tool, and a test teaches a newcomer nothing.

## Claims

Every scenario declares the requirements and decisions it exercises, and every
assertion names one. A failing assertion therefore reports which claim is now
unsupported rather than which two numbers differ:

```text
FAIL  divergence/  step 4
  claim  CMP.DM-R04 — divergence is a state, not an error
  expect head members == 2
  actual head members == 1
  the losing side is not in the catalog; a write was discarded
```

The last line is the point. A count mismatch is a symptom; "a write was
discarded" is the failure.

## Assertions

Assertions are properties, never bytes:

```text
versions == 3
heads == 2
divergence open
step <captured-ref> present
step <captured-ref> accepted
ready contains <captured-ref>
exit == 0
stderr contains "already carries exactly this intent"
```

References are captured from output during the run, never written literally,
because they are minted from randomness (CMP.EVAL-R08). Content hashes never
appear in an expectation at all: identity is a hash of the document, so a field
added anywhere rewrites every hash, and a suite pinned to them would demand
regeneration on every change until nobody read the diffs.

## Simulating replication

Multi-machine scenarios use two catalog directories and copy files between them
without deleting. That is precisely what union, newer-wins, no-delete
replication does, so a scenario reproduces divergence, orphans, and convergence
with no network and no sync mechanism:

```text
catalog A ──copy──▶ catalog B     both directions, never removing
```

An orphan is produced by copying a subset — a version whose predecessor has not
arrived yet — which is the real condition rather than a simulation of it.

## Coverage

The suite reports which requirements and decisions no scenario names. Unproven
claims are listed rather than hidden, because a suite that covers what was easy
while appearing complete is worse than a smaller honest one.

Coverage is over *claims*, not over code or inputs. A claim can be named by a
scenario and still be shallowly tested; the report says what is asserted
somewhere, not what is asserted well.

## What belongs here, and what does not

An eval exercises the operator surface and asserts what an operator observes.
Unit tests of parsing, hashing, or predicate evaluation belong beside the code:
they are faster, they localize failure better, and routing them through a
scenario would only make them slower and vaguer.

The dividing question is whether a claim is about *the system* or about *a
function*. "A retried commit writes nothing" is about the system. "SHA-256 of
the empty string is e3b0c442…" is about a function.

## Scenarios

Each scenario earns its place by defending a claim that is otherwise only
asserted:

| Scenario | Defends |
| --- | --- |
| a hypothesis that dies | the Rationale chain is the artifact — CMP-R03 |
| two machines, one plan | divergence survives replication — CMP-R04, CMP.DM-R04 |
| staleness, not disagreement | an orphan is distinguished from divergence — CMP.DM-R06 |
| the crash-retry | a repeated mutation records once — CMP.DM-R07a, decision 0010 |
| a step is retired | dependents are stranded visibly — CMP.DM-R05b |
| evidence contradicts | acceptance is a current reading, not a latch — CMP.DM-R12 |
| a claim without standing | predicates bind recorded fields — decision 0008 |
| a plan that is not code | the model is domain-neutral |

The last one is deliberate. Nothing in the model mentions code, and a suite
composed only of engineering scenarios would quietly make that claim false by
never testing it.
