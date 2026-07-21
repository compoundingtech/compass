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
  predecessor, its sequence, its author, or where its draft lives. Every such
  argument is a chance to supply the one wrong answer, and it is a chance the
  tool created. _refines: CMP-R10._

- **CMP.CLI-R10 Authoring is a document, not an invocation.** Intent is composed
  in a draft and committed, never assembled from arguments. A caller names the
  Plan; everything about *what* is intended lives in the document.
  _refines: CMP-R02, CMP-R10._

- **CMP.CLI-R11 A rejected commit costs no work.** Validation failure leaves the
  draft exactly as authored, and says where it is. Under append-only storage a
  refusal is cheap and lost authoring is not, so nothing that refuses may also
  destroy. _refines: CMP-R07._

- **CMP.CLI-R12 An unchanged draft is a no-op, not a failure.** Committing a
  draft that changes nothing reports that plainly and writes nothing. It is the
  ordinary shape of a retry, and a retry that behaved correctly must not be told
  it failed. _refines: CMP-R02._

- **CMP.CLI-R13 What a commit would do is inspectable before it happens.** The
  structural change a draft makes against its predecessors is reportable without
  committing, since under no-delete replication a commit cannot be walked back.
  _refines: CMP-R07._
