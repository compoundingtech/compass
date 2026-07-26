//! Acceptance: the committed example plans evaluate and their filenames are the
//! hash of their source bytes; and a full lifecycle runs through the CLI.

use compass::cli::{Command, Invocation};
use compass::{catalog, chain, cmd, eval};
use std::path::{Path, PathBuf};

fn example_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir("examples").unwrap() {
        let plans = e.unwrap().path().join("catalog/plans");
        if !plans.is_dir() {
            continue;
        }
        for pl in std::fs::read_dir(&plans).unwrap() {
            let vdir = pl.unwrap().path().join("versions");
            for f in std::fs::read_dir(&vdir).unwrap() {
                let p = f.unwrap().path();
                if p.extension().map(|x| x == "ts").unwrap_or(false) {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}

/// Identity is the SHA-256 of source bytes (decision 0014): the filename must
/// reproduce from the content alone.
#[test]
fn example_filenames_reproduce_from_source_bytes() {
    let files = example_files();
    assert!(files.len() >= 9, "expected the committed examples");
    for p in &files {
        let bytes = std::fs::read(p).unwrap();
        let hash = compass::sha256::sha256_hex(&bytes);
        let name = p.file_name().unwrap().to_str().unwrap();
        let (_seq, prefix) = compass::model::parse_filename(name)
            .unwrap_or_else(|| panic!("{name} is not <seq>-<hash>.ts"));
        assert_eq!(prefix, hash[..12], "{name} does not name its content");
    }
}

/// Every committed example evaluates: reading a plan runs it (decision 0014).
#[test]
fn every_example_evaluates() {
    for p in example_files() {
        let map = eval::eval_plan_file(&p)
            .unwrap_or_else(|e| panic!("{}: [{}] {}", p.display(), e.kind(), e.message()));
        let canon = std::fs::canonicalize(&p).unwrap();
        assert!(map.contains_key(&canon), "{} produced no plan", p.display());
    }
}

/// The reconciliation carries forward every step of both divergent sides, and
/// states only what it changed.
#[test]
fn reconciliation_carries_both_sides_forward() {
    let p = Path::new(
        "examples/two-machines/catalog/plans/pl_nested_groups/versions/003-037d5ddb9db7.ts",
    );
    let map = eval::eval_plan_file(p).unwrap();
    let v = map.get(&std::fs::canonicalize(p).unwrap()).unwrap();
    let names: Vec<&str> = v.steps.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"fuzz") && names.contains(&"guard"),
        "{names:?}"
    );
    let fuzz = v.steps.iter().find(|s| s.name == "fuzz").unwrap();
    assert!(fuzz.depends_on.contains(&"guard".to_string()));
    assert!(fuzz.depends_on.contains(&"fix".to_string()));
}

// ---- e2e lifecycle through the CLI ----

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        fn chmod(p: &Path) {
            if p.is_dir() {
                for e in std::fs::read_dir(p).into_iter().flatten().flatten() {
                    chmod(&e.path());
                }
            } else if let Ok(m) = std::fs::metadata(p) {
                let mut perms = m.permissions();
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
                let _ = std::fs::set_permissions(p, perms);
            }
        }
        chmod(&self.0);
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(root: &Path, cmd: Command) -> Result<cmd::Output, String> {
    cmd::execute(&Invocation {
        command: cmd,
        json: false,
        catalog: Some(root.to_path_buf()),
        author: Some("cos".into()),
    })
}

fn commit_module(root: &Path, plan: &str, source: &str) -> String {
    let vdir = catalog::versions_dir(root, plan);
    std::fs::create_dir_all(&vdir).unwrap();
    let draft = vdir.join("draft.ts");
    // draft.ts may be read-only from a prior write; ensure writable.
    let _ = std::fs::remove_file(&draft);
    std::fs::write(&draft, source).unwrap();
    let out = run(
        root,
        Command::Commit {
            path: draft.clone(),
            plan: Some(plan.into()),
        },
    )
    .unwrap_or_else(|e| panic!("commit failed: {e}"));
    assert_eq!(out.code, 0);
    // The freshly committed head file.
    let store = catalog::load_plan(root, plan).unwrap();
    let an = chain::analyze(&store);
    an.head
        .iter()
        .max_by_key(|a| a.seq)
        .unwrap()
        .path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

#[test]
fn e2e_start_commit_show_history_revise_reconcile() {
    let root = std::env::temp_dir().join(format!("compass-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let guard = Tmp(root.clone());
    let plan = "pl_demo";

    // start — scaffolds a runnable draft (CMP-R11).
    run(
        &root,
        Command::Start {
            plan: plan.into(),
            goal: Some("Ship the widget".into()),
        },
    )
    .unwrap();
    // `start` scaffolds a runnable draft in the plan's versions/ directory.
    let scaffolded = std::fs::read_dir(catalog::versions_dir(&root, plan))
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| {
            let n = e.file_name();
            let n = n.to_string_lossy();
            n.ends_with(".ts") && compass::model::parse_filename(&n).is_none()
        });
    assert!(scaffolded, "start should scaffold an editable draft module");

    // commit a real root (edit stands in for the operator editing the draft).
    let root_mod = r#"import { plan, step, evidence } from "compass"
export const build = step({ work: "Build it", accept: evidence.test({ name: "t", status: "pass" }) })
export const ship = step({ work: "Ship it", dependsOn: [build], accept: evidence.review({ actor: "cos", verdict: "approved" }) })
export default plan({ author: "cos", goal: "Ship the widget", why: "It is time.", steps: [build, ship] })
"#;
    let v1 = commit_module(&root, plan, root_mod);

    // show + history + ready read the plan by evaluating it.
    assert!(run(&root, Command::Show { plan: plan.into() })
        .unwrap()
        .text
        .contains("build"));
    let hist = run(&root, Command::History { plan: plan.into() }).unwrap();
    assert!(hist.text.contains("It is time"));
    assert!(run(&root, Command::Ready { plan: plan.into() })
        .unwrap()
        .text
        .contains("build"));

    // an identical re-commit is a no-op success (a repeat is a repeat).
    let vdir = catalog::versions_dir(&root, plan);
    std::fs::write(vdir.join("again.ts"), root_mod).unwrap();
    let again = run(
        &root,
        Command::Commit {
            path: vdir.join("again.ts"),
            plan: Some(plan.into()),
        },
    )
    .unwrap();
    assert!(again.text.contains("already committed"), "{}", again.text);

    // revise — a function of the predecessor, carrying every step forward.
    let rev = format!(
        r#"import {{ step, evidence }} from "compass"
import prior from "./{v1}"
export default prior.revise({{
  author: "cos",
  why: "Reword the build step; the intent is unchanged but clearer.",
  edit: [prior.steps.build.with({{ work: "Build it, carefully" }})],
}})
"#
    );
    let v2 = commit_module(&root, plan, &rev);
    assert!(run(&root, Command::Show { plan: plan.into() })
        .unwrap()
        .text
        .contains("carefully"));

    // diverge: two revisions from the same predecessor v2.
    let side_a = format!(
        r#"import {{ step, evidence }} from "compass"
import prior from "./{v2}"
export const fuzz = step({{ work: "Fuzz it", dependsOn: [prior.steps.build], accept: evidence.test({{ name: "fz", status: "pass" }}) }})
export default prior.revise({{ author: "cos", why: "Add a fuzz step.", add: [fuzz] }})
"#
    );
    let side_b = format!(
        r#"import {{ step, evidence }} from "compass"
import prior from "./{v2}"
export const doc = step({{ work: "Document it", dependsOn: [prior.steps.build], accept: evidence.review({{ actor: "cos", verdict: "approved" }}) }})
export default prior.revise({{ author: "dev", why: "Add a docs step.", add: [doc] }})
"#
    );
    let a = commit_module(&root, plan, &side_a);
    // side_b's draft is a different file so both land as divergent heads.
    std::fs::write(vdir.join("b.ts"), &side_b).unwrap();
    run(
        &root,
        Command::Commit {
            path: vdir.join("b.ts"),
            plan: Some(plan.into()),
        },
    )
    .unwrap();
    let _ = std::fs::remove_file(vdir.join("b.ts"));

    let store = catalog::load_plan(&root, plan).unwrap();
    let an = chain::analyze(&store);
    assert_eq!(an.head.len(), 2, "two divergent heads");
    assert!(an.diverged());
    let b = an
        .head
        .iter()
        .find(|h| h.path.file_name().unwrap().to_str().unwrap() != a)
        .unwrap()
        .path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // reconcile the two divergent heads.
    let recon = format!(
        r#"import {{ reconcile }} from "compass"
import sideA from "./{a}"
import sideB from "./{b}"
export default reconcile({{ revises: [sideA, sideB], author: "cos", why: "Both are worth keeping." }})
"#
    );
    commit_module(&root, plan, &recon);
    let store = catalog::load_plan(&root, plan).unwrap();
    let an = chain::analyze(&store);
    assert_eq!(an.head.len(), 1, "reconciliation converges the heads");
    assert!(!an.diverged());
    assert!(
        an.ever_diverged(),
        "the divergence remains visible as history"
    );

    // the reconciled head carries every step of both sides.
    let head = an.head[0];
    let map = eval::eval_plan_file(&head.path).unwrap();
    let v = map
        .get(&std::fs::canonicalize(&head.path).unwrap())
        .unwrap();
    let names: Vec<&str> = v.steps.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"fuzz") && names.contains(&"doc"),
        "{names:?}"
    );

    drop(guard);
}

// ---- engine failure modes (regression guards) ----

fn tmp_module(dir: &Path, name: &str, src: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, src).unwrap();
    p
}

#[test]
fn a_missing_import_reads_as_unresolved_not_failed() {
    let root = std::env::temp_dir().join(format!("compass-unres-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let _g = Tmp(root.clone());
    let p = tmp_module(
        &root,
        "002-000000000000.ts",
        "import prior from \"./001-ffffffffffff.ts\"\nexport default prior.revise({ author: \"cos\", why: \"x\" })\n",
    );
    match eval::eval_plan_file(&p) {
        Err(e) => assert_eq!(e.kind(), "unresolved", "got: {}", e.message()),
        Ok(_) => panic!("expected an unresolved read"),
    }
}

#[test]
fn a_nonterminating_plan_is_stopped_not_awaited() {
    let root = std::env::temp_dir().join(format!("compass-stop-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let _g = Tmp(root.clone());
    let p = tmp_module(
        &root,
        "001-000000000000.ts",
        "import { plan, step, evidence } from \"compass\"\nwhile (true) {}\nexport const a = step({ work: \"x\", accept: evidence.test({ status: \"pass\" }) })\nexport default plan({ author: \"cos\", goal: \"g\", why: \"w\", steps: [a] })\n",
    );
    assert_eq!(eval::eval_plan_file(&p).unwrap_err().kind(), "stopped");
}

#[test]
fn the_sandbox_grants_no_dynamic_code_or_clock() {
    let root = std::env::temp_dir().join(format!("compass-sandbox-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let _g = Tmp(root.clone());
    // eval / Date are absent (decision 0011): a plan that reaches for them fails.
    for probe in ["eval(\"1\")", "new Date()", "Math.random()"] {
        let src = format!(
            "import {{ plan, step, evidence }} from \"compass\"\nconst _x = {probe}\nexport const a = step({{ work: \"x\", accept: evidence.test({{ status: \"pass\" }}) }})\nexport default plan({{ author: \"cos\", goal: \"g\", why: \"w\", steps: [a] }})\n"
        );
        let p = tmp_module(&root, "001-000000000000.ts", &src);
        assert_eq!(
            eval::eval_plan_file(&p).unwrap_err().kind(),
            "failed",
            "probe `{probe}` should be unavailable"
        );
    }
}
