# DELTA-001: Repair has no command

Status: open

## Divergence

Damage can be detected but not repaired through a dedicated operation.

## VRS

[requirements.md](../requirements.md) CMP.FS-R11 requires that recovery from
damage proceed by authoring new content that records the damage and continues
from the last intact parent. [04-cli/spec.md](../../04-cli/spec.md) states
that verification and repair are separate commands, and gives the reason:
verification is safe to run anywhere, while repair authors permanent content
that replication makes irreversible, so collapsing them would put the
irreversible operation one keystroke from the safe one.

## Implementation

`compass verify` is read-only and reports rejected files, orphans, and a
frontier that will not evaluate. `compass repair` now exists as a *distinct*
command: it re-runs verification and **refuses when nothing is wrong** (so the
irreversible operation is never one keystroke from the safe one), identifies the
last intact parent, and lists which versions are unverifiable.

What it does not yet do is author the damage-recording version itself: it
scaffolds the `prior.revise({...})` continuation from the last intact
parent and directs the operator to `compass commit` it. The separation the
spec relies on is enforced (verify is read-only; repair is its own command that
refuses on a clean catalog); the authoring is guided rather than performed.

## Direction

update implementation

## Resolution Signal

A distinct command authors a damage-recording version: it identifies the last
intact parent, requires a Rationale, records which versions are
unverifiable and why, and refuses to run when verification reports nothing
wrong. Verification remains read-only.
