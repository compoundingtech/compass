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
relative path, and `*` does not cross a directory separator — so a filter like
`*.kdl` matches only files at the top level. Compass's layout nests several
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
