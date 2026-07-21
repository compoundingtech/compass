# Requirements: Data Model

> **Role.** What a Plan *is*, independent of how it is stored. This node defines
> lineage, identity, divergence, progress, acceptance, and readiness as logical
> structure; [02-artifacts](../02-artifacts/requirements.md) realizes it as
> files. Each requirement `refines:` a `CMP-R*`.

## Requirements

### Lineage

- **CMP.DM-R01 A Plan is a lineage of versions.** Each version records its
  predecessors, so the history of intent is reconstructible from the versions
  alone. _refines: CMP-R02._

- **CMP.DM-R02 Every version carries a Rationale.** The reason for the revision
  is part of the version, not metadata about it. _refines: CMP-R03._

- **CMP.DM-R03 Head is derived.** The frontier of a Plan is computed from the
  lineage rather than recorded, so no stored value can disagree with the
  versions. Head is a set; ordinarily it has one member.
  _refines: CMP-R02, CMP-R04._

- **CMP.DM-R04 Divergence is a state, not an error.** Versions sharing a
  predecessor are both valid and both reported. _refines: CMP-R04._

- **CMP.DM-R05 Divergence resolves by authorship.** Reconciliation is an
  ordinary version naming every predecessor it reconciles, with its own
  Rationale. Nothing reconciles automatically, and a reconciliation may itself
  diverge. _refines: CMP-R03, CMP-R04._

- **CMP.DM-R05a Open divergence is distinguishable from settled.** A divergence
  is open until its sides share a descendant. Because a divergence is a
  permanent feature of the lineage, a system that cannot tell open from settled
  reports every past disagreement as outstanding forever — and a report that is
  always on carries no information. Only open divergence may prompt action.
  _refines: CMP-R04._

- **CMP.DM-R05b Retirement may strand dependents.** Retiring a Step that others
  depend on leaves them permanently unsatisfiable, because a retired Step never
  becomes accepted. This is a consequence of retirement, not a defect, but it
  must be visible at the moment of retirement rather than discovered later
  through readiness that never advances. _refines: CMP-R01._

- **CMP.DM-R06 An absent predecessor is not divergence.** A version whose
  predecessor is unknown must be distinguished from one that disagrees. The
  first ordinarily means state is still arriving; treating it as the second
  writes permanent intent to resolve a transient condition.
  _refines: CMP-R04, CMP-R05._

- **CMP.DM-R06a An unreadable Plan is distinguished from an incomplete one.** A
  Plan that cannot be evaluated because something it references is absent must
  be reported as unresolved, distinctly from a version whose predecessor is
  merely missing. The two look alike and are not: an incomplete lineage still
  answers what the Plan says, while an unresolved Plan answers nothing at all.
  Reporting the second as the first invites waiting for a repair that has
  already arrived, or reconciling to paper over an absence.
  _refines: CMP-R05, CMP-R07._

- **CMP.DM-R07 Versions are attributable.** Each version records its author.
  Reconciling divergence requires knowing who wrote each side. Order is not
  recorded but derived from the lineage: a counter cannot order divergent
  siblings, since neither observed the other, and where one version does precede
  another the lineage already says so. _refines: CMP-R03, CMP-R04, CMP-R10._

- **CMP.DM-R07a Repeating a mutation does not repeat its effect.** A mutation
  applied twice produces one version. This must follow from the data — an
  identical mutation yields an identical version, therefore the same identity —
  rather than from a token a caller supplies and could supply wrongly. It holds
  only because a revision states its predecessor as part of its own content: a
  base that were read at the moment of application would have moved by the time
  a retry arrived, and the retry would differ from the attempt it repeats.
  _refines: CMP-R02, CMP-R10._

- **CMP.DM-R07b A revision must change something.** A version that alters no
  Step and no goal is refused. Without this, a retry whose rationale was
  reworded produces a permanent duplicate that no derived value can detect,
  because the content genuinely differs. This is distinct from re-applying the
  identical revision, which is one version by CMP.DM-R07a and is not a failure.
  The cost is that a deliberate non-change cannot be recorded.
  _refines: CMP-R02, CMP-R03._

- **CMP.DM-R07c A revision carries its predecessor forward.** A revision is
  expressed against the version before it and can edit a Step, add one, or
  retire one. It has no way to remove one. Dropping a Step is therefore not
  something Compass detects and refuses but something a revision cannot express
  — which is the stronger property, because a refusal only catches what it was
  built to look for and a plan that restates itself wholesale gives it a great
  deal to look for. Removal is retirement, and a retired Step stays in the
  Plan. _refines: CMP-R02._

### Identity

- **CMP.DM-R08 A Step's identity is the name it is declared under.** Identity is
  the declared name qualified by its Plan: not minted, not random, and not
  derived from the Step's content. It is fixed where the Step is first declared
  and carried forward by revision, so a Step that outlives many versions keeps
  the identity it was born with, and rewording the work does not disturb it.
  _refines: CMP-R02, CMP-R10._

- **CMP.DM-R09 A dependency names a declaration, never a spelling.** One Step
  depending on another must reference the declaration itself, so a dependency
  cannot be invented, mistyped, or left dangling — a reference that does not
  resolve fails where it is written. A reference an author types from memory can
  be plausible and wrong, which is the worst available outcome: it parses,
  validates, and means something nobody intended. _refines: CMP-R01, CMP-R10._

- **CMP.DM-R09a An unnamed Step has no identity and is refused.** A Step that is
  not a named declaration cannot be referenced, carried forward, or retired, so
  it is rejected rather than admitted with an identity Compass invents for it.
  This constrains how intent may be written, which is the point.
  _refines: CMP-R02, CMP-R10._

- **CMP.DM-R10 Retired names are final.** A name is never reused after the Step
  it identified is retired, because reuse would silently attach one Step's
  history to another. Identity changes — split, merge, replacement — are stated
  explicitly rather than inherited silently. _refines: CMP-R02._

- **CMP.DM-R10a Re-identification is detectable.** Renaming a declaration in
  already-committed content changes what that content says about identity, and
  that change must be visible rather than silent. This holds only while the
  declared identity is inside what identity is computed over; see
  [CMP.FS-R02a and CMP.FS-R03](../02-artifacts/requirements.md). It is
  load-bearing rather than
  incidental, because without it a committed Step can be re-identified while
  every hash in the lineage stays constant. _refines: CMP-R02, CMP-R07._

### Progress and acceptance

- **CMP.DM-R11 Progress is append-only.** Progress records never alter intent
  and never create a version. Correction is a further record, never an edit.
  _refines: CMP-R02._

- **CMP.DM-R11a Progress names one version, deterministically.** A record cites
  the version it was observed against. Under divergence there may be several
  head members carrying the same Step, so the choice must be deterministic and
  disclosed rather than arbitrary. A record whose cited version differs between
  two machines observing identical state would make progress itself a source of
  divergence. _refines: CMP-R04._

- **CMP.DM-R12 Acceptance is evaluable.** A Step's acceptance criterion is
  expressible in a form Compass can evaluate against recorded evidence. Prose
  may accompany it but is not the criterion, because readiness cannot fold over
  prose. _refines: CMP-R01._

- **CMP.DM-R13 Only Compass judges completion.** An external record never
  completes a Step; completion follows from the Step's own acceptance criterion.
  _refines: CMP-R01._

- **CMP.DM-R13a Evidence is a claim with a recorded author.** Compass records
  who made a claim and does not adjudicate whether it is true. It must not imply
  attestation it cannot perform. _refines: CMP-R01, CMP-T02._

- **CMP.DM-R13b Predicates bind recorded fields, never claimed ones.** A
  predicate term naming a recorded field of an evidence record binds that
  recorded field. Claimed attributes must not shadow recorded field names, and
  an attempt to write such an attribute is refused. Without this, a criterion
  requiring a named approver is satisfiable by anyone willing to write that name
  — an acceptance indistinguishable from a genuine one.
  _refines: CMP-R01, CMP-R10._

- **CMP.DM-R13c The evidence vocabulary is not declared.** Compass fixes the
  structure of a Plan and fixes nothing about the words used inside an
  acceptance criterion. It does not define, validate, or adjudicate what a kind
  of evidence means, because a vocabulary carried inside a version would make
  widening it mint a new version of every Plan that used it — versions nobody
  authored, in a system whose central claim is that a version means intent
  changed. _refines: CMP-R01, CMP-R02._

- **CMP.DM-R13d A record that can never match is reported when it is written.**
  A criterion naming one spelling and a record naming another leaves the
  criterion valid and permanently unsatisfiable, and readiness then reports the
  Step as waiting forever, indistinguishably from work that is simply not done.
  A record must therefore be checked, as it is written, against the criteria it
  could contribute to, and a value outside the domain those criteria establish
  reported with the criterion it most likely intended. This reports and never
  refuses: the domain is derived from what the Plan's own criteria mention, so
  it is necessarily incomplete, and a legitimate value no criterion happens to
  name must not be blocked. A record matching a negated criterion is reported as
  consequential rather than mistaken — it withdraws acceptance rather than
  granting it. _refines: CMP-R01, CMP-R07._

- **CMP.DM-R13e A self-contradicting criterion is refused.** A criterion that
  cannot be satisfied by any record, because it contradicts itself, is rejected
  when it is written. This is deliberately smaller than deciding whether a
  criterion *will* be satisfied, which is not decidable here — satisfaction can
  depend on whether a named actor ever acts. Being smaller, it is a claim that
  can always be kept. _refines: CMP-R01._

### Readiness

- **CMP.DM-R14 Readiness is derived.** What can be worked on now follows from
  the Step graph at head, accepted progress, and gates. It is part of the model,
  not a projection over it. _refines: CMP-R01._

- **CMP.DM-R15 Readiness explains itself.** Every answer names the unsatisfied
  dependencies and gates. An answer that cannot say why is neither trustworthy
  nor debuggable, and this constrains what an acceptance criterion may express.
  _refines: CMP-R01._

- **CMP.DM-R16 Readiness is defined under divergence.** With more than one head
  member, readiness is reported per member and labelled. It never selects a side
  and never merges graphs, which would assert intent nobody authored. The normal
  state of the system must not be an undefined state of its primary query.
  _refines: CMP-R04._
