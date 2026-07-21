# DELTA-001: Repair has no command

Status: open

## Divergence

Damage can be detected but not repaired through a dedicated operation.

## VRS

[requirements.md](../requirements.md) CMP.FS-R11 requires that recovery from
damage proceed by authoring new content that records the damage and continues
from the last intact predecessor. [04-cli/spec.md](../../04-cli/spec.md) states
that verification and repair are separate commands, and gives the reason:
verification is safe to run anywhere, while repair authors permanent content
that replication makes irreversible, so collapsing them would put the
irreversible operation one keystroke from the safe one.

## Implementation

`compass verify` detects and reports rejected files and orphans. There is no
`compass repair`. The repair *path* exists — an operator can author a
damage-recording version with `compass revise` — but it is neither named,
guided, nor distinguished from ordinary revision, so nothing enforces the
separation the spec relies on.

## Direction

update implementation

## Resolution Signal

A distinct command authors a damage-recording version: it identifies the last
intact predecessor, requires a Rationale, records which versions are
unverifiable and why, and refuses to run when verification reports nothing
wrong. Verification remains read-only.
