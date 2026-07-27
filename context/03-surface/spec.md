# Spec: Plan Surface

Realizes [requirements.md](./requirements.md).

## Shape

```text
read(PlanId)     -> PlanView | NotFound | Unresolved | Stopped
ready(PlanId)    -> Readiness | NotFound | Unresolved | Stopped
commit(Module)   -> Version | Rejected
record(Progress) -> Recorded | Rejected
```

Four operations, transport-neutral. The surface holds no state; it is the
sanctioned path to the state described in
[02-artifacts](../02-artifacts/spec.md). It has exactly two write acts, and they
are not the same kind of thing: a Commit stores a Plan Version, and recording
Progress appends an inert event. The first produces new intent; the second never
does.

A read is an evaluation, so it has failure modes a lookup does not.
`Unresolved` says the Plan is here but something it references is not, and the
repair is to wait. `Stopped` says evaluation exceeded its bound in time or
memory, and the repair is not to wait — a Plan that will not terminate will not
terminate later either. Collapsing either into `NotFound` would send a caller
looking for a Plan that is sitting in front of it.

## Commit and progress

The two write acts are distinct, not two spellings of one umbrella.

A **Commit** stores a Plan Version. It names one version-producing transition —
bring a Plan into being, revise its intent, or reconcile a divergence — and it
is applied whole or not at all. Changing a Step's acceptance and retiring a Step
are revisions, not separate acts.

Recording **Progress** appends a Progress Event against a Step: start, update,
handoff, completion, evidence. It produces no version, alters no structural
intent, and is inert data — read without being evaluated. It is a write only in
that it goes through the one sanctioned path; it is nothing like a Commit
otherwise.

The command vocabulary — how each act is spelled by a caller — belongs to the
consumer, not to this contract. [04-cli](../04-cli/spec.md) defines one
spelling.

## What a Commit returns

An accepted Commit returns the **Plan Version** it stored: its identity and
lineage coordinates. Content addressing means there is no separate token to
return — the version's hash *is* the durable reference, valid forever, which is
what lets an external system record a fact about a Commit without holding Compass
state.

A rejected Commit returns `Rejected` and nothing else — no version, no partial
write, no basis for an external success record, and no change to what the caller
authored.

## Repetition

Committing the same content twice records it once.

Two Commits are the same when they carry the same authored content. That content
names its own parent, so a retry is not re-evaluated against a base that
moved while it was away: it produces the same bytes, therefore the same identity,
therefore the version that already landed. Nothing is written and the caller is
told what is already there.

A retry that was *reworded* is a different Commit by construction, since the
content differs. It is caught instead by refusing a revision that alters no Step
and no goal — which is a different rule for a different case, and the reason
both exist.

No caller-supplied key is involved. A key is a value a caller chooses, and can
therefore be reused for a different Commit, regenerated per attempt, or
forgotten, each quietly.

## Reads and convergence

A read reports the convergence state of what it read. This is not decoration: a
Plan's files may be mid-arrival, and a caller that cannot distinguish a settled
answer from a provisional one will treat superseded intent as current. Where the
substrate cannot answer, the state is reported as unknown — never assumed
settled.

## Composition

An external system may record a fact referencing a Plan Version after the Commit
is accepted. That record never becomes Compass state, and its failure is reported
separately without touching the result.

Integrations exchange references, commits, queries, and version identities. They
do not share mutable files and do not write Compass state directly — which is
what keeps CMP.SURF-R01 true in the presence of other tools.
