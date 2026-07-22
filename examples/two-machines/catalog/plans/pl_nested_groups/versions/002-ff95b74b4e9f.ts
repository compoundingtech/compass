import { step, evidence } from "compass"
import prior from "./001-8e528ff9bc56.ts"

// Authored on machine B, from the same version 001-8e528ff9bc56.ts. Shares a predecessor with
// the machine-A revision, so the two are a Divergence, not a sequence.
export default prior.revise({
  author: "dev",
  why: `Splitting the fix so each part lands reviewable: the tokenizer change,
        then a grammar guard that rejects an unterminated group outright.`,
  add: [
    step({
      work: "Add a grammar guard for unterminated groups",
      dependsOn: [prior.steps.fix],
      accept: evidence.test({ name: "parser::unterminated_guard", status: "pass" }),
    }),
  ],
})
