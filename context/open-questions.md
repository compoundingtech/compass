# Open Questions: Compass

**DQ01 — What governs mutation idempotency?**
A caller-supplied idempotency key and a content address answer different
questions. A key answers "did I already apply this mutation?", surviving a
reworded but equivalent retry. A content address answers "which version is
this?", and is what makes divergence visible under union replication. Using only
keys loses corruption-evidence as a first-class property; using only content
addresses means a reworded retry writes a spurious version.

The likely resolution is both at different layers — hash for version identity,
key for mutation dedup, with the receipt binding one to the other — but this is
unproven and the added surface may not earn its keep. Until it settles, the
ontology does not claim exactly-once application.

Blocked on: a prototype. The state space is small enough to enumerate —
retry, reworded retry, conflicting reuse, concurrent divergence,
divergence-then-retry — so an invariant oracle over a portable model can settle
it before any storage code exists.

**DQ02 — What is the serialization syntax?**
KDL is the working assumption for versions, following the agent-spec precedent:
it reads well for authored declarative documents with nested blocks. Progress
Events are assumed JSON, since they are machine-written and never hand-edited.

Unresolved: whether accepting multiple input formats is worth the parsing
surface for files written almost exclusively by a CLI, and whether the hash is
taken over raw bytes or a canonical form. Raw bytes are simpler and stricter; a
canonical form survives reformatting but requires the canonicalization itself to
be specified and stable forever.

**DQ03 — What vocabulary expresses acceptance?**
Decision 0006 requires acceptance to be machine-checkable, which settles that it
cannot be prose. It does not settle what the predicate language is: how evidence
is named and matched, whether predicates compose, whether they can reference
other Steps, and how a predicate that can never be satisfied is detected. This
must land before v1 and is the largest remaining design task.

**DQ04 — How are references minted without collision?**
CMP-R07 requires refs that two machines can mint concurrently without colliding
and without a coordinator. Random identifiers of sufficient width are the
obvious answer; whether to encode the minting host, and how wide is sufficient
given the catalog is walked and refs appear in prose, is unresolved.

**DQ05 — How are Plans scoped and discovered?**
The catalog is a single tree replicating across machines. Unresolved: whether a
Plan declares an owning workspace, whether agents filter by host as the agent
catalog does, and what "my plans" means when plans outlive the worktree they
started in.

**DQ06 — When does the catalog need an index?**
CMP-A03 assumes the catalog is walkable in interactive time. Discovery currently
parses every file to identify it, and the progress layer grows without bound.
The threshold at which this stops holding is unmeasured, and the ontology
currently discourages an index. Measuring it is a prerequisite to designing
compaction, since both answer the same underlying problem.
