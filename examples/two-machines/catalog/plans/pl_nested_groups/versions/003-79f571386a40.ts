import { reconcile } from "compass"
import fuzzSide from "./002-7280a933f7cc.ts"
import splitSide from "./002-ff95b74b4e9f.ts"

// A Reconciliation is an ordinary revision with more than one predecessor.
// Every Step of both sides is carried forward — neither the fuzz step nor the
// guard can be dropped by choosing the other. The only thing stated here is
// what actually changed: the fuzz run waits on the guard.
export default reconcile({
  revises: [fuzzSide, splitSide],
  author: "cos",
  why: `Both were right. Keeping dev's grammar guard and cos's fuzz step, and
        gating the fuzz run on the guard so it exercises the guarded parser.`,
  edit: [
    fuzzSide.steps.fuzz.with({
      dependsOn: [fuzzSide.steps.fix, splitSide.steps.guard],
    }),
  ],
})
