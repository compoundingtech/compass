# examples

Three plans, worked end to end, in three different domains. Each is a real
catalog: the version files are named by the hash of their own bytes, and each
revision imports its predecessor by that name — so the lineage is genuine, not
illustrative.

They are point-in-time snapshots. Compass is early and does not yet evaluate
these; they show what authoring a plan looks like and are the reference the
design is measured against. The authoring surface is specified in
[`context/06-api/`](../context/06-api/spec.md).

| Example | Domain | What it shows |
| --- | --- | --- |
| [hypothesis-dies](./hypothesis-dies/) | engineering | a plan that turns out wrong, and why that is the point |
| [editorial-review](./editorial-review/) | writing / research | the model is not about code; a human sign-off is a criterion |
| [two-machines](./two-machines/) | any | two agents revise at once; divergence, then reconciliation |

## How to read a plan

A plan is a TypeScript module. A step is a named declaration, so its name is its
identity. A dependency is a reference to another step's binding — never a string
— so a dependency that does not resolve is a name that does not exist, caught
where it is written.

```ts
export const measure = step({ work: "...", accept: evidence.measurement({ of: "build-phases" }) })
export const fix     = step({ work: "...", dependsOn: [measure], accept: /* ... */ })
export default plan({ author: "cos", goal: "...", why: "...", steps: [measure, fix] })
```

A revision imports its predecessor and is a function of it. It can edit, add, and
retire — it has no way to *remove* a step, because every step is carried forward
by the revision itself. A step that is no longer wanted is retired, and stays in
the record marked as such.

```ts
import prior from "./001-<hash>.ts"
export default prior.revise({ author: "cos", why: "...", retire: [prior.steps.fixCache], add: [ /* ... */ ] })
```

The words inside `accept(...)` — `test`, `measurement`, `review`, `waiver`, and
their attributes — are not Compass's. A use case declares its own vocabulary as
typed constructors, so a mistyped attribute is a compile error while authoring,
and Compass stays neutral about what any of it means.

## Verifying the artifacts

Every filename is the first 12 hex of the SHA-256 of the file's bytes:

```sh
for f in $(find . -name '*.ts'); do
  want=$(basename "$f" | sed 's/^[0-9]*-//;s/.ts//')
  got=$(sha256sum "$f" | cut -c1-12)
  [ "$want" = "$got" ] && echo "ok   $f" || echo "BAD  $f"
done
```
