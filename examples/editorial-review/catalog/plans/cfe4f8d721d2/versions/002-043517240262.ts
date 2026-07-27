import prior from "./001-cfe4f8d721d2.ts"

export default prior.revise({
  author: "writer",
  why: `One tool changed its storage engine mid-benchmark, so the dataset is not
        comparable across all three. Narrowing the piece to the two that held
        still, and saying so.`,
  edit: [
    prior.steps.benchmark.with({
      work: "Run the two stable tools on one identical workload",
    }),
    prior.steps.draft.with({
      work: "Draft the comparison, two tools, and name the third's instability",
    }),
  ],
})
