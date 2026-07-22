import { step, evidence } from "compass"
import prior from "./001-8e528ff9bc56.ts"

// Authored on machine A, from version 001-8e528ff9bc56.ts, before B's revision had replicated.
export default prior.revise({
  author: "cos",
  why: `One reproduction is not enough. Adding a fuzz step so the fix is checked
        against generated nestings, not just the one case that started this.`,
  add: [
    step({
      work: "Fuzz nested-group parsing",
      dependsOn: [prior.steps.fix],
      accept: evidence.test({ name: "parser::nested_fuzz", status: "pass" }),
    }),
  ],
})
