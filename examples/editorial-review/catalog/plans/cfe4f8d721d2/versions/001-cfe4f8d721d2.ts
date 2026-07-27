import { plan, step, evidence } from "compass"

export const benchmark = step({
  work: "Run all three tools on one identical workload",
  accept: evidence.dataset({ of: "agent-memory-bench", reviewed: "yes" }),
})

export const draft = step({
  work: "Draft the comparison from the dataset",
  dependsOn: [benchmark],
  accept: evidence.artifact({ kind: "draft" }),
})

export const publish = step({
  work: "Editorial sign-off before publishing",
  dependsOn: [draft],
  // A human waiver is as valid a criterion as a passing test. Compass records
  // who claimed it; it does not adjudicate whether the claim is true.
  accept: evidence.any(
    evidence.review({ verdict: "approved", actor: "editor" }),
    evidence.waiver({ actor: "editor" }),
  ),
})

export default plan({
  author: "writer",
  goal: "Publish a defensible comparison of agent-memory tools",
  why: `Three tools claim the same ground and nobody has compared them on one
        workload. Worth writing only if the comparison is real.`,
  steps: [benchmark, draft, publish],
})
