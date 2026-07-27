import { plan, step, evidence } from "compass"

export const reproduce = step({
  work: "Reproduce with a failing test",
  accept: evidence.test({ name: "parser::nested_groups", status: "fail" }),
})

export const fix = step({
  work: "Fix the tokenizer's delimiter handling",
  dependsOn: [reproduce],
  accept: evidence.test({ name: "parser::nested_groups", status: "pass" }),
})

export default plan({
  author: "cos",
  goal: "Nested groups parse correctly",
  why: `Reproduction showed the tokenizer drops the closing delimiter. Fix it,
        gated on a failing test that turns green.`,
  steps: [reproduce, fix],
})
