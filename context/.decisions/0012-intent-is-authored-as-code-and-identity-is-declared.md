# Intent is authored as code, and identity is declared

Status: accepted

Supersedes the authoring surface of
[0010](./0010-intent-is-authored-as-a-document-then-committed.md) and the
minting mechanism of [0004](./0004-stepref-is-minted-not-content-derived.md).

## Context

Decision 0010 moved authoring from command arguments into a document, because a
composition assembled inside one process has nowhere durable to live. That
diagnosis was right and its consequences held: retries became idempotent,
reconciliation became derivable.

But the document carried three problems it could not solve. A reference had to
be minted and written back, so a value the author never chose still had to
survive in a file the author edits. A dependency was a reference typed as a
string, so it could be mistyped or invented — an editing experiment across
several document formats found that where a reference sits determines whether an
author fabricates one, and that a plausible fabrication is the worst available
outcome. And a document restates the whole plan to change one line, so the
authoring surface was permanently exposed to a class of silent loss that had to
be refused at commit rather than prevented.

## Evidence and Argument

Expressing intent in a programming language dissolves all three, because a
language already has the mechanisms a document was imitating.

A dependency is a **variable**, so a reference is never typed and cannot be
invented, mistyped, or omitted. A revision is a **function of its predecessor**
that takes edits, additions, and retirements, so there is no parameter capable of
removing a step and the silent-loss class is unrepresentable rather than
refused. A cross-plan reference is an **import**, resolved and checked rather
than spelled.

Identity follows from the same observation. A minted reference existed because
content addressing breaks under rewording. But a declaration already has a
stable, author-given, tool-checked handle — its **exported name**. Rewording the
work does not touch it; renaming it breaks every importer loudly, where a
mistyped random value fails silently. So identity can be *declared* rather than
minted, and the entire apparatus that existed to make a random value durable —
minting, write-back, provenance checking, and a ledger recording what was minted
— disappears.

This was proven rather than assumed. Evaluation is deterministic across
repetitions and across engine versions; identity survives rewording; a retired
step is carried forward and marked rather than omitted; a plan that reads the
clock is refused; and renaming a declaration in a committed version changes its
hash, so re-identification is detectable.

That last property is conditional and was nearly missed: it holds **only because
the derived identity is part of the hashed content**. Before that, renaming a
declaration silently re-identified a step while the hash stayed constant. The
requirement is therefore load-bearing, not incidental.

The cost is real and stated plainly: intent is now a program, so evaluating it
demands determinism and a capability boundary — see
[0011](./0011-the-javascript-runtime-is-embedded-not-invoked.md).

## Options

| Option | Tradeoffs |
| --- | --- |
| Intent as code, identity declared | References cannot be invented; dropping is unrepresentable; cross-plan references are checked; no minting apparatus at all. Intent becomes a program that must be evaluated safely and deterministically |
| Intent as a document, references minted | No evaluation and no capability boundary; a reference must be minted and made durable, dropping must be refused rather than prevented, and every dependency is a string an author can get wrong |
| Intent as a document, identity content-derived | No minting; identity breaks the moment work is reworded, which is the ordinary case |
| Intent as code, references still minted | Keeps a mechanism the language makes unnecessary, and reintroduces a value the author neither chose nor can verify |

## Decision

Intent is authored as code, evaluated to a canonical form, and stored
immutably.

A Step's identity is the **name it is declared under**, qualified by its Plan. It
is not minted, not random, and not derived from content. It is assigned where the
Step is first declared and carried forward by revision, so a Step that outlives
many versions keeps the identity it was born with.

A Step declared without a name has no identity and is refused. A name is never
reused once retired.

**The derived identity is part of the hashed content.** Without this, renaming a
declaration re-identifies a Step invisibly.

A revision is expressed as a function of its predecessor. It may edit, add, and
retire; it has no way to remove. Steps are carried forward, so omission is not
expressible and the drop-refusal of 0010 becomes unnecessary rather than
enforced.

Dependencies and cross-plan references are ordinary language references.

## Consequences

- Minting, write-back, reference provenance, and the minted-reference ledger are
  all removed. Four mechanisms and one invented concept disappear together.
- A rejected commit leaves authored content untouched, because nothing needs to
  be written back into it. What 0010 could only approximate becomes literally
  true.
- Renaming a declaration is a change of identity. For a Step carried forward
  this cannot arise, since it descends from its original declaration rather than
  being re-declared; for a newly declared Step before commit it is harmless. It
  is only consequential when editing already-committed source, which is
  detectable and is itself a violation.
- A Step must be a named declaration. Anonymous construction is refused, which
  is a constraint on how intent is written.
- Intent is a program. Everything about determinism and capability in 0011
  follows from this decision, and neither can be relaxed independently.
- The authored form is what replicates, what is hashed, and what is read.

## Amendment 1 — there is one artifact, not two

The final consequence originally read that the authored form and the stored form
are different artifacts, and that the stored form is never authored by hand. That
is superseded by
[0014](./0014-a-version-is-a-module-and-peer-code-is-executed.md): a committed
version *is* the authored module, stored unchanged.

The error was carried over from the document-authoring design this decision
replaced, where a draft was rendered into a separate stored form. Once intent is
code and a revision references its predecessor by importing it, a second form is
not merely unnecessary but impossible — a reference cannot be imported from a
rendering.

Nothing else in this decision depends on the distinction. Declared identity, the
absence of minting, and the unrepresentability of dropping a Step all hold
regardless of how many artifacts exist, and they are strengthened by there being
one: the declared name is inside the hashed bytes for free, rather than needing
to be deliberately carried into a derived form.
