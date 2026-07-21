# Requirements: Artifacts

> **Role.** One realization of the [data model](../01-data-model/requirements.md)
> as stored modules on disk: layout, identity, admission, integrity, repair, and
> the derived index that makes reading affordable. The model does not depend on
> this realization; this node depends on the model. Each requirement `refines:` a
> `CMP-R*` or a `CMP.DM-R*`.

## Requirements

### Layout and identity

- **CMP.FS-R01 The catalog root is configuration.** No storage location is
  compiled in. _refines: CMP-R06._

- **CMP.FS-R02 A version is stored exactly as authored.** What was written is
  what is kept. There is no second form, no canonical rendering, and no
  normalization step, so nothing can be stored that diverges from what its
  author read. A stored form distinct from the authored one is two artifacts
  that can disagree, with no way to establish which is right.
  _refines: CMP-R02._

- **CMP.FS-R02a Identity is the hash of the stored bytes.** A version is named by
  a hash of its own content with nothing excluded and nothing normalized away.
  Identical content is one version and any difference is another, which is what
  makes divergence survive replication instead of becoming a lost write.
  _refines: CMP.DM-R03, CMP.DM-R04._

- **CMP.FS-R02b Every byte is identity-bearing, deliberately.** Formatting
  changes a version's identity. This is not a fragility to be engineered away:
  a committed version is immutable, so reformatting one is not an edit but an
  alteration of something that was supposed to be fixed — and a hash over every
  byte is precisely what makes that alteration visible. Any normalization
  introduced to make identity survive reformatting would remove the detection
  along with the sensitivity. _refines: CMP-R07, CMP.DM-R10a._

- **CMP.FS-R03 Hashes address versions only.** Step identity is the name a Step
  is declared under and is never a hash of anything. Because a version is stored
  as authored, that name is inside the bytes identity is computed over, which is
  what makes a re-identification detectable rather than silent.
  _refines: CMP.DM-R08, CMP.DM-R10a._

- **CMP.FS-R04 Nothing records head.** No file names the current version. A
  stored head is a cell two writers can contend on, and union replication cannot
  merge one. _refines: CMP.DM-R03._

- **CMP.FS-R05 Paths are machine-agnostic.** Authored content refers to
  locations by variable rather than absolute path, so one catalog is valid on
  machines with different layouts. An unresolvable reference fails loudly;
  resolving it to an empty or plausible path is worse than failing.
  _refines: CMP-R06._

### Admission

- **CMP.FS-R06 Admission is deliberate.** A file becomes state only in an
  expected location and only when its content matches its recorded identity.
  Parsing successfully is not sufficient. Under no-delete replication a wrongly
  admitted file is permanent. _refines: CMP-R07._

- **CMP.FS-R06a Admission never evaluates.** Whether a file is admitted depends
  on its bytes alone, never on whether it can be run. A version arrives before
  the versions it references as often as after, so an admission that required
  evaluation would make the order replication happened to deliver files in
  decide what became state — and that order is not a property of the catalog.
  Evaluability is a question reads answer, not a condition of storage.
  _refines: CMP-R05, CMP.DM-R06a._

- **CMP.FS-R07 Identity mismatch is rejection.** A file whose content does not
  match its identity is refused with a clear error, not accepted with a warning.
  The mismatch is precisely the corruption signal.
  _refines: CMP-R07, CMP.FS-R02a._

- **CMP.FS-R08 Received files are untrusted.** Files arriving by replication are
  admitted on the same terms as local ones. No property conferred by the local
  filesystem may be assumed to have survived the wire, and no property of the
  sending machine may be assumed at all — a received version is a program that
  will be run here. _refines: CMP-R07, CMP-R12._

### Integrity and repair

- **CMP.FS-R09 Intent resists accidental edit.** Written intent is stored so
  that an in-place edit fails visibly rather than succeeding silently. This
  guards against error, not against a determined writer.
  _refines: CMP-R02, CMP-T02._

- **CMP.FS-R10 The lineage is checkable.** Compass verifies that every version
  matches its identity and that every reference it makes is present, and reports
  what is broken and where. _refines: CMP-R07._

- **CMP.FS-R11 Repair never rewrites history.** Recovery from damage proceeds by
  authoring new content that records the damage and continues from the last
  intact predecessor. Editing or deleting a damaged version cascades through
  every descendant, and deletion returns on the next sync.
  _refines: CMP-R07, CMP-R02._

- **CMP.FS-R11a Damage propagates to readability.** A damaged or absent version
  does not merely leave a gap in a lineage: every later version that references
  it cannot be evaluated, so the Plan stops answering questions rather than
  answering them incompletely. Verification and repair must be sized against
  that consequence, which is strictly larger than a broken link.
  _refines: CMP-R07, CMP.DM-R06a._

- **CMP.FS-R12 Retirement is content.** Decommissioning is authored and carried
  forward by every later version. Deletion is never the mechanism, and neither
  is omission. _refines: CMP-R02, CMP.DM-R07c._

- **CMP.FS-R13 Unwritable content stays written.** Content that should never
  have been recorded cannot be recalled from replicas. Compass can mark it inert
  and flag it; it cannot make it absent. This is a property to design around,
  not a gap to close. _refines: CMP-T03._

### The index

- **CMP.FS-R14 Reading is served from an index.** Answering a question about a
  Plan means evaluating it and everything it references, and a question spanning
  many Plans means many such evaluations. Something must stand between reads and
  repeated evaluation, or the catalog stops being readable in interactive time
  well before it stops being small. _refines: CMP-R14._

- **CMP.FS-R15 The index is keyed by content hash.** An entry is keyed by the
  identity of the version it describes, which removes the question of staleness
  rather than answering it: content that changed is a different identity and
  therefore a different key, so a lookup either hits or misses. Nothing needs to
  track freshness and no write path needs to remember to invalidate — the two
  places a cache is usually wrong. _refines: CMP-R14, CMP.FS-R02a._

- **CMP.FS-R16 The index carries no authority.** An entry is a memo of a pure
  function over immutable input, so it holds nothing the committed content does
  not already determine. Deleting the whole index must always be safe and must
  never lose information; a corrupt entry is repaired by discarding it; and no
  read may consult it for an answer it could not otherwise derive. Anything that
  can be thrown away cannot become load-bearing, and that is the property being
  bought. _refines: CMP-R14, CMP-R01._

- **CMP.FS-R17 The index is machine-local.** It is never replicated. A value
  derivable from what does replicate does not need shipping, and shipping it
  would make it something two machines could disagree about — the one failure
  this shape otherwise rules out. _refines: CMP-R06, CMP-R14._
