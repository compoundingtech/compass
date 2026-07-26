use compass::eval::eval_plan_file;
use std::path::Path;

fn show(p: &str) {
    let path = Path::new(p);
    println!("\n===== {p}");
    match eval_plan_file(path) {
        Ok(map) => {
            let canon = std::fs::canonicalize(path).unwrap();
            let v = map.get(&canon).expect("entry in map");
            println!(
                "author={} goal={:?} retired={}",
                v.author, v.goal, v.retired
            );
            println!("why={:?}", v.why);
            for s in &v.steps {
                println!(
                    "  step {} retired={} deps={:?} supersedes={:?}",
                    s.name, s.retired, s.depends_on, s.supersedes
                );
                println!("      work={:?}", s.work);
                println!("      accept={}", s.accept);
            }
        }
        Err(e) => println!("ERR [{}] {}", e.kind(), e.message()),
    }
}

#[test]
fn spike_all_examples() {
    for p in [
        "examples/editorial-review/catalog/plans/pl_agent_memory_piece/versions/001-cfe4f8d721d2.ts",
        "examples/editorial-review/catalog/plans/pl_agent_memory_piece/versions/002-043517240262.ts",
        "examples/hypothesis-dies/catalog/plans/pl_ci_speed/versions/001-634e2a7c458b.ts",
        "examples/hypothesis-dies/catalog/plans/pl_ci_speed/versions/002-e10822d3395b.ts",
        "examples/hypothesis-dies/catalog/plans/pl_ci_speed/versions/003-549d0e4af2eb.ts",
        "examples/two-machines/catalog/plans/pl_nested_groups/versions/001-8e528ff9bc56.ts",
        "examples/two-machines/catalog/plans/pl_nested_groups/versions/002-7280a933f7cc.ts",
        "examples/two-machines/catalog/plans/pl_nested_groups/versions/002-ff95b74b4e9f.ts",
        "examples/two-machines/catalog/plans/pl_nested_groups/versions/003-79f571386a40.ts",
    ] {
        show(p);
    }
}
