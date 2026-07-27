# A Plan's identity is its origin

Status: accepted

Resolves DQ04. Completes [0012](./0012-intent-is-authored-as-code-and-identity-is-declared.md),
which settled Step identity and removed the minting that had supplied Plan
references too, without saying what replaced them.

## Context

A Step's identity is the name it is declared under (0012). A Plan has no such
declaration site — it is not an export in another module — so that mechanism does
not transfer. Minting is gone. In its absence the implementation fell back on the
catalog directory as the Plan's handle, which makes identity a filesystem
location: exactly what the ontology and CMP.DM-R08 forbid, and it means moving or
misfiling a version silently changes which Plan it belongs to.

Three candidates were on the table: a declared name, a path segment, and the
content hash of the first version.

## Evidence and Argument

The anti-minting principle (CMP-R10) is really about non-determinism — a random
value a retry regenerates. A *chosen* name does not have that defect: it is
deterministic and idempotent under retry. So a Plan being named is not the sin
minting was. The sin the implementation committed is a different one: identity in
the *path* rather than in the *content*, which no principle here permits.

That narrows it to two honest options — a name declared *in the content*, or the
content itself — and a required human summary decides between them. A Plan
already carries a required `goal`, which is human-readable and surfaced
everywhere a Plan is listed or referenced. Human intuition is therefore already
covered without the identity carrying it. Once identity does not have to be
readable, the derived option dominates: making it a declared name would overload
one value with two jobs and reintroduce an assertion that can be typed wrong,
for a readability that `goal` already provides.

So identity is the hash of the **origin** — the single predecessor-less version.
It is fully derived (CMP-R10), encodes no location (CMP.DM-R08), cannot collide,
and makes "the same Plan" a content fact: two versions belong to the same Plan
iff they descend from the same origin. There is a clean invariant in it — the
origin version's own identity *is* the PlanRef, since both are the hash of the
same bytes.

The one objection DQ04 itself raised — a hash is unavailable before the first
version exists, which seems to collide with CMP-R11 (starting must be trivial) —
does not hold. Authoring a Plan references nothing by PlanRef: a first version
declares steps and a goal, imports only `compass`, and names no plan identity.
The ref comes into being when the origin is committed, which is exactly when a
Plan first exists. Starting stays a single command; the author never types or
needs a ref.

## Options

| Option | Tradeoffs |
| --- | --- |
| Origin content hash | Fully derived, collision-free, location-independent, "same Plan" is a content fact; opaque, and unavailable until the origin is committed |
| Name declared in `plan()` | Readable and idempotent; asserts an identity that `goal` already makes readable, and can be typed wrong |
| Catalog path segment | Simplest and matches a naive implementation; makes identity a location, which CMP.DM-R08 forbids, so a moved file changes identity |

## Decision

A Plan's identity, its PlanRef, is the content hash of its origin version — the
version with no predecessor. It is derived, never declared and never minted.

Two versions are the same Plan when they share an origin. The catalog files a
Plan under its PlanRef, and a version whose derived Plan does not match where it
is filed is rejected rather than reinterpreted, on the same terms as a version
whose content does not match its own name.

`goal` is required on every version and is the human handle: what `compass`
shows in listings and references in place of the hash. Identity is machine-facing
and derived; readability is human-facing and lives in `goal`. Neither carries the
other's job.

## Consequences

- The PlanRef is not known until the origin is committed. This does not affect
  starting or authoring, which reference no PlanRef; it affects only how a Plan
  is addressed afterwards, where `goal` is the readable handle and the hash is
  the exact one.
- The origin version's identity and the PlanRef are the same hash. A Plan is,
  precisely, its first stated intent.
- Cross-plan references resolve to a PlanRef and so are content-addressed; their
  import paths are opaque, which is acceptable because they are machine-written.
- Renaming is not an operation. A Plan cannot be renamed because it was never
  named; its `goal` can be revised like any other intent, and its identity is
  unaffected because identity is the origin, not the goal.
- Moving or misfiling a version cannot change its Plan: the Plan is derived from
  the origin it descends from, and a mismatch with where it is filed is rejected.
