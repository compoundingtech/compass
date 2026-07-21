# Open Questions: Compass

**DQ01 — What governs mutation idempotency?**
A caller-supplied idempotency key and a content address answer different
questions. A key answers "did I already apply this mutation?", surviving a
reworded but equivalent retry. A content address answers "which version is
this?", and is what makes divergence visible under union replication. Using only
keys loses corruption-evidence as a first-class property; using only content
addresses means a reworded retry writes a spurious version.

Both at different layers — hash for version identity, key for mutation dedup,
with the receipt binding one to the other — is plausible but unproven, and the
added surface may not earn its keep. Until it resolves, the ontology does not
claim exactly-once application.

Settled by exhausting a small state space: retry, reworded retry, conflicting
key reuse, concurrent divergence, divergence followed by retry, and arrival of a
version whose predecessor is absent. Each has an observable right answer, so the
question is decidable without usage evidence.

**DQ02 — What is the serialization syntax?**
A declarative block syntax suits authored versions, which humans read and
occasionally write. Progress events are machine-written and never hand-edited,
so they have different constraints and need not share a format.

Unresolved: whether accepting multiple input formats is worth the parsing
surface for files written almost exclusively by a CLI, and whether the content
hash is taken over raw bytes or a canonical form. Raw bytes are simpler and
stricter; a canonical form survives reformatting, but requires the
canonicalization to be specified and then stable permanently, since changing it
invalidates every hash ever written.

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
