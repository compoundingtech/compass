# Spec: CLI

Realizes [requirements.md](./requirements.md). Every command maps to a
[Plan Surface](../03-surface/spec.md) operation; this document adds spelling and
presentation, not meaning.

## Command spelling

Spelling is unresolved and settles with the implementation. What is fixed is the
mapping: each command is one surface operation, and no command combines a read
with a write.

The operations an operator needs are: create a Plan; revise its intent; record
progress and evidence against a Step; ask what is ready; read the lineage and
its reasons; reconcile a divergence; verify integrity; repair damage; and report
build identity.

Verification and repair are separate commands. Verification is safe to run
anywhere at any time; repair authors permanent content that replication makes
irreversible. Collapsing them into one command with a flag would make the
irreversible operation one keystroke from the safe one.

## Output

Every command renders for a human by default and for a program on request, from
the same underlying values. A field present in one is present in the other.

Machine output is a contract: consumers depend on it, so a field is added
compatibly and removed only deliberately.

## Reporting convergence

Every command that reports Plan state reports whether that state was settled,
still arriving, or unknown — including when the answer is "unknown," which is
the honest report when no replication mechanism is configured or when it cannot
be reached.

This is stated positively rather than as a warning on the exceptional path. An
operator who has to notice the *absence* of a warning to know an answer is
trustworthy will eventually not notice.

## Rendering divergence

A Plan with several head members renders all of them, labelled, with their
authors, logical times, and reasons. Readiness renders per member.

The presentation makes the disagreement legible — what each side intends and why
— because the operator's next action is to reconcile it, and reconciling
requires understanding both sides. Nothing is ranked, defaulted, or hidden
behind a flag.

An orphan renders differently from a divergence, and says so: the repair for one
is to wait, and for the other to author a reconciliation. Rendering them alike
would invite the wrong action.

## Rendering reasons

Reading a Plan's lineage renders its reasons in order — the sequence of
rationales that produced the current intent. This is the system's primary
output, so it is a first-class command rather than a verbose mode.

## Errors

A rejected mutation reports what was rejected and why, and states plainly that
nothing was recorded. Ambiguity about whether a failed write partially applied
is the worst outcome an append-only store can present.
