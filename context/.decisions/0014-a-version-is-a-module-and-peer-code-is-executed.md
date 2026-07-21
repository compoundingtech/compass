# A version is a module, and peer code is executed

Status: accepted

Supersedes [0009](./0009-kdl-is-parsed-upstream-and-rendered-canonically-here.md).

## Context

Decision 0012 makes intent a program, so that a dependency is a variable and a
prior Step is referenced by importing it. That idiom only works if a prior
version **is a module** — a reference cannot be imported from a document.

An earlier design kept a separate stored form and treated the authored source as
transient. That is incoherent with the import idiom and was carried for one
reason: the belief that hashing source makes formatting identity-bearing, and
therefore fragile.

## Evidence and Argument

The objection dissolves on inspection. A committed version is immutable — nobody
reformats one, because doing so is not an edit but a forgery. Identity changing
under reformatting is not a fragility; it is the detection mechanism. A content
hash carried in the filename makes any alteration of committed source visible,
which is precisely the guarantee wanted.

So a version is a module, stored as authored, named by the hash of its bytes.
There is no second artifact, no canonical renderer to own, and no divergence
possible between what was written and what was stored.

The consequence has to be stated rather than discovered: **reading a plan
evaluates it, and evaluating a version evaluates every version and plan it
imports.** Under replication those modules were authored elsewhere. Compass
therefore executes code that arrived from another machine.

This was considered against two alternatives. Storing an evaluated form
alongside the source only half-helps, since a successor importing a predecessor
still evaluates the original. Generating data-only binding modules and importing
those would avoid executing foreign authored logic entirely, at the cost of a
generated artifact per version, a generator to keep honest, and an import that no
longer refers to the module actually written.

The direct form is chosen. A plan is code; running a peer's plan is running a
peer's code, and pretending otherwise by interposing a generated shim buys less
than it costs — the shim is still executed, and the honesty is worse.

What makes this acceptable is not trust but the evaluation environment. A plan
runs against an explicitly constructed global with no clock, no filesystem, no
network, no dynamic code construction, and no capability the host did not name.
A hostile plan can compute; it cannot reach anything.

## Options

| Option | Tradeoffs |
| --- | --- |
| A version is a module, imported directly | The import idiom works as written; one artifact; alteration detectable by hash. Reading executes code authored elsewhere |
| A version is a module plus a stored evaluated form | Reads avoid evaluation; a successor still evaluates its predecessor, so foreign execution is only reduced, and two artifacts can disagree |
| Imports resolve to generated data-only bindings | Foreign authored logic never runs; adds a generated artifact and a generator per version, and an import stops referring to the module written |
| A version is a document; references are spelled | No execution at all; loses the import idiom, and returns every reference to a string that can be mistyped or invented |

## Decision

A committed version is the authored module, stored unchanged and named by the
hash of its bytes.

Altering committed source changes its hash and is detected. Committed source is
never rewritten, reformatted, or migrated in place.

Reading a plan evaluates it and everything it imports. **Compass executes modules
authored on other machines.** This is a deliberate consequence of intent being
code, not an oversight, and it is the reason the evaluation environment in 0011
is a correctness boundary rather than a convenience.

Evaluation is bounded. A plan that does not terminate, or that allocates without
limit, is stopped and reported — executing foreign code makes resource
exhaustion a reachable state rather than a theoretical one.

## Consequences

- One artifact per version. The authored form and the stored form are the same
  thing, so they cannot drift.
- Identity is the hash of source bytes. Whitespace is therefore significant, and
  that is deliberate: it is what makes tampering visible.
- Reads are expensive, because they evaluate. A derived, rebuildable index is
  no longer an optimisation but a requirement for reading at scale.
- The capability boundary carries far more weight than when only local drafts
  were evaluated. Every exclusion in 0011 is now protecting against a peer rather
  than against an accident, and relaxing any of them is a change to this
  decision.
- Execution must be bounded in time and memory. A denial of service is now the
  most plausible hostile act, and it is the one the capability list does not
  address.
- A plan cannot be read on a machine that lacks its imports. An unreplicated
  import is worse than a missing predecessor: the plan cannot be evaluated at
  all, rather than merely showing an incomplete lineage.
