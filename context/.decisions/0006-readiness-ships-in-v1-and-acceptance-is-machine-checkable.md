# Readiness ships in v1, and acceptance is machine-checkable

Status: accepted

Amends [0005](./0005-compass-owns-execution-progress.md).

## Context

Decision 0005 gave Compass ownership of progress events, and its entire argument
was readiness: splitting intent from progress "fails on a specific question,"
because what can be worked on now is a function of the dependency graph *and*
accepted progress.

The roadmap then deferred readiness to a later release, on the grounds that it
is "a pure fold over material Compass already owns" and "needs no schema
change."

## Evidence and Argument

Both halves of that deferral were wrong, and they fail together.

**Readiness is not a fold over the current model.** Acceptance was specified as
free prose — "how the Plan is judged complete." Prose cannot be folded.
Evaluating it requires a judge, human or model, which is an unspecified
subsystem; and giving acceptance a form Compass can evaluate *is* a schema
change. The claim used to make the deferral safe was the claim that was false.

**Deferring it makes v1 incoherent.** If readiness does not ship, nothing in v1
consumes a progress event. The progress layer becomes write-only memory — which
is precisely the failure that motivated graph-based work trackers in the first
place, and which this design set out to improve on. v1 would pay every cost 0005
identified (a larger tool, unbounded event growth, a superseded work-log surface
to migrate) and realize none of its stated benefit.

**Readiness was also undefined in the state the design makes normal.** Readiness
was described as reading "the step graph at head." Under divergence, head is not
a single version, and the two sides may carry different graphs. The flagship
query had no definition in the system's expected condition.

The three findings share one root: acceptance was left unstructured while
everything around it — references, hashes, the chain, supersession edges — was
given careful structure. The one field the model needed to compute over was the
one field left as text.

## Options

| Option | Tradeoffs |
| --- | --- |
| Structure acceptance and ship readiness in v1 | v1 is coherent and the progress layer has a consumer; costs acceptance-schema design before there is usage evidence |
| Keep readiness deferred, correct the claims | Cheapest; leaves a write-only event layer in v1 and 0005 unjustified by anything shipping |
| Defer progress as well; v1 is intent-only | Also coherent and much smaller; reverses 0005 and gives up the unified work container it argued for |
| Keep prose acceptance, judge it with a model | No schema work; makes readiness non-deterministic and unexplainable, and readiness must be explainable to be trusted |

## Decision

Readiness ships in v1. Acceptance is expressed in a form Compass can evaluate
against recorded evidence, and readiness is derived from the step graph at head,
accepted progress, and gates.

Readiness under divergence produces a defined and explained result rather than
an arbitrary one. Divergence is a normal state and the primary query must have a
meaning there.

Prose remains available as accompanying description, but it is not the
acceptance criterion.

## Consequences

- The progress layer has a consumer from the first release; nothing in v1 is
  write-only.
- 0005's justification is realized rather than promised.
- Acceptance gains a schema, which is design work that must land before v1 and
  which will need revision once real plans exercise it.
- The roadmap no longer defers readiness. Contention between concurrent workers
  and compaction remain deferred, and are unaffected by this decision.
- Readiness must be able to explain itself — which dependency or gate is
  unsatisfied — since an unexplained answer cannot be trusted or debugged.
