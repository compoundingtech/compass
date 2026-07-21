# Spec: Plan Surface

Realizes [requirements.md](./requirements.md).

## Shape

```text
read(PlanRef)    -> PlanView | NotFound | Unresolved | Stopped
ready(PlanRef)   -> Readiness | NotFound | Unresolved | Stopped
mutate(Mutation) -> Receipt | Rejected
```

Three operations, transport-neutral. The surface holds no state; it is the
sanctioned path to the state described in
[02-artifacts](../02-artifacts/spec.md).

A read is an evaluation, so it has failure modes a lookup does not.
`Unresolved` says the Plan is here but something it references is not, and the
repair is to wait. `Stopped` says evaluation exceeded its bound in time or
memory, and the repair is not to wait — a Plan that will not terminate will not
terminate later either. Collapsing either into `NotFound` would send a caller
looking for a Plan that is sitting in front of it.

## Mutations

A `Mutation` names one domain transition: create a Plan, revise intent, change
acceptance, record progress, retire, or reconcile a divergence. Each is applied
whole or not at all.

The command vocabulary — how a mutation is spelled by a caller — belongs to the
consumer, not to this contract. [04-cli](../04-cli/spec.md) defines one
spelling.

## Receipts

An accepted mutation returns a `Receipt`: the affected references and the
resulting version identity. A receipt remains valid for later reference, which
is what lets an external system record a fact about a mutation without holding
Compass state.

A rejected mutation returns `Rejected` and nothing else — no receipt, no partial
write, no basis for an external success record, and no change to what the caller
authored.

## Repetition

Submitting the same mutation twice records it once.

Two submissions are the same when they carry the same authored content. That
content names its own predecessor, so a retry is not re-evaluated against a base
that moved while it was away: it produces the same bytes, therefore the same
identity, therefore the version that already landed. Nothing is written and the
caller is told what is already there.

A retry that was *reworded* is a different submission by construction, since the
content differs. It is caught instead by refusing a revision that alters no Step
and no goal — which is a different rule for a different case, and the reason
both exist.

No caller-supplied key is involved. A key is a value a caller chooses, and can
therefore be reused for a different mutation, regenerated per attempt, or
forgotten, each quietly.

## Reads and convergence

A read reports the convergence state of what it read. This is not decoration: a
Plan's files may be mid-arrival, and a caller that cannot distinguish a settled
answer from a provisional one will treat superseded intent as current. Where the
substrate cannot answer, the state is reported as unknown — never assumed
settled.

## Composition

An external system may record a fact referencing a Receipt after the mutation is
accepted. That record never becomes Compass state, and its failure is reported
separately without touching the result.

Integrations exchange references, mutations, queries, and receipts. They do not
share mutable files and do not write Compass state directly — which is what
keeps CMP.SURF-R01 true in the presence of other tools.
