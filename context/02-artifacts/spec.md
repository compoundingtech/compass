# Spec: Artifacts

Realizes [requirements.md](./requirements.md). The logical model it stores is in
[01-data-model](../01-data-model/spec.md) and is not restated here.

## Layout

```text
catalog/plans/<plan>/versions/<seq>-<hash>.<ext>    immutable, mode 0444
catalog/plans/<plan>/events/<ts>-<id>.<ext>         append-only
```

`seq` is a reading aid, not a key. Divergent versions may share one, and after a
reconciliation of unequal lineages it follows the longest predecessor. Nothing
resolves on `seq`; the hash is the identity.

`<ext>` and the serialization syntax are unresolved (DQ02), as is whether the
hash covers raw bytes or a canonical form.

## Identity and integrity

A version's identity is a hash of its content, and its filename embeds a prefix
of that hash. Because each version records its predecessors by hash, altering
any version changes its identity and breaks every descendant's link — so damage
is detectable by walking, without a separate manifest.

Mode `0444` converts an agent's in-place edit into a visible permission error.
It is not enforcement: the writer owns the filesystem and can recompute any
derived value. The lineage gives corruption-evidence; permissions give accident
resistance. Neither substitutes for the other, and neither is a security
boundary.

## Admission

A file becomes state when it sits in an expected location and its content
matches the identity in its name. Files that merely parse are ignored; files
whose content contradicts their name are rejected with an error naming both.

This strictness is forced by replication rather than chosen for tidiness. Under
no-delete semantics, a wrongly admitted file cannot be removed — it returns on
the next sync — so admission is the only point at which the catalog can be kept
clean.

Files arriving by replication are admitted on the same terms as local ones. File
modes are not assumed to survive the wire.

## Head, divergence, orphans on disk

Head is computed by walking. Nothing on disk records it, so concurrent writers
have nothing to contend on.

```text
versions/003-a1b2….ext   parent = 002-…
versions/003-c3d4….ext   parent = 002-…       ← divergence: shared predecessor
versions/004-e5f6….ext   parent = [a1b2…, c3d4…]   ← reconciliation
```

An orphan is a version naming a predecessor no local file provides. It is
reported as incomplete state, not as divergence, and never offered
reconciliation as its repair.

Because completeness cannot be read from the catalog — no file states how many
versions a Plan should have — a shorter lineage and an incompletely replicated
one are indistinguishable from the files alone. Convergence therefore comes from
the replication mechanism; see
[05-integrations](../05-integrations/spec.md).

## Repair

Damage is detected by verification and repaired by authorship. Compass never
edits or deletes a damaged version:

- deleting returns the file on the next sync,
- rewriting changes its identity and dangles every descendant,
- rewriting the descendants cascades to the tip and forks the lineage in the act
  of repairing it.

Instead a new version records the damage, states what is known of the lost
intent, and continues from the last intact predecessor. The surviving record of
reasons is preserved, which is the property worth protecting; the damaged bytes
remain on disk and are excluded from interpretation.

The same mechanism is the only response to content that should not have been
written. A credential recorded in a Step description cannot be recalled from
replicas — Compass marks it inert and flags it, and cannot make it absent. The
catalog is append-only in the strongest sense: do not write what must later be
unwritten.

## Discovery

The catalog is walked and files are admitted per the rules above. Path segments
may supply defaults; content wins on disagreement.

Discovery parses every candidate file to classify it, which bounds catalog size
to what can be scanned in interactive time (CMP-A03). The threshold is
unmeasured and shares a root with compaction (DQ06).
