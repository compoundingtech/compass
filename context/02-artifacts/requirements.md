# Requirements: Artifacts

> **Role.** One realization of the [data model](../01-data-model/requirements.md)
> as files on disk: layout, identity, admission, integrity, and repair. The
> model does not depend on this realization; this node depends on the model.
> Each requirement `refines:` a `CMP-R*` or a `CMP.DM-R*`.

## Requirements

### Layout and identity

- **CMP.FS-R01 The catalog root is configuration.** No storage location is
  compiled in. _refines: CMP-R06._

- **CMP.FS-R02 Version identity is the content.** A version's stored identity is
  a hash of its content, so identical content is one version and any difference
  is another. This is what makes divergence survive replication instead of
  becoming a lost write. _refines: CMP.DM-R03, CMP.DM-R04._

- **CMP.FS-R03 Content hashes never address Steps.** Step references are minted
  values carried in the document, never derived from it.
  _refines: CMP.DM-R08._

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

- **CMP.FS-R07 Identity mismatch is rejection.** A file whose content does not
  match its identity is refused with a clear error, not accepted with a warning.
  The mismatch is precisely the corruption signal.
  _refines: CMP-R07, CMP.FS-R02._

- **CMP.FS-R08 Received files are untrusted.** Files arriving by replication are
  admitted on the same terms as local ones. No property conferred by the local
  filesystem may be assumed to have survived the wire.
  _refines: CMP-R07._

### Integrity and repair

- **CMP.FS-R09 Intent resists accidental edit.** Written intent is stored so
  that an in-place edit fails visibly rather than succeeding silently. This
  guards against error, not against a determined writer.
  _refines: CMP-R02, CMP-T02._

- **CMP.FS-R10 The lineage is checkable.** Compass verifies that every version
  matches its identity and that every recorded predecessor is present, and
  reports what is broken and where. _refines: CMP-R07._

- **CMP.FS-R11 Repair never rewrites history.** Recovery from damage proceeds by
  authoring new content that records the damage and continues from the last
  intact predecessor. Editing or deleting a damaged version cascades through
  every descendant, and deletion returns on the next sync.
  _refines: CMP-R07, CMP-R02._

- **CMP.FS-R12 Retirement is content.** Decommissioning is an authored flag.
  Deletion is never the mechanism. _refines: CMP-R02._

- **CMP.FS-R13 Unwritable content stays written.** Content that should never
  have been recorded cannot be recalled from replicas. Compass can mark it inert
  and flag it; it cannot make it absent. This is a property to design around,
  not a gap to close. _refines: CMP-T03._
