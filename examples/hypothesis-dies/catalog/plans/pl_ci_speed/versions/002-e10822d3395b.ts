import { step, evidence } from "compass"
import prior from "./001-634e2a7c458b.ts"

export default prior.revise({
  author: "cos",
  why: `The measurement kills the hypothesis. 26 of the 34 minutes are artifact
        download from a single-homed mirror; cache misses cost about 90 seconds
        total. Retiring the cache work and retargeting at the mirror.`,
  retire: [prior.steps.fixCache],
  add: [
    step({
      work: "Put build artifacts behind a regional mirror",
      dependsOn: [prior.steps.measure],
      accept: evidence.measurement({ of: "artifact-download", below: "120" }),
    }),
  ],
})
