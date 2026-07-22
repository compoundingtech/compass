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
step reproduce present
step reproduce accepted
ready contains reproduce
read unresolved
read stopped
exit == 0
stderr contains "already committed"
```

Step identities are written literally, because they are the names the scenario
declared. Content hashes never appear in an expectation at all: identity is a
hash of a version's source, so any change to that source rewrites every hash,
and a suite pinned to them would demand regeneration on every change until
nobody read the diffs.

## Simulating replication

Multi-machine scenarios use two catalog directories and copy files between them
without deleting. That is precisely what union, newer-wins, no-delete
replication does, so a scenario reproduces divergence, orphans, and convergence
with no network and no sync mechanism:

```text
catalog A ──copy──▶ catalog B     both directions, never removing
```

An orphan is produced by copying a subset — a version whose predecessor has not
arrived yet — which is the real condition rather than a simulation of it. The
same subset produces an unresolved Plan, and a scenario distinguishes the two by
what the read returns rather than by how the files were arranged.

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
| the crash-retry | a repeated mutation records once — CMP.DM-R07a, CMP.CLI-R12 |
| the same work, said better | identity survives rewording — CMP.DM-R08 |
| a step nobody can drop | dropping is unrepresentable — CMP.DM-R07c, CMP.EVAL-R09 |
| a step is retired | dependents are stranded visibly — CMP.DM-R05b |
| a committed version, edited | tampering with stored source is detected — CMP.FS-R02b, CMP.FS-R07 |
| a plan that will not finish | evaluation is stopped, not awaited — CMP-R13, CMP.SURF-R09 |
| an import that never arrived | unresolved is not absence — CMP.DM-R06a, CMP.INT-R10 |
| evidence contradicts | acceptance is a current reading, not a latch — CMP.DM-R12 |
| a claim without standing | predicates bind recorded fields — CMP.DM-R13b |
| a word nobody else used | a record that can never match is reported — CMP.DM-R13d |
| a plan that is not code | the model is domain-neutral |
| an agent given a small task | starting is trivial — CMP-R11 |

Four of these defend claims that are easy to assert and easy to leave untested.

*The same work, said better* rewords a Step and shows one Step across the
lineage rather than a retirement and a replacement — the property a
byte-addressed history cannot provide.

*A step nobody can drop* is not a test that a check fires. It attempts to author
a revision that omits a Step and finds there is no way to say it: the Step is
carried forward regardless. What is asserted is the absence of a spelling, which
is why the scenario reads oddly and why it is worth keeping.

*A committed version, edited* alters a stored version in place and shows the
alteration surfacing — the identity no longer matches the name, and every later
version that references it says so. This is the return on making every byte
identity-bearing, and without a scenario it is only a claim.

*A plan that will not finish* and *an import that never arrived* cover the two
failures accepted when reading became execution. The first must be stopped and
reported, and must not be reported as something worth waiting for; the second
must be reported as unresolved rather than as an empty or absent Plan.

The domain-neutral scenario is deliberate. Nothing in the model mentions code,
and a suite composed only of engineering scenarios would quietly make that claim
false by never testing it. It bears restating in a system whose intent is
written as a program: the *authoring language* is code, and nothing about the
*work* has to be.

The last scenario is of a different kind and is the only one that cannot be
asserted mechanically. CMP-R11 claims starting a plan is trivial enough that
nobody reaches for a checklist instead — which is a claim about behaviour under
choice, not about output. It is measured by giving an agent a small planning task
with Compass available and observing whether it uses it, and it fails when the
agent writes its own list. A scenario that merely drives the commands would prove
the commands work while leaving the claim untested.
