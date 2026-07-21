# Spec: Plan Surface

Realizes [requirements.md](./requirements.md).

## Shape

```text
read(PlanRef)    -> PlanView | NotFound
ready(PlanRef)   -> Readiness
mutate(Mutation) -> Receipt | Rejected
```

Three operations, transport-neutral. The surface holds no state; it is the
sanctioned path to the state described in
[02-artifacts](../02-artifacts/spec.md).

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
write, and no basis for an external success record.

## Repetition

Submitting the same mutation twice must not record it twice.

What makes two submissions the same is unresolved. A caller-supplied key and a
content address answer different questions — "did I already apply this?" versus
"which version is this?" — and a reworded but equivalent retry is the case that
separates them. Until DQ01 resolves, this specification requires only that
repetition not duplicate; it does not claim exactly-once application, and the
ontology does not either.

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
