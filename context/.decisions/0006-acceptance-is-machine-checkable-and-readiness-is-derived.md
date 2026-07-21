# Acceptance is machine-checkable and readiness is derived

Status: accepted

Amends [0005](./0005-compass-owns-execution-progress.md).

## Context

Decision 0005 gave Compass ownership of progress events, and its argument was
readiness: separating intent from progress fails because what can be worked on
now is a function of the dependency graph *and* accepted progress.

That argument only holds if readiness is computable. Whether it is depends
entirely on the form acceptance takes, which 0005 left unspecified.

## Evidence and Argument

Acceptance expressed as prose — "how the Step is judged complete" — cannot be
folded. Evaluating it requires a judge, human or model, which is a separate
subsystem with its own failure modes; and giving acceptance a form Compass can
evaluate is a change to the model, not an addition to it. A design that treats
readiness as a later projection over an unchanged model is therefore mistaken
about its own data.

The consequence compounds. If readiness is not computed, nothing consumes a
progress event, and the progress layer is write-only memory — the failure that
motivated dependency-aware work tracking in the first place. Compass would then
carry every cost 0005 identified (a larger tool, unbounded event growth, a
superseded work-log surface) and realize none of its benefit. Ownership of
progress is justified by readiness or it is not justified at all.

Readiness must also have a meaning under divergence. Divergence is a normal
state of this design, and a primary query undefined in a normal state is not a
query. Reading "the graph at head" is insufficient when head is a set whose
members carry different graphs.

The root of all three is one asymmetry: references, hashes, lineage, and
supersession were each given careful structure, while the single field the model
must compute over was left as text.

## Options

| Option | Tradeoffs |
| --- | --- |
| Machine-checkable acceptance; readiness derived from it | The model is coherent and progress has a consumer; costs a predicate vocabulary designed before usage evidence exists |
| Prose acceptance, readiness judged by a model | No vocabulary to design; readiness becomes non-deterministic and unexplainable, and an unexplainable readiness cannot be trusted |
| Prose acceptance, no readiness | Smallest; leaves progress events with no consumer and 0005 without its justification |
| Prose acceptance, no progress layer either | Coherent and much smaller; reverses 0005 and gives up the unified work container it argued for |

## Decision

Acceptance is expressed in a form Compass can evaluate against recorded
evidence. Prose may accompany it but is not the criterion.

Readiness is derived from the Step graph at head, accepted progress, and gates,
and is part of the model rather than a projection over it. It explains itself:
which dependency or gate is unsatisfied.

Under divergence, readiness is reported per head member and labelled. It does
not select a side, and it does not merge graphs, which would assert intent
nobody authored.

## Consequences

- Progress events have a consumer, so no part of the model is write-only.
- 0005's justification is realized rather than promised.
- Acceptance requires a predicate vocabulary. Its shape is unresolved (DQ03) and
  is the largest open question in the model.
- Readiness must be explainable, which constrains the vocabulary: a predicate
  that cannot report *why* it failed is unusable regardless of its expressive
  power.
- Contention between concurrent workers and compaction are unaffected and remain
  outside the contract.
