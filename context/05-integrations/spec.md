# Spec: Integrations

Realizes [requirements.md](./requirements.md). The Compass-owned mechanism is in
the [parent spec](../spec.md) and is not restated here.

## What is borrowed vs. what is composed

Two different relationships, often conflated:

| Concern | Owner | Compass's relationship |
| --- | --- | --- |
| union / newer-wins / no-delete guarantee | fabric, as a sync policy | **composed** — a live dependency |
| catalog form: content-based discovery, path defaults, `retired` | agent-spec, as a convention | **borrowed** — a pattern, copied |
| what a Plan Version and Progress Event contain | Compass | defined here |

The distinction matters operationally. A borrowed convention drifting costs
nothing until someone notices. A composed guarantee drifting silently breaks
correctness, which is why it must be verified rather than trusted
(CMP.INT-R07).

## Replication declaration

Compass declares its catalog to fabric's sync engine with the `catalog` policy,
which expands to union, newer-wins, and no propagation of deletes. Conflict
ordering uses a logical counter rather than wall-clock time, so replication is
not sensitive to clock skew across machines.

The declaration covers the catalog directory **recursively and without a path
filter.**

This is deliberate and load-bearing. Fabric's include-globs match a normalised
relative path, and `*` does not cross a directory separator — so a filter naming
one extension matches only files at the top level. Compass's layout nests several
levels deep, so such a filter would propagate nothing while the sync reported
itself healthy. The failure is silent, survives restarts, and looks exactly like
"no changes to sync." Since the catalog directory is purpose-built and contains
nothing Compass did not write, syncing all of it is both correct and simpler
than any filter.

If filtering is ever genuinely needed, patterns must be anchored to cross
separators, and the declaration must be tested against the real nesting depth
rather than a flat fixture.

## Convergence

Compass asks the replication mechanism whether the catalog is converged. It does
not infer this from the catalog itself, which structurally cannot express
completeness — no file states how many versions a plan should have, so a chain
missing its middle is indistinguishable from a shorter chain.

Reads that would report authoritative state must first establish convergence.
When the catalog is still receiving, Compass reports that plainly rather than
serving a head that a pending file would supersede. Serving stale intent
silently is worse than reporting a wait, because the reader has no way to detect
the former.

Where the substrate cannot answer, Compass says so — an unknown convergence
state is reported as unknown, never assumed converged.

## Availability and degradation

The sync mechanism is optional. Compass probes for it, and its absence produces
a clear report that replication is inactive — never a failure, and never silence
that could be mistaken for a healthy single-peer sync.

Presence alone is insufficient. Compass also confirms the declared policy does
not propagate deletes, because a delete-propagating sync removes history that
cannot be recovered and whose absence is indistinguishable from a plan that was
simply shorter.

## Received files are untrusted

Files arriving through replication are treated exactly as local files are: the
content-addressed name is checked against the content before the file is
admitted as a version. Compass does not rely on file modes surviving
replication, since the sync materialises files under its own semantics. Mode
`0444` is a local accident-guard, not a property of the wire.

The name check is an integrity check and nothing more. It establishes that a
file is the file its name claims; it establishes nothing about what that file
does.

## Received versions are executed

Replication moves modules, and reading a Plan runs them. A version authored on
another machine is evaluated on this one, transitively, whenever anyone reads
the Plan that references it.

This is a consequence of intent being code rather than an oversight, and it is
where the evaluation environment earns its place. A received Plan runs against a
global the host constructs explicitly: it can compute, and it can reach nothing —
no clock, no filesystem, no network, no capability that was not deliberately
introduced. The guarantee is that the capability is *absent*, not that it was
taken away, which is why it does not weaken as the environment grows.

What that environment does not address is exhaustion, so evaluation is bounded
in time and memory. Denial of service is the most plausible hostile act once
foreign code runs on a read, and it is the one the capability boundary has
nothing to say about.

## An undelivered reference is not an ordinary gap

Replication delivers files in no particular order, so a Plan routinely
references something that has not arrived. That Plan is **Unresolved**: it
cannot be evaluated, so it reports no goal, no Steps, and no readiness.

This is more severe than an orphan and is reported as its own condition. An
orphan has an incomplete lineage and still says what it intends. An unresolved
Plan says nothing, and the difference matters at exactly the moment an operator
is deciding whether to wait: waiting is right in both cases, but only one of
them is showing anything in the meantime, and treating the silent one as the
partial one invites the conclusion that the Plan is empty.

It is repaired by the next sync, and it is permanent if what it references was
never committed anywhere.

## What replicates

Committed versions and progress events replicate. Nothing derived does. The
index Compass keeps to avoid re-evaluating what it has already evaluated is
machine-local and is not declared to the sync mechanism: it is computable from
what does replicate, so shipping it would buy nothing and would create a value
two machines could disagree about.
