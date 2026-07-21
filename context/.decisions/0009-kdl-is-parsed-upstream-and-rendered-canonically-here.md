# KDL is parsed upstream and rendered canonically here

Status: accepted

## Context

Versions and progress records need a serialization. The surrounding tools
publish agent specifications in KDL, so a Compass catalog sitting beside an
agent catalog in the same replication tree should not introduce a second
document language for the same kind of authored declarative content.

Identity is a hash over serialized bytes, so the choice of serialization is also
a choice about what determines identity.

## Evidence and Argument

A hand-written subset was the obvious way to avoid a dependency, and it is a
trap. A subset *looks* like KDL, so readers and adjacent tooling reasonably
expect raw strings, keyword literals, type annotations, and child blocks to
work. They would not. A format that honestly announces itself as bespoke is
better than one that impersonates a specification it does not implement.

The dependency objection is weaker than it appears here. Avoiding dependencies
is a default, not a rule, and the reference implementation is the specification
maintainers' own. Writing a second, partial implementation of a published
grammar is not obviously the cheaper or safer path.

The sharper question is what determines identity. Rendering through an external
library makes that library's formatting decisions load-bearing: a version bump
that changes spacing, quoting, or key ordering would change every hash ever
written, invalidating every catalog and every committed example, and it would do
so as an *upstream* change nobody here reviewed. Parsing has no such exposure —
a parser change alters what is accepted, not what a document hashes to.

So the two directions are not symmetric, and they should not be decided
together.

## Options

| Option | Tradeoffs |
| --- | --- |
| Parse with the reference implementation, render canonically here | Real KDL on input; identity stays under local control; costs one dependency and a renderer to maintain |
| Hand-write a subset | No dependency; produces a document that claims a grammar it does not implement |
| Use the reference implementation for both | Least code and fully conformant output; makes upstream formatting the definition of identity |
| Keep a bespoke line format | No dependency and no pretence; introduces a second document language into a tree that already has one |

## Decision

Documents are parsed with the reference KDL implementation, so anything valid in
the published grammar is accepted.

Documents are rendered by a canonical form defined here, and identity is the
hash of that canonical rendering. The canonical form is owned locally and
changing it is a deliberate act with known consequences, rather than an
inherited side effect of a dependency upgrade.

This closes the second half of DQ02: identity is over a canonical form, not over
raw bytes.

## Consequences

- Compass acquires its first external dependency, for parsing only.
- A document may be authored in any valid form; it is stored in canonical form.
  Reformatting an authored document does not change its identity, because
  identity is computed after canonicalization.
- The canonical renderer is now a compatibility surface. Changing it rewrites
  every identity, so it carries the same weight as a schema change and must be
  treated as one.
- Comments in an authored document are not preserved through canonicalization.
  Versions are immutable and never rewritten, so a comment has no later document
  to survive into; anything worth keeping belongs in the Rationale, which is a
  field.
