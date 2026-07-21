# Requirements: CLI

> **Role.** The operator interface — how an agent or a human reads and changes a
> Plan from a terminal. It renders the
> [Plan Surface](../03-surface/requirements.md) and owns no semantics: any
> behaviour it appears to add is a defect in this node or a missing requirement
> elsewhere. Each requirement `refines:` a `CMP-R*`.

## Requirements

- **CMP.CLI-R01 The CLI adds no semantics.** Every command is a rendering of a
  Plan Surface operation. A command that computes, infers, or decides something
  the surface does not is a second authority in disguise.
  _refines: CMP-R01._

- **CMP.CLI-R02 Both audiences, one truth.** Every command that renders for a
  human also renders for a program, carrying the same fields. The two must not
  disagree, and neither may omit something the other reports.
  _refines: CMP-R01._

- **CMP.CLI-R03 Convergence is always visible.** Every command reporting Plan
  state states whether what it read was settled, still arriving, or unknown. A
  reader must never have to ask separately whether an answer can be trusted.
  _refines: CMP-R05._

- **CMP.CLI-R04 Divergence renders, never errors.** Any command reporting head
  handles more than one head member, labels them, and shows their authors and
  reasons. Divergence is a normal state and must read as one — an error message
  would teach operators to treat it as breakage. _refines: CMP-R04._

- **CMP.CLI-R05 Answers carry their reasons.** Readiness renders the
  unsatisfied dependencies and gates alongside its answer, and verification
  renders what is broken and where. An unexplained answer is not usable.
  _refines: CMP-R01, CMP-R07._

- **CMP.CLI-R06 Inspection is separated from authorship.** Commands that only
  read are distinguishable from commands that write permanent content, and the
  latter are never a side effect of the former. Under append-only storage a
  write cannot be undone, so it must never be incidental.
  _refines: CMP-R02, CMP-R07._

- **CMP.CLI-R07 The rationale is a first-class output.** Reading a Plan's
  history — its reasons in sequence — is a primary operation, not a flag on
  another command. It is the output the system exists to produce.
  _refines: CMP-R03._

- **CMP.CLI-R08 Identity is reported honestly.** The CLI reports its own version
  and build provenance, so an operator can tell which build produced an output.
  _refines: CMP-R07._

- **CMP.CLI-R09 Never ask for a value that has one valid answer.** If a value
  can be derived from state the CLI can already reach, the CLI derives it rather
  than accepting it. A caller naming a Plan must not also be asked for its
  sequence or its file layout. Every such argument is a chance to supply the one
  wrong answer, and it is a chance the tool created. A revision's predecessor is
  neither derived nor asked for: it is written in the revision itself, which is
  what makes a retry repeat rather than drift. _refines: CMP-R10._

- **CMP.CLI-R10 Authoring is a module, not an invocation.** Intent is written as
  code and committed, never assembled from arguments. A caller names what to
  commit; everything about *what* is intended lives in the module, where a
  dependency is a reference the language checks rather than a string the CLI
  accepts. _refines: CMP-R02, CMP-R10._

- **CMP.CLI-R11 A rejected commit costs no work.** Refusal leaves what was
  authored exactly as authored, byte for byte, and says where it is. Nothing is
  ever written back into authored content, so there is no path by which a
  refusal could modify it — this holds by construction rather than by
  discipline. _refines: CMP-R07._

- **CMP.CLI-R12 Committing what already landed is a no-op, not a failure.**
  Submitting content identical to a version already committed reports what is
  there and writes nothing. This is the ordinary shape of a retry, and a retry
  that behaved correctly must not be told it failed. It is distinct from
  authoring *new* content that changes no Step and no goal, which is refused
  (CMP.DM-R07b) — the first repeats an act, the second asserts a revision that
  did not revise. The two must not report alike. _refines: CMP-R02._

- **CMP.CLI-R13 What a commit would do is inspectable before it happens.** The
  structural change authored content makes against its predecessors is
  reportable without committing, since under no-delete replication a commit
  cannot be walked back. Establishing it means evaluating the content, so this
  is a read that runs a program and is subject to the same bound as any other.
  _refines: CMP-R07, CMP-R13._

- **CMP.CLI-R14 A read that could not answer says which kind of failure it was.**
  A Plan that is absent, one that cannot be evaluated because something it
  references has not arrived, and one whose evaluation was stopped are rendered
  distinctly, each naming what is missing or which bound was reached. The
  operator's next action differs in each case — look elsewhere, wait, or stop
  trusting the Plan — so a single "could not read" would leave the choice to
  guesswork. _refines: CMP-R05, CMP-R07._
