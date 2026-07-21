# Requirements: Plan Surface

> **Role.** The logical, transport-neutral way to read a Plan and to change one:
> the only sanctioned path by which intent becomes recorded. It owns no
> semantics of its own — the model is in
> [01-data-model](../01-data-model/requirements.md) — and no presentation, which
> belongs to [04-cli](../04-cli/requirements.md). Each requirement `refines:` a
> `CMP-R*`.

## Requirements

- **CMP.SURF-R01 One way in.** Every change to a Plan passes through the Plan
  Surface. No path may write Compass state around it, because a second writer
  would be a second authority. _refines: CMP-R01, CMP-R09._

- **CMP.SURF-R02 Transport-neutral.** The surface is defined in terms of
  references, mutations, queries, and results, and carries no assumption about
  process, protocol, or invocation. Changing how it is reached must not change
  what it means. _refines: CMP-R08._

- **CMP.SURF-R03 A mutation is one accepted transition.** Each mutation names
  one domain transition — creation, revision, acceptance change, progress,
  retirement, reconciliation — and is applied whole or not at all.
  _refines: CMP-R02._

- **CMP.SURF-R04 Success yields a stable Receipt.** An accepted mutation returns
  a result naming the affected references and the resulting version identity,
  which remains valid for later reference. _refines: CMP-R09._

- **CMP.SURF-R05 Failure yields nothing.** A rejected mutation produces no
  receipt, records no state, and permits no external record claiming success.
  _refines: CMP-R09._

- **CMP.SURF-R06 Repetition is not duplication.** Submitting the same mutation
  more than once must not record it more than once. What makes two submissions
  *the same* is unresolved (DQ01), and until it resolves the surface does not
  claim exactly-once application. _refines: CMP-R02._

- **CMP.SURF-R07 Reads report their own reliability.** A query answers with the
  convergence state of what it read, so a caller can distinguish a settled
  answer from a provisional one rather than inferring it.
  _refines: CMP-R05._

- **CMP.SURF-R08 External records follow, never lead.** An outside system may
  record a fact referencing a Receipt only after the mutation is accepted. Its
  failure never rolls back, blocks, or alters the result.
  _refines: CMP-R09._
