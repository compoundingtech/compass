# DELTA-001: Convergence is always unknown

Status: open

## Divergence

The convergence guarantee is structurally correct and currently vacuous. Every
command reports `unknown (no sync configured)`, and will do so regardless of
what the replication substrate is actually doing.

## VRS

[requirements.md](../requirements.md) CMP.INT-R06 requires the
converged-or-arriving signal to come from the replication mechanism's own state.
CMP.INT-R07 requires Compass to confirm the declared replication carries
no-delete semantics. Root CMP-R05 makes observable convergence a property of the
system, and it is the correction that
[decision 0002](../../.decisions/0002-plans-are-immutable-versions-with-a-derived-head.md)
Amendment 1 introduced in response to the derived head removing the only
completeness signal.

## Implementation

Compass reports convergence on every command and reports it honestly as unknown.
No declaration to a replication mechanism is made, no substrate is queried, and
no policy is verified. The honest report is the whole implementation.

## Direction

update implementation

## Resolution Signal

Compass declares its catalog to the replication substrate, queries it for
convergence, and reports settled, arriving, or unknown from that answer rather
than by default. Policy verification reports a delete-propagating configuration
as a misconfiguration. Until then the reported `unknown` is accurate, and the
guarantee that reads depend on remains unrealized.

## Note

The declaration must cover the catalog recursively (CMP.INT-R04). A path filter
that does not cross directory separators propagates nothing for this layout
while reporting itself healthy — a silent failure already observed in an
adjacent consumer of the same substrate.
