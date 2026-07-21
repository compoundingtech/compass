# The index is a content-keyed cache with no authority

Status: accepted

## Context

Decision 0014 makes a version a module, so reading a plan evaluates it and
everything it imports. A question spanning many plans therefore becomes many
module-graph evaluations, and the assumption that a catalog can be scanned in
interactive time stops holding much earlier than it did when reading meant
parsing a document.

Something must stand between reads and evaluation. What that something is
allowed to claim is the decision.

## Evidence and Argument

Any cache raises one question: how do you know it is still correct? The usual
answers are unsatisfying — timestamps that lie across machines, generation
counters that must be maintained, invalidation that must be remembered at every
write.

Here the question can be removed rather than answered. A version is named by the
hash of its content, so **keying the cache by that hash makes staleness
impossible**. A module that changed is a different hash and therefore a different
key; the old entry is not wrong, it simply describes a version that still exists
and still evaluates that way. A lookup either hits or misses. There is nothing to
invalidate and nothing to remember.

That property also decides authority. An entry is a memo of a pure function over
immutable input, so it can carry no information the modules do not already
determine. It follows that deleting the whole cache is always safe, that a
corrupt entry is repaired by discarding it, and that the cache cannot be a second
source of truth even by accident.

It also decides replication. A memo derivable from replicated data need not
itself replicate, and replicating it would mean shipping a value another machine
can compute — while making it something that could disagree between machines,
which is the one thing this shape rules out.

Committing the evaluated form alongside each version was considered seriously,
because it would let a reading machine avoid evaluation entirely and would
restore the boundary 0014 gave up. It was rejected because it creates two
artifacts per version that can disagree, and because the disagreement would be
permanent: both would be immutable and replicated, with no way to establish which
was right.

## Options

| Option | Tradeoffs |
| --- | --- |
| Content-keyed derived cache, machine-local | Staleness is structurally impossible; deletion is always safe; carries no authority. Does not remove evaluation, only repeats of it |
| Evaluated form committed beside each version | Reading machines never evaluate, restoring the execution boundary; two immutable artifacts that can disagree with no way to adjudicate |
| One database over the whole catalog | Fastest for questions spanning many plans; a single artifact spanning many versions can be partially stale, and reintroduces mutable state |
| No cache | Nothing to build or reason about; cost grows with catalog size, and the threshold is unmeasured |

## Decision

Reads are served from a cache keyed by version content hash, holding the
evaluated form of that version.

The cache has **no authority**. Committed modules are the only source of truth.
An entry is a memo of a pure function over immutable input, so it can be
discarded at any time and rebuilt on demand.

The cache is machine-local and is never replicated, because it is derivable from
what does replicate.

A miss is ordinary and costs an evaluation. There is no invalidation, because a
changed module produces a different key.

## Consequences

- Deleting the cache is always safe and never loses information. This is the
  property that makes it trustworthy: anything that can be thrown away cannot
  become load-bearing.
- Nothing needs to track freshness, and no write path needs to remember to
  invalidate.
- Evaluation still happens, and still executes modules authored elsewhere; the
  cache reduces how often, not whether. The capability boundary is unaffected.
- Entries accumulate for versions no longer at the frontier. Reclaiming them is
  ordinary cache eviction rather than a question about the model, since nothing
  is lost by evicting.
- An Unresolved Plan cannot be cached, because it cannot be evaluated. Reads of
  it fail on every attempt until its imports arrive.
