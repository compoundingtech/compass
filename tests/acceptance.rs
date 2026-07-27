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
    let p =
        Path::new("examples/two-machines/catalog/plans/8e528ff9bc56/versions/003-037d5ddb9db7.ts");
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

/// The PlanRef a source files under (decision 0017): for an origin, the
/// hash-prefix of its own bytes. Revisions of it share this same PlanRef.
fn planref_of(source: &str) -> String {
    compass::sha256::sha256_hex(source.as_bytes())[..12].to_string()
}

/// Author `source` as a draft in the plan's versions dir (so a revision's sibling
/// import resolves), commit it — the CLI names no plan, deriving it (decision
/// 0017) — and return the freshly committed head filename.
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

    // The real origin the operator will author (edit stands in for editing the
    // scaffold). Its PlanRef is derived from these bytes (decision 0017).
    let root_mod = r#"import { plan, step, evidence } from "compass"
export const build = step({ work: "Build it", accept: evidence.test({ name: "t", status: "pass" }) })
export const ship = step({ work: "Ship it", dependsOn: [build], accept: evidence.review({ actor: "cos", verdict: "approved" }) })
export default plan({ author: "cos", goal: "Ship the widget", why: "It is time.", steps: [build, ship] })
"#;
    let plan = &planref_of(root_mod);

    // start — scaffolds a runnable draft (CMP-R11), naming no plan (decision 0017).
    run(&root, Command::Start { goal: None }).unwrap();
    // `start` scaffolds a runnable draft into the staging area, not a plan dir —
    // the plan has no identity until its origin is committed.
    let scaffolded = std::fs::read_dir(catalog::drafts_dir(&root))
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| e.file_name().to_string_lossy().ends_with(".ts"));
    assert!(scaffolded, "start should scaffold an editable draft module");

    // commit the origin: it files under its own hash, which is the PlanRef.
    let v1 = commit_module(&root, plan, root_mod);
    assert_eq!(
        &v1[4..16],
        plan.as_str(),
        "the origin's version-id and the PlanRef are the same hash (decision 0017)"
    );
    assert!(
        catalog::plan_dir(&root, plan).is_dir(),
        "the origin files under its derived PlanRef {plan}"
    );

    // show + history + ready read the plan by evaluating it.
    assert!(run(&root, Command::Show { plan: plan.clone() })
        .unwrap()
        .text
        .contains("build"));
    let hist = run(&root, Command::History { plan: plan.clone() }).unwrap();
    assert!(hist.text.contains("It is time"));
    assert!(run(&root, Command::Ready { plan: plan.clone() })
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
    assert!(run(&root, Command::Show { plan: plan.clone() })
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

/// Write `source` as an admitted (content-addressed, hash-named) version file
/// and return its filename, so a sibling or cross-plan import can reference it.
fn admit(root: &Path, plan: &str, seq: u64, source: &str) -> String {
    let (p, _hash, _created) = catalog::write_version(root, plan, seq, source.as_bytes()).unwrap();
    p.file_name().unwrap().to_str().unwrap().to_string()
}

// ---- Fix 1: the module resolver admits only legitimate modules ----

#[test]
fn imports_are_restricted_to_legitimate_modules() {
    let root = std::env::temp_dir().join(format!("compass-imports-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let _g = Tmp(root.clone());
    let plan = "pl_imp";
    let vdir = catalog::versions_dir(&root, plan);
    std::fs::create_dir_all(&vdir).unwrap();

    // A legitimate root version, admitted so a sibling import can resolve it.
    let root_src = r#"import { plan, step, evidence } from "compass"
export const a = step({ work: "do a", accept: evidence.test({ status: "pass" }) })
export default plan({ author: "cos", goal: "g", why: "w", steps: [a] })
"#;
    let v1 = admit(&root, plan, 1, root_src);

    // (a) an absolute-path import is refused at eval as illegitimate. The import
    // is used (`String(_x)`) so the TypeScript transpiler does not elide it.
    let abs = tmp_module(
        &vdir,
        "draft_abs.ts",
        "import { plan, step, evidence } from \"compass\"\nimport _x from \"/etc/passwd\"\nexport const a = step({ work: \"x\", accept: evidence.test({ status: \"pass\" }) })\nexport default plan({ author: \"cos\", goal: \"g\", why: String(_x), steps: [a] })\n",
    );
    let e = eval::eval_plan_file(&abs).unwrap_err();
    assert_eq!(e.kind(), "failed", "{}", e.message());
    assert!(
        e.message().contains("illegitimate import"),
        "absolute import must be refused: {}",
        e.message()
    );

    // (b) a relative path that escapes the catalog is refused, even when its
    // basename is content-addressed.
    let outside = tmp_module(
        &vdir,
        "draft_out.ts",
        "import { plan, step, evidence } from \"compass\"\nimport _x from \"../../../../000-000000000000.ts\"\nexport const a = step({ work: \"x\", accept: evidence.test({ status: \"pass\" }) })\nexport default plan({ author: \"cos\", goal: \"g\", why: String(_x), steps: [a] })\n",
    );
    let e = eval::eval_plan_file(&outside).unwrap_err();
    assert_eq!(e.kind(), "failed", "{}", e.message());
    assert!(
        e.message().contains("illegitimate import"),
        "an escaping import must be refused: {}",
        e.message()
    );

    // (c) a legitimate same-plan sibling import resolves and evaluates.
    let rev_src = format!(
        r#"import prior from "./{v1}"
export default prior.revise({{ author: "cos", why: "reword", edit: [prior.steps.a.with({{ work: "do a, better" }})] }})
"#
    );
    let good = tmp_module(&vdir, "draft_good.ts", &rev_src);
    let map = eval::eval_plan_file(&good).unwrap_or_else(|e| {
        panic!(
            "valid sibling import must resolve: [{}] {}",
            e.kind(),
            e.message()
        )
    });
    let v = map.get(&std::fs::canonicalize(&good).unwrap()).unwrap();
    assert_eq!(
        v.steps.iter().find(|s| s.name == "a").unwrap().work,
        "do a, better"
    );
}

// ---- Fix 2: a cross-plan reference commits and is not a predecessor ----

#[test]
fn a_cross_plan_reference_is_not_a_predecessor() {
    let root = std::env::temp_dir().join(format!("compass-xplan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let _g = Tmp(root.clone());
    catalog::init(&root).unwrap();

    // The other plan, filed under its own PlanRef (its origin hash).
    let dep_src = r#"import { plan, step, evidence } from "compass"
export const seed = step({ work: "Seed work", accept: evidence.test({ status: "pass" }) })
export default plan({ author: "cos", goal: "dep", why: "the referenced plan", steps: [seed] })
"#;
    let dep_plan = planref_of(dep_src);
    let dep_v1 = admit(&root, &dep_plan, 1, dep_src);

    // A first version of the main plan that references the dep plan's version
    // cross-plan, by its PlanRef directory. The reference is real (it reads a
    // value from the other plan's version) but is not a predecessor: no parent.
    let main_src = format!(
        r#"import {{ plan, step, evidence }} from "compass"
import dep from "../../{dep_plan}/versions/{dep_v1}"
export const local = step({{ work: "Local, mirrors " + dep.steps.seed.work, accept: evidence.test({{ status: "pass" }}) }})
export default plan({{ author: "cos", goal: "main", why: "references dep cross-plan", steps: [local] }})
"#
    );
    // The main plan has no predecessor, so its PlanRef is its own hash — author
    // the draft in that dir so the cross-plan `../../` reference resolves.
    let main_plan = planref_of(&main_src);
    let vdir = catalog::versions_dir(&root, &main_plan);
    std::fs::create_dir_all(&vdir).unwrap();
    let draft = vdir.join("draft.ts");
    std::fs::write(&draft, &main_src).unwrap();

    let out = run(
        &root,
        Command::Commit {
            path: draft.clone(),
        },
    )
    .unwrap_or_else(|e| panic!("cross-plan commit must succeed, not be rejected: {e}"));
    assert_eq!(out.code, 0, "{}", out.text);
    assert!(
        out.text.contains("created"),
        "a cross-plan reference has no predecessor, so this is a creation: {}",
        out.text
    );

    // The committed version records no parent, is filed under its own PlanRef,
    // and is not an orphan.
    let store = catalog::load_plan(&root, &main_plan).unwrap();
    assert_eq!(store.versions.len(), 1, "rejected: {:?}", store.rejected);
    assert!(
        store.versions[0].parents.is_empty(),
        "a cross-plan reference must not be a parent: {:?}",
        store.versions[0].parents
    );
    let an = chain::analyze(&store);
    assert!(
        !an.is_orphan(&store.versions[0].hash),
        "a cross-plan-referencing version must not read as an orphan"
    );
}

// ---- Fix 3: reconciliation refuses silent divergent-step loss ----

#[test]
fn reconciliation_refuses_divergent_same_step_without_an_explicit_edit() {
    let root = std::env::temp_dir().join(format!("compass-recon-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let _g = Tmp(root.clone());
    let plan = "pl_rec";
    let vdir = catalog::versions_dir(&root, plan);
    std::fs::create_dir_all(&vdir).unwrap();

    let v1_src = r#"import { plan, step, evidence } from "compass"
export const build = step({ work: "Build", accept: evidence.test({ status: "pass" }) })
export const ship = step({ work: "Ship", accept: evidence.test({ status: "pass" }) })
export default plan({ author: "cos", goal: "g", why: "w", steps: [build, ship] })
"#;
    let v1 = admit(&root, plan, 1, v1_src);

    // Two sides that edit the SAME step differently.
    let side_a = admit(
        &root,
        plan,
        2,
        &format!(
            r#"import prior from "./{v1}"
export default prior.revise({{ author: "cos", why: "carefully", edit: [prior.steps.build.with({{ work: "Build carefully" }})] }})
"#
        ),
    );
    let side_b = admit(
        &root,
        plan,
        2,
        &format!(
            r#"import prior from "./{v1}"
export default prior.revise({{ author: "dev", why: "quickly", edit: [prior.steps.build.with({{ work: "Build quickly" }})] }})
"#
        ),
    );

    // Without an explicit edit, the divergent `build` is a refused conflict.
    let bad = tmp_module(
        &vdir,
        "recon_bad.ts",
        &format!(
            r#"import {{ reconcile }} from "compass"
import sa from "./{side_a}"
import sb from "./{side_b}"
export default reconcile({{ revises: [sa, sb], author: "cos", why: "merge" }})
"#
        ),
    );
    let e = eval::eval_plan_file(&bad).unwrap_err();
    assert_eq!(e.kind(), "failed", "{}", e.message());
    assert!(
        e.message().contains("reconciliation conflict") && e.message().contains("build"),
        "the refusal must name the conflicting step: {}",
        e.message()
    );

    // With an explicit edit stating the surviving intent, it succeeds.
    let good = tmp_module(
        &vdir,
        "recon_good.ts",
        &format!(
            r#"import {{ reconcile }} from "compass"
import sa from "./{side_a}"
import sb from "./{side_b}"
export default reconcile({{ revises: [sa, sb], author: "cos", why: "merge", edit: [sa.steps.build.with({{ work: "Build carefully and quickly" }})] }})
"#
        ),
    );
    let map = eval::eval_plan_file(&good).unwrap_or_else(|e| {
        panic!(
            "an explicit edit must resolve the conflict: {}",
            e.message()
        )
    });
    let v = map.get(&std::fs::canonicalize(&good).unwrap()).unwrap();
    assert_eq!(
        v.steps.iter().find(|s| s.name == "build").unwrap().work,
        "Build carefully and quickly"
    );

    // A reconciliation whose sides diverge only by ADDING different steps (no
    // shared step disagrees) is not a conflict and still works.
    let add_a = admit(
        &root,
        plan,
        2,
        &format!(
            r#"import {{ step, evidence }} from "compass"
import prior from "./{v1}"
export const fuzz = step({{ work: "Fuzz", dependsOn: [prior.steps.build], accept: evidence.test({{ status: "pass" }}) }})
export default prior.revise({{ author: "cos", why: "add fuzz", add: [fuzz] }})
"#
        ),
    );
    let add_b = admit(
        &root,
        plan,
        2,
        &format!(
            r#"import {{ step, evidence }} from "compass"
import prior from "./{v1}"
export const doc = step({{ work: "Doc", dependsOn: [prior.steps.build], accept: evidence.test({{ status: "pass" }}) }})
export default prior.revise({{ author: "dev", why: "add doc", add: [doc] }})
"#
        ),
    );
    let ok = tmp_module(
        &vdir,
        "recon_ok.ts",
        &format!(
            r#"import {{ reconcile }} from "compass"
import sa from "./{add_a}"
import sb from "./{add_b}"
export default reconcile({{ revises: [sa, sb], author: "cos", why: "both are worth keeping" }})
"#
        ),
    );
    let map = eval::eval_plan_file(&ok)
        .unwrap_or_else(|e| panic!("a non-conflicting reconcile must work: {}", e.message()));
    let v = map.get(&std::fs::canonicalize(&ok).unwrap()).unwrap();
    let names: Vec<&str> = v.steps.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"build")
            && names.contains(&"ship")
            && names.contains(&"fuzz")
            && names.contains(&"doc"),
        "{names:?}"
    );
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

// ---- decision 0017: a Plan's identity is its origin ----

const DEMO_ORIGIN: &str = r#"import { plan, step, evidence } from "compass"
export const build = step({ work: "Build it", accept: evidence.test({ name: "t", status: "pass" }) })
export default plan({ author: "cos", goal: "Ship the widget", why: "It is time.", steps: [build] })
"#;

/// An origin commit files under its own hash — the origin's version-id and the
/// PlanRef are the same hash — and a revision inherits that same PlanRef.
#[test]
fn an_origin_files_under_its_own_hash_and_a_revision_shares_the_planref() {
    let root = std::env::temp_dir().join(format!("compass-0017-id-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let _g = Tmp(root.clone());
    let plan = &planref_of(DEMO_ORIGIN);

    let v1 = commit_module(&root, plan, DEMO_ORIGIN);
    assert_eq!(
        &v1[4..16],
        plan.as_str(),
        "an origin's version-id is its PlanRef"
    );
    assert!(catalog::plan_dir(&root, plan).is_dir());

    // A revision, authored as a sibling of its predecessor, inherits the Plan.
    let rev = format!(
        r#"import prior from "./{v1}"
export default prior.revise({{ author: "cos", why: "Reword.", edit: [prior.steps.build.with({{ work: "Build it, carefully" }})] }})
"#
    );
    let v2 = commit_module(&root, plan, &rev);
    let store = catalog::load_plan(&root, plan).unwrap();
    assert_eq!(
        store.versions.len(),
        2,
        "the revision files under the same PlanRef as its origin: {:?}",
        store.rejected
    );
    let head = chain::analyze(&store)
        .head
        .into_iter()
        .max_by_key(|a| a.seq)
        .unwrap()
        .clone();
    assert_eq!(head.path.file_name().unwrap().to_str().unwrap(), v2);
    assert_eq!(head.seq, 2, "the revision is seq 2 of the same lineage");
}

/// A version placed in the wrong plan dir is rejected — never reinterpreted into
/// the Plan it was filed under — on the same terms as a content-hash mismatch.
#[test]
fn a_version_in_the_wrong_plan_dir_is_rejected() {
    let root = std::env::temp_dir().join(format!("compass-0017-misfiled-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let _g = Tmp(root.clone());
    catalog::init(&root).unwrap();

    // File the origin under a directory that is not its own hash. The filename
    // still matches the content (so this is not tampering), but the dir does not.
    let wrong = "abcabcabcabc";
    assert_ne!(wrong, planref_of(DEMO_ORIGIN));
    admit(&root, wrong, 1, DEMO_ORIGIN);

    let store = catalog::load_plan(&root, wrong).unwrap();
    assert!(
        store.versions.is_empty(),
        "a misfiled version is not admitted"
    );
    assert_eq!(store.rejected.len(), 1);
    assert!(
        store.rejected[0].reason.contains("misfiled"),
        "the rejection must name the misfiling: {}",
        store.rejected[0].reason
    );

    // verify surfaces it as a problem, so the plan does not read clean.
    let out = run(
        &root,
        Command::Verify {
            plan: Some(wrong.into()),
            all: false,
        },
    )
    .unwrap();
    assert_ne!(out.code, 0, "a misfiled version fails verification");
}

/// A commit whose goal is empty is refused, and nothing is recorded (CMP.DM-R12).
#[test]
fn a_commit_with_an_empty_goal_is_refused() {
    let root = std::env::temp_dir().join(format!("compass-0017-goal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let _g = Tmp(root.clone());
    catalog::init(&root).unwrap();

    let empty_goal = r#"import { plan, step, evidence } from "compass"
export const a = step({ work: "x", accept: evidence.test({ status: "pass" }) })
export default plan({ author: "cos", goal: "", why: "w", steps: [a] })
"#;
    // An origin has no predecessor import, so it may be authored anywhere.
    let draft = tmp_module(&root, "empty-goal.ts", empty_goal);
    let err = match run(&root, Command::Commit { path: draft }) {
        Ok(o) => panic!("an empty goal must be refused, got: {}", o.text),
        Err(e) => e,
    };
    assert!(
        err.contains("goal") && err.contains("nothing was recorded"),
        "an empty goal must be refused with nothing recorded: {err}"
    );
    // The refusal recorded nothing: the plan dir does not exist.
    assert!(!catalog::plan_dir(&root, &planref_of(empty_goal)).exists());
}

/// `status` and `show` present a Plan by its goal, the human handle, not by its
/// raw hash (CMP.DM-R12).
#[test]
fn status_and_show_display_the_goal() {
    let root = std::env::temp_dir().join(format!("compass-0017-handle-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let _g = Tmp(root.clone());
    let plan = &planref_of(DEMO_ORIGIN);
    commit_module(&root, plan, DEMO_ORIGIN);

    let status = run(&root, Command::Status).unwrap();
    assert!(
        status.text.contains("Ship the widget"),
        "status shows the goal: {}",
        status.text
    );
    let show = run(&root, Command::Show { plan: plan.clone() }).unwrap();
    assert!(
        show.text.contains("Ship the widget"),
        "show shows the goal: {}",
        show.text
    );
}

/// `progress` and `evidence` reference the Plan by its goal (the human handle),
/// not the raw hash — the version is still cited by hash (CMP.DM-R12).
#[test]
fn progress_and_evidence_reference_the_plan_by_goal() {
    let root = std::env::temp_dir().join(format!("compass-0017-record-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let _g = Tmp(root.clone());
    let plan = &planref_of(DEMO_ORIGIN);
    commit_module(&root, plan, DEMO_ORIGIN);

    let prog = run(
        &root,
        Command::Progress {
            plan: plan.clone(),
            step: "build".into(),
            kind: "start".into(),
            note: None,
        },
    )
    .unwrap();
    assert!(
        prog.text.contains("Ship the widget"),
        "progress references the plan by goal: {}",
        prog.text
    );

    let ev = run(
        &root,
        Command::Evidence {
            plan: plan.clone(),
            step: "build".into(),
            kind: "test".into(),
            attrs: vec![
                ("name".into(), "t".into()),
                ("status".into(), "pass".into()),
            ],
        },
    )
    .unwrap();
    assert!(
        ev.text.contains("Ship the widget"),
        "evidence references the plan by goal: {}",
        ev.text
    );
}
