# StepRef is minted, not content-derived

Status: accepted

## Context

Decision 0002 makes Plan Versions content-addressed. The obvious extension is to
address Steps the same way, giving one identity mechanism for the whole model.

## Evidence and Argument

Content addressing and stable identity are in direct conflict for Steps.

A Step's reference must survive revision of its own text. Rewording "fix the
parser" to "fix the expression parser" is a clarification of the same intended
work; a reader following that work across the version chain must see one Step,
not a retirement and a replacement. If StepRef were the content hash, every
wording change would mint a new identity and the chain would become
uninterpretable exactly where interpretation matters most.

The conflict does not arise for versions, because a version's identity *should*
change whenever its bytes change — that is what makes the chain tamper-evident
and forks visible. The two layers want opposite properties, so they need
different mechanisms rather than one unified one.

This is the first thing an implementation encounters, and getting it wrong is
expensive: refs leak into receipts, external observations, and any tool holding
a reference, so a late correction is a data migration.

## Options

| Option | Tradeoffs |
| --- | --- |
| Minted opaque StepRef | Survives revision; requires generating and carrying an id in authored content |
| Content-hash StepRef | One identity mechanism for everything; any wording change silently mints a new Step |
| Path or title slug | Human-readable; breaks on rename, and collides |
| Positional index | No bookkeeping; breaks on any reordering or insertion |

## Decision

`StepRef` is minted at Step creation and is opaque. It is never derived from
Step content. It survives title and metadata revision while the intended work is
unchanged, and is never reused after the Step is retired.

Content hashes address Plan Versions only.

When intended work genuinely changes identity — a Step is split, merged, or
replaced — the successor names what it replaces with an explicit `supersedes`
edge rather than silently inheriting or reusing the reference.

`PlanRef` follows the same rule for the same reason.

## Consequences

- Authored Plan Versions carry Step references explicitly; they are not derivable
  from the document text.
- Identity changes are always explicit and auditable, never a side effect of
  editing prose.
- Two identity mechanisms coexist in the model. This is intentional and the
  ontology names both, since a single mechanism cannot satisfy both properties.
