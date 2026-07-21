# Open Questions: Compass

**DQ01 — Resolved.** See
[decision 0007](./.decisions/0007-identity-is-derived-never-asserted.md).
Identity is a hash of the whole body with nothing excluded; the ordering field
that broke retry idempotency is removed, and a revision that changes nothing is
refused. No caller-supplied key exists, because an asserted value can be
supplied wrongly and the consequence would be permanent.

**DQ02 — Resolved.** See
[decision 0009](./.decisions/0009-kdl-is-parsed-upstream-and-rendered-canonically-here.md).
KDL, parsed by the reference implementation and rendered by a canonical form
owned here. Identity is the hash of that canonical rendering rather than of raw
bytes, so reformatting an authored document does not change what it is, and an
upstream formatting change cannot invalidate stored identities.

**DQ03 — What vocabulary expresses acceptance?**
Decision 0006 establishes that acceptance is machine-checkable, which rules out
prose. It does not establish the predicate language: how evidence is named and
matched, whether predicates compose, whether one may reference another Step, and
how a predicate that can never be satisfied is detected.

Constrained by CMP-R12: readiness must explain itself, so a predicate that
cannot report why it failed is unusable regardless of expressive power. This is
the largest open question in the model, and the contract is incomplete without
it.

**DQ04 — How are references minted without collision?**
CMP-R07 requires references two machines can mint concurrently without a
coordinator. Random identifiers of sufficient width are the obvious answer.
Unresolved: how wide is sufficient given references appear in authored prose and
are read aloud, and whether encoding the minting host is worth the loss of
opacity.

**DQ05 — What is a Plan scoped to?**
The catalog is a single tree replicating across machines. Unresolved: whether a
Plan declares an owning workspace, whether agents filter by host as the agent
catalog does, and what "my plans" means for a Plan that outlives the worktree it
began in.

**DQ06 — When does the catalog require an index?**
CMP-A03 assumes a catalog is walkable in interactive time, while discovery
parses every file to identify it and the progress layer grows without bound. The
threshold where that assumption fails is unmeasured.

It shares a root with compaction: both are responses to unbounded growth, and a
solution to either constrains the other. An index also sits uneasily with a
model whose authority is the files themselves, so its authority would have to be
derived and rebuildable.
