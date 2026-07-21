# Identity is derived, never asserted

Status: accepted

Amends [0002](./0002-plans-are-immutable-versions-with-a-derived-head.md).

## Context

Content addressing was adopted so that concurrent revision would produce two
visible versions rather than one lost write. It was not examined for a second
property it appeared to provide: whether repeating a mutation repeats its
effect.

It does not. A version's body carried `at`, a counter derived as one more than
the highest seen. An agent that revises, loses its process before reading the
result, and correctly retries will re-read the catalog, now observe its own
landed write, compute a higher `at`, and hash differently. The retry is recorded
as a further revision whose stated reason duplicates its predecessor's.

The resulting lineage asserts that intent changed and gives, as the reason,
something already recorded. Under no-delete replication it is permanent on every
machine. This corrupts the one record the system exists to keep.

## Evidence and Argument

Two mechanisms were proposed and both were wrong for the same reason.

A caller-supplied idempotency key would answer "did I already apply this," but
it is a value an actor chooses — so it can be reused for a different mutation,
regenerated per attempt, or forgotten, and each failure is quiet. Naming
versions by author and per-author sequence has the same defect in a different
place: the sequence can be stale, the author identity can collide or fall back
to a default, and a name can then assert a position the content does not
support.

The distinction that matters is not how many mechanisms exist but how much a
caller can get wrong. A derived value cannot be chosen badly because it is not
chosen. Every asserted value is a degree of freedom, and a degree of freedom in
an append-only store is permanent when misused.

Under that lens `at` is the actual defect: an ordering field that participates
in identity, and that changes on retry precisely because the retry succeeded.
It also earns nothing. It cannot order divergent siblings — by construction
neither side observed the other, so their counters are equal — and where one
version does precede another, the lineage already says so.

Excluding `at` from the hash while keeping it in the body was considered and
rejected as worse than either option: two files would then share a name and hold
different bytes, so union replication would silently keep one author's record
and discard the other. That reintroduces the lost write the design exists to
prevent, in the one place nobody would look for it.

## Options

| Option | Tradeoffs |
| --- | --- |
| Drop `at`; identity is a hash of the whole body | Retry is idempotent with no new mechanism; ordering comes from the lineage, which is where it already was |
| Keep `at`, exclude it from the hash | Retry is idempotent, but one name may hold different bytes and replication resolves that by discarding an author's record |
| Keep `at`; add caller-supplied idempotency keys | Preserves the field; adds an asserted value whose misuse is silent and permanent |
| Name versions by author and per-author sequence | One mechanism for identity and dedup; replaces a derived name with an asserted one, and depends on author identity never colliding or defaulting |
| Keep as-is | No rework; a correct retry permanently duplicates a rationale on every replica |

## Decision

A version's identity is a hash of its entire body. No field is excluded, so the
name always determines the content.

`at` is removed. Ordering follows the lineage; attribution is the `author` field
alone.

A revision that changes no Step and no goal is rejected. A retry whose rationale
was reworded would otherwise still produce a duplicate, and no derived value can
detect that, because the bodies genuinely differ. Refusing the empty revision
closes it without introducing anything a caller supplies.

Compass carries no caller-supplied idempotency key.

## Consequences

- A repeated mutation produces a byte-identical body, therefore the same name,
  therefore no second version. Idempotency is a property of the data rather than
  a protocol both sides must implement correctly.
- Two machines that independently make the same revision from the same parent
  now converge to one version instead of diverging. Identical intent is
  identical, which is the behaviour the hash always implied.
- A deliberate non-change cannot be recorded. "We reconsidered and are keeping
  this plan" is real planning content and is now inexpressible — accepted as the
  cost of having no way to express a duplicate either. Revisit if it is missed
  in practice; adding a distinct verb later is compatible with this decision,
  whereas a flag on revision would not be.
- DQ01 is resolved and closed.
- Applies beyond identity: prefer a derived value to an asserted one wherever
  both would work, and treat every field a caller sets as something that will
  eventually be set wrongly.

## Amendment 1 — the retry is closed by the empty-revision rule, not by removing `at`

The first consequence above claimed a repeated mutation "produces a
byte-identical body, therefore the same name, therefore no second version."
That is false, and implementing this decision proved it.

A retry re-reads the catalog and observes its own landed write, so head has
moved. The candidate version therefore names a *different predecessor* and sits
at a different depth. Its bytes differ regardless of what `at` does — there is
no hash collapse, and removing `at` alone would have fixed nothing. The Context
section's framing of `at` as the cause of the duplicate is likewise wrong: it
was one of three fields that shift.

What closes the retry is the refusal of a revision that changes nothing.
Re-applying the same edits to a version that already carries them alters no Step
and no goal, so it is refused.

Removing `at` earns its place for a different property: two machines with
different local history that make the same revision from the same parent now
produce identical bytes and converge to one version, instead of diverging over a
counter neither of them meant. That is worth having, and it is not what the
Context claimed.

The decision stands; two of its stated reasons did not.

## Amendment 2 — a correct retry now fails, and this is visible to the caller

An agent that crashed mid-write and correctly retries receives an error and a
non-zero exit, not a success. The record is right — one version, one rationale —
but the caller is told it failed for behaving correctly. The message says the
earlier attempt landed and points at head, which is the most that can be done
without a verb this decision does not authorize.

Related and unclosed: a retried revision that *adds a Step* is not caught. The
new Step's reference is minted fresh on each attempt, so the Step sets genuinely
differ, the empty-revision rule does not fire, and the same work is recorded
twice under two references. That is this decision's own failure mode surviving in
the one shape its reasoning did not reach, and it remains open.
