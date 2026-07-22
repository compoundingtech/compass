# Requirements: Authoring API

> **Role.** The library a Plan imports — the surface through which intent is
> written as code. It is what makes a Step a named declaration, a dependency a
> variable, and a revision a function of its predecessor. It realizes the data
> model of [01-data-model](../01-data-model/requirements.md) as a thing an
> author writes, and its output is evaluated per
> [decision 0014](../.decisions/0014-a-version-is-a-module-and-peer-code-is-executed.md).
> Each requirement `refines:` a `CMP-R*`.

## Assumptions

- **CMP.API-A01 The author is an agent or a person writing code.** The surface
  is judged by whether such an author writes a correct Plan without consulting a
  grammar — the same standard the whole pivot to code was made against.

## Requirements

- **CMP.API-R01 Identity is the declaration.** A Step's identity is the binding
  it is declared under, so authoring an identity and authoring the work are the
  same act. The surface must offer no way to set an identity separately, because
  a separate identity is a value that can be set wrongly.
  _refines: CMP-R10, CMP-R02._

- **CMP.API-R02 A dependency is a reference, not a name.** A Step depends on
  another by referring to its binding, never by writing its identity as a
  string. An unresolved dependency must therefore be a reference that does not
  exist — caught where it is written — rather than a dangling edge discovered
  later. _refines: CMP-R10._

- **CMP.API-R03 A revision is a function of its predecessor.** Producing a
  version from a prior one is an operation *on* that version. It takes edits,
  additions, and retirements, and offers nothing that removes a Step: every Step
  of the predecessor is carried forward by the operation itself, so a Step
  cannot be dropped by omission. _refines: CMP-R02._

- **CMP.API-R04 An edit names the Step it changes.** Editing a carried-forward
  Step refers to that Step through the predecessor, so the edit cannot silently
  create a new Step or target one that is not there. What an edit may change is
  the work, the acceptance, and the dependencies; it cannot change identity.
  _refines: CMP-R02, CMP-R10._

- **CMP.API-R05 A cross-plan reference is an import.** Depending on a Step in
  another Plan is importing that Plan's version and referring to the Step. This
  is what makes the reference checkable, and it is why the other Plan must be
  present to evaluate this one. _refines: CMP-R08, CMP-R09._

- **CMP.API-R06 The evidence vocabulary is open and typed.** The constructors
  for acceptance evidence are values a use case defines, not a fixed set the API
  ships. Their shape is expressed in the type system, so a mistyped attribute or
  value is caught while authoring — but the API defines no vocabulary of its own
  and privileges none, keeping the model domain-neutral.
  _refines: CMP-R08, CMP.DM-R13d._

- **CMP.API-R07 A plan is pure data.** The surface exposes only construction of
  values. It offers nothing that reads the clock, the filesystem, the network,
  or the environment, and composing a Plan cannot have an effect. Purity is a
  property the API must not let an author violate through it, independent of the
  evaluation environment that also forbids it.
  _refines: CMP-R12, CMP-R13._

- **CMP.API-R08 The output is authored, not generated.** What the author writes
  is the stored version — the API does not compile intent into a separate form
  behind the author's back. A reader of a committed version reads the same
  library calls the author wrote. _refines: CMP-R02._

- **CMP.API-R09 Starting is one import and one call.** Authoring a first Plan
  requires importing the library and calling it — no configuration, no
  registration, no identifiers to obtain first. The floor of effort is a single
  file that already runs. _refines: CMP-R11._

- **CMP.API-R10 The API is versioned, and a plan records what it was authored
  against.** The library will change, and a committed version is immutable and
  read forever. A version must be evaluable under the API it was written against,
  so the API carries a compatibility identity and a version declares it.
  _refines: CMP-R07._
