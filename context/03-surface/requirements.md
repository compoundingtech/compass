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
  receipt, records no state, permits no external record claiming success, and
  leaves what the caller authored untouched. Nothing is written back into
  authored content, so a refusal costs exactly the work of resubmitting.
  _refines: CMP-R09, CMP-R02._

- **CMP.SURF-R06 Repetition is not duplication.** Submitting the same mutation
  more than once records it once. Two submissions are the same when they carry
  the same authored content, which yields the same identity and therefore one
  version — a property of the data rather than a protocol both sides must
  implement correctly. _refines: CMP-R02, CMP-R10._

- **CMP.SURF-R07 Reads report their own reliability.** A query answers with the
  convergence state of what it read, so a caller can distinguish a settled
  answer from a provisional one rather than inferring it.
  _refines: CMP-R05._

- **CMP.SURF-R08 External records follow, never lead.** An outside system may
  record a fact referencing a Receipt only after the mutation is accepted. Its
  failure never rolls back, blocks, or alters the result.
  _refines: CMP-R09._

- **CMP.SURF-R09 A read distinguishes why it could not answer.** Absent,
  unresolved, and stopped are three different answers and must not collapse into
  one. A Plan that is not here, a Plan that cannot be evaluated because what it
  references has not arrived, and a Plan whose evaluation exceeded its bound
  each call for a different response — look elsewhere, wait, or stop trusting
  the Plan — and a caller told only that the read failed will pick among them by
  guessing. _refines: CMP-R05, CMP-R07, CMP-R13._
