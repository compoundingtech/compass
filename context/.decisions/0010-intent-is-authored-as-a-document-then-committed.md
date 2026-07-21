# Intent is authored as a document, then committed

Status: accepted

Supersedes the flag-composed write surface described in
[04-cli](../04-cli/spec.md).

## Context

A Version was composed from command arguments: one flag per step added, edited,
or retired. The composition existed only inside one process, for the duration of
one command.

That has three consequences which are not matters of taste.

A reference is minted during the mutation, so an agent that adds a step, loses
its process before reading the result, and correctly retries mints a *second*
reference and records the same work twice, permanently. Minting inside a
mutation cannot be idempotent, because minting is non-deterministic.

Head is read during the mutation, so a retry is evaluated against a base that
has already moved.

And a reconciliation must carry one side's whole graph forward, chosen by an
argument that is then discarded. The record does not say which side was chosen,
so the change cannot be derived, and a step present on the other side can vanish
with no diff line at all — the one operation in Compass that loses intent and
cannot explain itself.

## Evidence and Argument

All three share a root: **the composition has nowhere durable to live.**

Give it a file and each dissolves. References are minted into the draft, which
survives the crash, so a retry commits the same bytes. The predecessor is pinned
when the draft is materialized rather than read at commit, so the base cannot
move underneath a retry. A reconciliation states its resolved graph directly, so
the change against each predecessor is derivable and nothing is implied.

The cost is a corruption class that arguments make impossible. A revision
assembled from flags cannot drop a step, because every parent step is carried
forward and edits only append or modify in place. A regenerated document can
omit one. This is answered structurally rather than by review: a step present at
a predecessor and absent from a draft is refused, because a step that is no
longer intended is retired and never merely absent.

That refusal covers the only *silent* loss. A reworded step, a changed
predicate, a removed dependency all appear in the diff as edits against
surviving references. The residue is a predicate weakened while being
regenerated — valid, parseable, and indistinguishable from a deliberate
loosening. It is reported, not refused, because refusing it would mean refusing
every legitimate change to acceptance.

Two surfaces were considered and rejected together. Keeping arguments alongside
documents would leave one mutation with two spellings whose *failure modes
differ* — one structurally unable to drop a step, one merely refused from doing
so — making the invariant conditional on which spelling a caller chose.

## Options

| Option | Tradeoffs |
| --- | --- |
| Author the document, then commit | Retries become idempotent, reconciliation becomes derivable and per-step, one grammar; a regenerated document can weaken a predicate unnoticed |
| Compose from arguments | Cannot drop a step or weaken a predicate; cannot be idempotent, and reconciliation stays lossy and unauditable |
| Both | Every benefit of each; the central invariant becomes conditional on the spelling a caller happened to use |
| Author only the change, as a delta | Dropping becomes inexpressible rather than refused, and the document stays small; no single document then states what the plan is |

## Decision

Intent is authored as a document and committed. A draft is materialized from the
current head, edited freely, validated, and committed once as an immutable
Version.

A draft is mutable and is not state: it lives outside the catalog, is never
replicated, never admitted, and may be deleted. The catalog remains immutable
and append-only. Mutability is a property of a location, not a mode, and no file
is ever both.

The command surface never asks for a value that has one valid answer. The
predecessor, the sequence, the author, and the draft's own location are derived.
A caller names the plan and nothing else.

A step present at a predecessor and absent from a draft is refused. Removal is
retirement.

The argument-composed write surface is removed rather than kept alongside.

## Consequences

- Retrying a commit writes nothing and reports what already landed, because the
  draft is a durable input and commit is a function of it.
- Reconciliation resolves per step rather than per side, and its change is
  derivable against every predecessor. `Basis::Unrecoverable` becomes
  unreachable for anything Compass authors.
- A draft is authored against a pinned predecessor, so replication arriving
  mid-edit produces a divergence on commit rather than a rejected write. This
  follows CMP-T01 and CMP-R04 and is reported before commit as well as after.
- A weakened acceptance predicate is reported, not prevented. This is the
  accepted cost, and it is the one failure mode arguments made impossible.
- Compass gains a mutable file it did not have. It must never be readable as the
  plan: it is not replicated, and the plan is what the catalog says.
- Depends on [0009](./0009-kdl-is-parsed-upstream-and-rendered-canonically-here.md),
  which is accepted and unimplemented. A hand-authored document is the forcing
  function for it.
