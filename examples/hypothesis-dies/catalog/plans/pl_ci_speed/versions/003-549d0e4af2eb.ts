import { step, evidence } from "compass"
import prior from "./002-e10822d3395b.ts"

export default prior.revise({
  author: "cos",
  why: `A regional mirror still leaves the cold-start case: the first build of a
        new branch is the slow one. Adding a warm path so branch creation is not
        also the slow build.`,
  add: [
    step({
      work: "Pre-warm the mirror on branch create",
      accept: evidence.measurement({ of: "cold-build", below: "600" }),
    }),
  ],
})
