import { plan, step, evidence } from "compass"

export const measure = step({
  work: "Measure where the 34 minutes actually go",
  accept: evidence.measurement({ of: "build-phases" }),
})

export const fixCache = step({
  work: "Fix cache key instability",
  dependsOn: [measure],
  accept: evidence.measurement({ of: "cache-hit-rate", above: "80" }),
})

export default plan({
  author: "cos",
  goal: "CI builds finish under 10 minutes",
  why: `Builds average 34 minutes and block every merge. The cache hit rate reads
        12%, so the cache is the obvious suspect. Measure first, then fix it.`,
  steps: [measure, fixCache],
})
