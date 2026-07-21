# Evidence is a claim, and predicates bind recorded fields

Status: accepted

## Context

Acceptance is evaluated against recorded evidence. Nothing constrains who may
record evidence, so a predicate is satisfied by whoever writes attributes that
match it.

In engineering work this is easy to overlook, because evidence like
`test(status=pass)` is typically emitted by a machine and reads as attested. In
work with no automatic evidence source — an editorial review, a sign-off, an
observation — the same predicate is satisfied by a person typing the attributes
directly, and the difference is invisible in the model.

## Evidence and Argument

An evidence event carries two kinds of value, and the model was treating them as
one. `actor` is **recorded**: Compass writes it from the acting identity, and
the writer does not choose it. `@attrs` are **claimed**: the writer chooses them
entirely.

Predicate evaluation matched only `@attrs`. An acceptance criterion of
`review(verdict=approved, actor=editor)` was therefore satisfied by an event
whose recorded actor was someone else entirely, because the writer supplied
`actor=editor` as an attribute:

```text
@event ev_…
actor = writer          ← recorded truthfully
@attrs
actor = editor          ← claimed, and this is what matched
verdict = approved
```

That is not a weak guarantee, it is an inverted one: the field that could be
trusted was ignored in favour of the field that could not, and the resulting
acceptance is indistinguishable from a genuine one.

Requiring attestation was considered and rejected. Compass cannot verify that a
test run really happened or that a reviewer really reviewed; a mechanism
implying otherwise would promise something it cannot keep, and CMP-T02 already
states that nothing here is a security boundary.

The available guarantee is narrower and honest: evidence is a **claim**, and the
claim's author is recorded rather than asserted. A reader can then see who
claimed what, which is the same basis on which a Rationale is trusted. That is
sufficient precisely because it does not pretend to be more.

## Options

| Option | Tradeoffs |
| --- | --- |
| Predicates bind recorded fields; attributes cannot shadow them | Restores the one trustworthy field; no new concepts, and the guarantee matches what Compass can actually observe |
| Predicates match attributes only | Status quo; any acceptance criterion is satisfiable by anyone, including criteria written specifically to require a named approver |
| Require attested evidence for some predicates | Stronger where attestation exists; needs a verification Compass cannot perform, so it implies a guarantee it cannot keep |
| Restrict who may record evidence | Appears to solve it; relocates the problem to identity, which is equally unverifiable in a local-first tool with no authority |

## Decision

Evidence is a claim. Compass records who made it and does not adjudicate whether
it is true.

A predicate term naming a recorded field binds that recorded field, never an
attribute of the same name. Recorded field names are reserved: an evidence event
may not carry an attribute that shadows one, and an attempt to write such an
attribute is rejected rather than silently ignored.

Acceptance therefore states who must claim something, and that requirement holds
against the identity Compass observed rather than one the writer supplied.

## Consequences

- `accept = review(verdict=approved, actor=editor)` is satisfied only by an
  event Compass recorded as authored by `editor`.
- Trust rests on identity being meaningful in the surrounding environment. Where
  an identity is shared or defaulted, the guarantee is correspondingly weak —
  and it is weak *visibly*, because the recorded actor is shown.
- Compass never claims evidence is true, only that it was claimed and by whom.
  This is the same standing a Rationale has.
- Shadowing attributes are refused at write time, so a plan cannot be authored
  against a criterion that could never have meant what it appeared to.
