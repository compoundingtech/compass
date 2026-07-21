# Spec: CLI

Realizes [requirements.md](./requirements.md). Every command maps to a
[Plan Surface](../03-surface/spec.md) operation; this document adds spelling and
presentation, not meaning.

## Command spelling

Spelling is unresolved and settles with the implementation. What is fixed is the
mapping: each command is one surface operation, and no command combines a read
with a write.

The operations an operator needs are: start a Plan; revise its intent; record
progress and evidence against a Step; ask what is ready; read the lineage and
its reasons; reconcile a divergence; verify integrity; repair damage; and report
build identity.

Starting must cost one command and produce something immediately workable
(CMP-R11). Intent being code raises the floor here rather than lowering it, so
the first command produces the module rather than asking the operator to know
what one looks like. Nothing about starting may require reading documentation,
choosing an identifier, or configuring a location.

## Authoring and committing

Intent is written as a module and committed. The commit reads that module,
evaluates it, and stores it unchanged.

Two consequences shape the commands:

- **A refusal changes nothing.** There is no write-back into authored content,
  so a rejected commit leaves the file exactly as it was and the operator edits
  and retries. Nothing that refuses can also destroy.
- **A repeat is a repeat.** Committing content that already landed produces no
  second version, because identical content has identical identity. The command
  reports what is there and exits successfully; being told a correct retry
  failed would teach an operator to distrust the report.

Authoring *new* content that revises nothing is a different case and is refused
with a different message. One says "this is already committed"; the other says
"this changes nothing." Rendering them alike would hide which happened.

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
authors and reasons. Readiness renders per member. There is no time to render:
order comes from the lineage, and divergent siblings have none relative to each
other.

The presentation makes the disagreement legible — what each side intends and why
— because the operator's next action is to reconcile it, and reconciling
requires understanding both sides. Nothing is ranked, defaulted, or hidden
behind a flag.

An orphan renders differently from a divergence, and says so: the repair for one
is to wait, and for the other to author a reconciliation. Rendering them alike
would invite the wrong action.

## Rendering a read that failed

Reading a Plan runs it, so it can fail in ways a lookup cannot, and each failure
names a different next action:

```text
not found      no such Plan here                → look elsewhere
unresolved     <what it references> has not arrived → wait
stopped        evaluation exceeded <which bound>    → do not wait; the Plan is at fault
```

The distinction is the whole point. Waiting for an unresolved Plan is correct
and waiting for a stopped one never ends, so a single "could not read" would be
worse than any of the three.

## Rendering reasons

Reading a Plan's lineage renders its reasons in order — the sequence of
rationales that produced the current intent. This is the system's primary
output, so it is a first-class command rather than a verbose mode.

## Errors

A rejected mutation reports what was rejected and why, and states plainly that
nothing was recorded. Ambiguity about whether a failed write partially applied
is the worst outcome an append-only store can present.
