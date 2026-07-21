# Open Questions: Compass

**DQ01 — What governs mutation idempotency?**
A caller-supplied idempotency key and a content address answer different
questions. A key answers "did I already apply this mutation?", surviving a
reworded but equivalent retry. A content address answers "which version is
this?", and is what makes forks visible under union replication. Using only keys
loses tamper-evidence as a first-class property; using only content addresses
means a reworded retry writes a spurious version.

The likely resolution is both at different layers — hash for version identity,
key for mutation dedup, with the receipt binding one to the other — but this is
unproven and the added surface may not earn its keep.

Blocked on: a prototype. The state space is small enough to enumerate
exhaustively — retry, reworded retry, conflicting reuse, concurrent fork,
fork-then-retry — so an invariant oracle over a portable model can settle it
before any storage code exists.

**DQ02 — What is the serialization format for Plan Versions?**
KDL is the working assumption, following the agent-spec precedent: it reads well
for authored declarative documents with nested blocks, and mixed-format
discovery is already a proven pattern. Progress Events are assumed JSON, since
they are machine-written and never hand-edited.

Unresolved: whether accepting multiple input formats is worth the parsing
surface for a tool whose files are written almost exclusively by a CLI, and
whether the hash is taken over raw bytes or a canonical form. Raw bytes are
simpler and stricter; a canonical form survives reformatting.

**DQ03 — How are Plans scoped and discovered by an agent?**
The Catalog is a single tree that replicates across machines. It is unresolved
whether a Plan declares an owning workspace, whether agents filter by host the
way convoy's catalog does, and what an agent's "my plans" query means when plans
outlive the worktree they started in.

**DQ04 — What is the retention story?**
Versions and events are append-only and never deleted, so a long-lived Plan
accumulates indefinitely. Compaction that preserves the Rationale chain while
shedding event volume is likely needed eventually. Deferring until real volume
exists is deliberate; designing it now would be speculative.
