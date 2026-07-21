# Spec: Artifacts

Realizes [requirements.md](./requirements.md). The logical model it stores is in
[01-data-model](../01-data-model/spec.md) and is not restated here.

## Layout

```text
catalog/plans/<plan>/versions/<seq>-<hash>.ts       immutable, mode 0444
catalog/plans/<plan>/events/<ts>-<id>.<ext>         append-only
```

`seq` is a reading aid, not a key. Divergent versions may share one, and after a
reconciliation of unequal lineages it follows the longest predecessor. Nothing
resolves on `seq`; the hash is the identity.

A version file is a module. A revision refers to its predecessor by referencing
that file, so a version's name appears inside its successors — which is why the
name is fixed at commit and never recomputed.

Progress events are not modules. They are inert data, read without being run;
their concrete form is unresolved (DQ02).

## Identity and integrity

A version is stored exactly as it was authored, and its identity is a hash of
those bytes. There is no rendering step and no canonical form, so there is no
second artifact to diverge from the first and nothing an author reads that
differs from what the catalog keeps.

Whitespace is therefore identity-bearing. That is the mechanism, not its price:
a committed version is immutable, nobody reformats one, and an identity that
moves when the bytes move is exactly what makes an alteration of committed
source visible. A normalization that let identity survive reformatting would
also let it survive tampering.

Because each version references its predecessors by name, and each name is a
hash of content, altering any version changes its identity and breaks every
descendant's reference — so damage is detectable by walking, without a separate
manifest.

A Step's declared name is inside the hashed bytes for free, since the bytes are
the authored source. Renaming a declaration in a committed version therefore
changes that version's identity, and re-identifying a Step cannot be done
quietly. This is worth stating rather than leaving implicit: it is a property of
storing the source, and it would be lost by any future stored form that is
derived rather than authored.

Mode `0444` converts an agent's in-place edit into a visible permission error.
It is not enforcement: the writer owns the filesystem and can recompute any
derived value. The lineage gives corruption-evidence; permissions give accident
resistance. Neither substitutes for the other, and neither is a security
boundary. The one boundary here that is load-bearing against a hostile writer is
elsewhere: it is the environment a version is evaluated in (CMP-R12), which
constrains what a stored module can *do* rather than who can write one.

## Admission

A file becomes state when it sits in an expected location and its content
matches the identity in its name. Files that merely parse are ignored; files
whose content contradicts their name are rejected with an error naming both.

Admission looks at bytes and nothing else. It does not evaluate the module, and
in particular does not require that what the module references be present:
replication delivers files in no useful order, so a version arrives before its
predecessor about as often as after. An admission rule that ran the module would
make delivery order decide what became state, and would reject a perfectly good
version for a gap that closes on the next sync.

This strictness is forced by replication rather than chosen for tidiness. Under
no-delete semantics, a wrongly admitted file cannot be removed — it returns on
the next sync — so admission is the only point at which the catalog can be kept
clean.

Files arriving by replication are admitted on the same terms as local ones. File
modes are not assumed to survive the wire, and neither is anything about the
machine that sent them: an admitted version is a program this machine will run
when someone reads the Plan.

## Head, divergence, orphans on disk

Head is computed by walking. Nothing on disk records it, so concurrent writers
have nothing to contend on.

```text
versions/003-a1b2….ts     predecessor = 002-…
versions/003-c3d4….ts     predecessor = 002-…            ← divergence: shared predecessor
versions/004-e5f6….ts     predecessors = [a1b2…, c3d4…]  ← reconciliation
```

An orphan is a version referencing a predecessor no local file provides. It is
reported as incomplete state, not as divergence, and never offered
reconciliation as its repair.

An orphan at the frontier is also unreadable. A version that cannot resolve what
it references cannot be evaluated, so the Plan reads as Unresolved: no goal, no
Steps, no readiness, rather than a Plan with a short lineage. Both conditions
are repaired by waiting and they are reported separately, because one shows the
Plan and the other shows nothing.

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

Repair is more urgent than a broken link would suggest. A version that cannot be
resolved cannot be evaluated, and every later version references it, so damage
mid-lineage does not leave a Plan partially readable — it leaves it unreadable
to the tip. The repair version, continuing from the last intact predecessor, is
what restores a readable frontier.

The same mechanism is the only response to content that should not have been
written. A credential recorded in a Step description cannot be recalled from
replicas — Compass marks it inert and flags it, and cannot make it absent. The
catalog is append-only in the strongest sense: do not write what must later be
unwritten.

## Discovery

The catalog is walked and files are admitted per the rules above.

Path segments may supply values the content omits. They do not override
identity: where content and location disagree about *which* Plan a version
belongs to, or where content disagrees with the hash in its name, the file is
rejected rather than reinterpreted. The borrowed catalog convention resolves
such disagreements in favour of content; Compass departs from it here, because
a misfiled file admitted under no-delete replication is permanent.

## The index

Reading a Plan evaluates it and everything it references, so a question spanning
many Plans is many module-graph evaluations. The index stands between reads and
repeated evaluation: it holds the evaluated form of a version, keyed by that
version's content hash.

Keying it that way removes the question every cache raises. There is no
staleness to detect, because content that changed is a different identity and
therefore a different key; the old entry is not wrong, it describes a version
that still exists and still evaluates that way. A lookup hits or misses. There
is nothing to invalidate and nothing to remember at write time.

The same property settles authority. An entry is a memo of a pure function over
immutable input, so it can say nothing the committed modules do not already
determine. Deleting the index is always safe, a corrupt entry is repaired by
discarding it, and the index cannot become a second account of what a Plan says
even by accident.

It is machine-local and never replicated: it is derivable from what does
replicate, and replicating it would ship a value the receiver can compute while
creating something two machines could disagree about.

A miss is ordinary and costs an evaluation. Entries accumulate for versions no
longer at the frontier, and reclaiming them is ordinary eviction rather than a
question about the model, since nothing is lost by evicting.

An Unresolved Plan has no entry, because it cannot be evaluated. Reads of it
fail on every attempt until what it references arrives.
