# Open Questions: Compass

**DQ01 — Resolved.** See
[decision 0007](./.decisions/0007-identity-is-derived-never-asserted.md) and
[decision 0014](./.decisions/0014-a-version-is-a-module-and-peer-code-is-executed.md).
Identity is a hash of a version's source with nothing excluded, and a revision
states its own predecessor, so re-submitting the same content yields the same
identity and therefore one version. A retry whose rationale was reworded is
closed separately, by refusing a revision that changes no Step and no goal. No
caller-supplied key exists, because an asserted value can be supplied wrongly
and the consequence would be permanent.

**DQ02 — What form do progress records take?**
[Decision 0014](./.decisions/0014-a-version-is-a-module-and-peer-code-is-executed.md)
settles versions: a version is the authored module, stored unchanged, and
identity is the hash of those bytes rather than of any rendering. It says
nothing about progress records, which are operational, written by Compass rather
than by hand, and must stay inert — nothing in the progress layer is evaluated,
because reading progress must not become another place foreign code runs.

Unresolved: what a progress record is serialized as, and how its identity is
computed given it is not a module and has no source bytes an author chose.

**DQ03 — What vocabulary expresses acceptance?**
Decision 0006 establishes that acceptance is machine-checkable, which rules out
prose. Decision 0013 settles that the vocabulary is not declared: Compass fixes
the structure of a Plan and nothing about the words inside a criterion, and the
defect that would otherwise be silent is caught by cross-checking a record
against the criteria it could contribute to when it is written.

What remains open is the predicate language itself: how evidence is named and
matched, whether predicates compose, and whether one may reference another Step.

Constrained by CMP.DM-R15: readiness must explain itself, so a predicate that
cannot report why it failed is unusable regardless of expressive power. This is
the largest open question in the model, and the contract is incomplete without
it.

**DQ04 — What determines a Plan's identity?**
Decision 0012 settles Step identity — the name it is declared under, qualified
by its Plan — and removes the minting mechanism that supplied both Step and Plan
references. It does not say what supplies a Plan reference instead, and
"qualified by its Plan" presupposes a Plan handle that nothing defines.

The candidates differ in what they cost. A declared name has the same virtues it
has for a Step and needs somewhere to be declared that is not itself a Plan. A
content hash of the first version is derived and stable but unreadable and
unavailable before the first version exists, which collides with CMP-R11. A path
segment makes identity a location, which CMP.FS-R05 and the ontology both
refuse.

**DQ05 — What is a Plan scoped to?**
The catalog is a single tree replicating across machines. Unresolved: whether a
Plan declares an owning workspace, whether agents filter by host as the agent
catalog does, and what "my plans" means for a Plan that outlives the worktree it
began in.

**DQ06 — Resolved.** See
[decision 0015](./.decisions/0015-the-index-is-a-content-keyed-cache-with-no-authority.md).
Reading is served from an index keyed by version content hash. It is required
rather than optional, because reading evaluates and a question spanning many
Plans is many evaluations. It holds no authority, is machine-local, and is never
replicated, so it can be deleted at any moment with nothing lost. Compaction
remains a separate problem: the index makes re-reading cheap and does nothing
about how many files exist.

**DQ07 — Is a version's author observed or claimed?**
A version is stored exactly as authored, so Compass cannot stamp it: whoever
writes the module writes the `author` field, and nothing checks it against the
identity Compass observed. Decision 0008 draws precisely this distinction for
evidence — a recorded actor can be trusted, a claimed attribute cannot — and
version authorship sits on the claimed side, while decision 0002 Amendment 3
justified recording an author on the grounds that reconciling divergence starts
with who wrote each side.

Unresolved: whether Compass refuses to commit a module whose stated author
differs from the acting identity, whether it records the observed identity
separately from the claimed one, or whether authorship is simply a claim like
any other and is documented as such. Progress records are unaffected — Compass
writes those, so their actor is observed.

**DQ08 — How does a reconciliation resolve a Step both sides edited?**
A reconciliation carries forward every Step of every predecessor, so nothing can
be lost by choosing a side. That leaves the case where two predecessors edited
the same Step differently, and the carried-forward value is ambiguous.

Unresolved: whether the reconciliation must state the surviving intent for every
such Step and is refused otherwise, or whether an unstated conflict resolves by
some rule. The first is consistent with divergence resolving by authorship; the
second would let a reconciliation assert intent nobody wrote, which is the
failure the whole divergence model exists to avoid.
**DQ09 — What are the evaluation bounds, and who sets them?**
Decision 0014 requires that evaluation be bounded in time and memory and that
exceeding a bound be reported. It does not say what the bounds are.

Unresolved, and awkward in both directions: a fixed bound is a value nobody can
tune for a large Plan, and a configurable one is a value an operator can set
wrongly — including wide enough to remove the protection — which is exactly what
CMP-R10 warns about. Whether a bound may differ between machines is the sharper
form of the question, since a Plan readable on one machine and stopped on
another is a Plan two machines disagree about.

