# Requirements: Evals

> **Role.** Executable scenarios that prove the system does what the other nodes
> claim. An eval is evidence, not a test of internals: it drives the same
> surface an operator drives and asserts what an operator would observe. Each
> requirement `refines:` a `CMP-R*`.

## Assumptions

- **CMP.EVAL-A01 Scenarios are reproducible offline.** Every scenario runs on
  one machine with no network and no replication mechanism, including scenarios
  about several machines. Replication is simulated by moving files, which is
  exactly what a union sync does.

## Acceptable Tradeoffs

- **CMP.EVAL-T01 Evals prove behaviour, not absence of bugs.** A passing suite
  says the claims it covers held for the cases it ran. Coverage of claims is
  reportable (CMP.EVAL-R04); coverage of inputs is not.

## Requirements

- **CMP.EVAL-R01 An eval proves a stated claim.** Every eval names the
  requirement or decision it exercises. An eval that proves nothing named is
  either an untracked claim or a test of internals, and both belong elsewhere.
  _refines: CMP-R01._

- **CMP.EVAL-R02 Evals assert observable properties, never exact output.**
  Assertions are about counts, relationships, and invariants — how many
  versions, which reference survived, whether divergence is open — never about
  bytes. Identity is a content hash, so any change to a document rewrites every
  hash in every expected output; a suite that pins bytes would churn on
  unrelated changes and be regenerated without being read.
  _refines: CMP-R07._

- **CMP.EVAL-R03 Evals drive the operator surface.** A scenario runs the CLI as
  an operator would, not the library. What is proven must be what a user can
  actually obtain; a claim provable only through internal APIs is not proven.
  _refines: CMP-R01._

- **CMP.EVAL-R04 Coverage of claims is visible.** The suite reports which
  requirements and decisions have no eval. An unproven claim is acceptable; an
  unproven claim that looks proven is not, and without this the suite silently
  drifts into covering only what was easy.
  _refines: CMP-R07._

- **CMP.EVAL-R05 A failure names the claim, not just the mismatch.** When an
  assertion fails, the report states which claim is now unsupported and at which
  step of the scenario. A diff of two numbers does not tell a reader what broke.
  _refines: CMP-R07._

- **CMP.EVAL-R06 Scenarios are readable as documentation.** A scenario states
  its situation in prose before it acts, so it explains the system to someone
  who never runs it. This is the same artifact serving two purposes deliberately:
  an example that cannot run goes stale, and a test nobody reads proves nothing
  to a newcomer. _refines: CMP-R03._

- **CMP.EVAL-R07 Scenarios cover the states the design calls normal.**
  Divergence, orphans, retries, and reconciliation are ordinary conditions here,
  not edge cases. A suite that exercises only the linear path proves the system
  in the one state its design does not consider interesting.
  _refines: CMP-R04, CMP-R05._

- **CMP.EVAL-R08 Evals are hermetic.** A scenario creates its own catalog,
  depends on no ambient state, and leaves nothing behind. Identity is minted
  from randomness, so a scenario must not assume a reference it did not capture
  from output. _refines: CMP-R06._
