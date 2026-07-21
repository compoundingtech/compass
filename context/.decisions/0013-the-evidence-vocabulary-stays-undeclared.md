# The evidence vocabulary stays undeclared

Status: accepted

## Context

Compass fixes the structure of a Plan and rejects unknown fields in it. It fixes
nothing about the words used inside an acceptance criterion — the kinds of
evidence and their attributes are supplied entirely by whoever writes the plan,
and are validated by nobody.

That produces a specific, silent defect. A criterion naming one spelling and an
evidence record naming another leaves the criterion *valid* and permanently
unsatisfied. Readiness reports the step as waiting, forever, in a way
indistinguishable from work that simply is not done.

The obvious remedy is to make the vocabulary declared, so criterion and evidence
are checked against one definition.

## Evidence and Argument

Two findings, each sufficient on its own, argue against declaring it.

**A declaration would put the vocabulary inside plan identity.** Identity is a
hash of the whole body, so a vocabulary carried in a version means widening an
enum or adding an attribute mints a substantive new version of *every plan that
uses it* — with no change to intent whatsoever. A system whose central claim is
that a version means intent changed would then manufacture versions nobody
authored. Keeping the declaration outside the version instead makes it a
dangling reference under replication, since the catalog replicates as files with
no resolution root.

**No declaration can reach across versions anyway.** Evidence is evaluated
against the criterion at the current frontier, not against the version it was
recorded under; the recorded version is provenance, not evaluation scope.
Validation is therefore only ever possible at write time, and a narrowing
revision leaves previously-valid evidence inert under every later criterion — the
original symptom, now with a declaration certifying the evidence as fine.

Against that, the defect is fully catchable without any declaration. The
criteria are already in the version being written against, so a record can be
checked against them at write time using only what is in memory. The domain of
each attribute is derivable from the values the plan's own criteria bind — the
plan supplies its own enumeration without declaring one. The resulting
diagnostic is *better* than a declaration would give: not that a value is
ill-typed, but which criterion was meant and how it differs.

Two subtleties were found by prototyping the naive form and breaking it.
Negation is idiomatic — "no known failure" — so a record matching the inner term
of a negated criterion is not a mistake to correct but an outcome to report; only
positively-stated criteria may suggest a correction. And a near-match is not
evidence of a mistake, because a genuine alternative outcome differs from the
expected one by exactly as much as a misspelling does. Both are handled by
scoping to polarity and deriving the domain from the plan's own corpus, and the
residual — a legitimate value no criterion happens to mention — is irreducible
and is why this reports rather than refuses.

## Options

| Option | Tradeoffs |
| --- | --- |
| Undeclared, with a write-time cross-check | Catches the defect with a better diagnostic, no dependency, no schema, no effect on identity; cannot see values no criterion mentions, so it warns rather than refuses |
| Declared, carried inside the version | Exact validation and static detection of unsatisfiable criteria; puts vocabulary inside identity, so a vocabulary change mints versions nobody authored |
| Declared, carried outside the version | Identity untouched; becomes a dangling reference under replication, and a fix cannot reach what is already committed |
| Undeclared, no checking at all | Nothing to build; the defect stays silent and indistinguishable from unfinished work |

## Decision

The evidence vocabulary remains undeclared. Compass does not define, validate, or
adjudicate what evidence means.

When a record is written, it is cross-checked against the acceptance criteria it
could contribute to. A value outside the domain those criteria establish is
reported, naming the criterion it most likely intended. This reports; it never
refuses, because the derivable domain is necessarily incomplete.

A record matching a negated criterion is reported as consequential rather than
mistaken: it will withdraw acceptance rather than grant it.

An attribute no criterion mentions remains accepted in silence. Matching is a
subset relation by design, and evidence may legitimately carry more context than
any criterion demands.

Separately, a criterion that **contradicts itself** is detected when it is
written. This is a strictly smaller claim than deciding whether a criterion can
be satisfied — which is undecidable here, since satisfaction may depend on
whether a named actor ever acts — and being smaller, it is a claim that can
always be kept.

## Consequences

- Compass stays domain-neutral. Nothing in it names a kind of work, which is why
  a plan for writing or research is expressible on the same terms as one for
  software.
- The defect is caught where it is made, with a diagnostic that names the
  criterion rather than the type.
- Some misspellings are missed — those whose value is plausible for a criterion
  that does not exist. This is the accepted ceiling.
- Compass records claims and does not certify their meaning, consistent with
  evidence being a claim whose author is recorded rather than adjudicated.
- A narrowing revision can strand previously-recorded evidence. Nothing here
  prevents that, and reporting it is a separate concern from validating a write.
