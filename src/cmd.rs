//! Command implementations.
//!
//! Every command produces its human rendering and its `--json` rendering from
//! the same values (04-cli), every read of Plan state reports convergence, and
//! a read that fails renders not-found / unresolved / stopped distinctly — each
//! naming a different next action.

use crate::catalog::{self, Admitted, PlanStore};
use crate::chain::{self, Analysis};
use crate::cli::{Command, Invocation};
use crate::convergence::Convergence;
use crate::eval::EvalError;
use crate::event::{Event, EventKind};
use crate::json::Json;
use crate::model::Version;
use crate::readiness::{self, HeadReadiness, StepState};
use crate::style;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A command's result, rendered two ways from one set of values.
pub struct Output {
    pub text: String,
    pub json: Json,
    pub code: i32,
}

impl Output {
    fn ok(text: String, json: Json) -> Output {
        Output {
            text,
            json,
            code: 0,
        }
    }
    fn coded(text: String, json: Json, code: i32) -> Output {
        Output { text, json, code }
    }
}

/// Dispatch.
pub fn execute(inv: &Invocation) -> Result<Output, String> {
    let root = resolved_root(inv)?;
    let author = inv.author.clone().unwrap_or_else(catalog::author);

    match &inv.command {
        Command::Help { topic } => Ok(Output::ok(
            crate::cli::help(topic.as_deref()),
            Json::obj(vec![("command", Json::str("help"))]),
        )),
        Command::Version => Ok(cmd_version()),
        Command::Start { plan, goal } => cmd_start(&root, plan, goal.as_deref(), &author),
        Command::Commit { path, plan } => cmd_commit(&root, path, plan.as_deref()),
        Command::Show { plan } => cmd_show(&root, plan),
        Command::History { plan } => cmd_history(&root, plan),
        Command::Ready { plan } => cmd_ready(&root, plan),
        Command::Status => cmd_status(&root),
        Command::Verify { plan, all } => cmd_verify(&root, plan.as_deref(), *all),
        Command::Repair { plan } => cmd_repair(&root, plan),
        Command::Progress {
            plan,
            step,
            kind,
            note,
        } => cmd_progress(&root, plan, step, kind, note.as_deref(), &author),
        Command::Evidence {
            plan,
            step,
            kind,
            attrs,
        } => cmd_evidence(&root, plan, step, kind, attrs, &author),
    }
}

pub fn resolved_root(inv: &Invocation) -> Result<PathBuf, String> {
    match &inv.catalog {
        Some(p) => Ok(p.clone()),
        None => catalog::root(),
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn convergence_line(c: &Convergence) -> String {
    format!("{} convergence: {}", style::dim("~"), c.describe())
}
fn convergence_json(c: &Convergence) -> (&'static str, Json) {
    (
        "convergence",
        Json::obj(vec![
            ("state", Json::str(c.state())),
            ("reason", Json::str(c.reason())),
        ]),
    )
}

fn load(root: &Path, plan: &str) -> Result<PlanStore, String> {
    if !catalog::exists(root) {
        return Err(format!(
            "no catalog at {}\n  run `compass start <plan>` to begin",
            root.display()
        ));
    }
    catalog::load_plan(root, plan)
}

/// Evaluate one admitted version to its intent, mapping the read failure modes.
fn evaluate(a: &Admitted) -> Result<Version, EvalError> {
    catalog::evaluate(a)
}

/// Render a failed read distinctly (04-cli): each names a different next action.
fn read_failure(plan: &str, e: &EvalError) -> Output {
    let next = match e {
        EvalError::Unresolved(_) => "wait: an imported version has not arrived",
        EvalError::Stopped(_) => "do not wait; the plan is at fault",
        EvalError::Failed(_) => "the plan does not evaluate",
    };
    let text = format!(
        "{} {}\n  {}\n  {}\n",
        style::red(e.kind()),
        plan,
        e.message(),
        style::dim(next),
    );
    let json = Json::obj(vec![
        ("command", Json::str("read")),
        ("plan", Json::str(plan)),
        ("status", Json::str(e.kind())),
        ("message", Json::str(e.message())),
    ]);
    Output::coded(text, json, crate::cli::EXIT_FAILURE)
}

fn not_found(plan: &str) -> Output {
    Output::coded(
        format!(
            "{} {plan}\n  no such plan here → look elsewhere\n",
            style::red("not found")
        ),
        Json::obj(vec![
            ("command", Json::str("read")),
            ("plan", Json::str(plan)),
            ("status", Json::str("not-found")),
        ]),
        crate::cli::EXIT_FAILURE,
    )
}

/// The intent of every version reachable from `head`, keyed by hash. Evaluating
/// a tip yields the whole lineage, so this is one evaluation.
fn graph_from(store: &PlanStore, head: &Admitted) -> Result<HashMap<String, Version>, EvalError> {
    let map = crate::eval::eval_plan_file(&head.path)?;
    let mut out = HashMap::new();
    for a in &store.versions {
        let canon = std::fs::canonicalize(&a.path).unwrap_or_else(|_| a.path.clone());
        if let Some(sem) = map.get(&canon).or_else(|| map.get(&a.path)) {
            out.insert(
                a.hash.clone(),
                Version::from_sem(&a.plan, a.seq, a.parents.clone(), sem),
            );
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// version
// ---------------------------------------------------------------------------

fn cmd_version() -> Output {
    let id = crate::version::identity();
    Output::ok(format!("{}\n", id.display_version), id.to_json())
}

// ---------------------------------------------------------------------------
// start
// ---------------------------------------------------------------------------

fn cmd_start(root: &Path, plan: &str, goal: Option<&str>, author: &str) -> Result<Output, String> {
    catalog::init(root)?;
    let dir = catalog::versions_dir(root, plan);
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let draft = dir.join("_new.ts");
    if draft.exists() {
        return Err(format!(
            "a draft already exists at {}\n  edit it and `compass commit {}`",
            draft.display(),
            draft.display()
        ));
    }
    let goal = goal.unwrap_or("State the goal here");
    let scaffold = starter_module(goal, author);
    std::fs::write(&draft, &scaffold)
        .map_err(|e| format!("cannot write {}: {e}", draft.display()))?;

    let text = format!(
        "scaffolded a starter plan for {}\n  edit   {}\n  commit compass commit {}\n",
        style::bold(plan),
        draft.display(),
        draft.display()
    );
    let json = Json::obj(vec![
        ("command", Json::str("start")),
        ("plan", Json::str(plan)),
        ("draft", Json::str(draft.to_string_lossy())),
    ]);
    Ok(Output::ok(text, json))
}

fn starter_module(goal: &str, author: &str) -> String {
    format!(
        r#"import {{ plan, step, evidence }} from "compass"

// A Step is a named binding; its name is its identity. A dependency is a
// reference to the binding — nothing to mistype, nothing to invent.
export const first = step({{
  work: "The first piece of work",
  accept: evidence.test({{ name: "the-check", status: "pass" }}),
}})

export default plan({{
  author: {author:?},
  goal: {goal:?},
  why: "Why this plan exists — the durable record of reasons.",
  steps: [first],
}})
"#
    )
}

// ---------------------------------------------------------------------------
// commit
// ---------------------------------------------------------------------------

fn cmd_commit(root: &Path, path: &Path, plan_opt: Option<&str>) -> Result<Output, String> {
    let source = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let source_str = String::from_utf8(source.clone())
        .map_err(|e| format!("{}: not valid UTF-8: {e}", path.display()))?;

    let plan = match plan_opt {
        Some(p) => p.to_string(),
        None => infer_plan(path)
            .ok_or_else(|| "cannot determine the plan; pass --plan <plan>".to_string())?,
    };

    // Evaluate the authored module (imports resolve at its location).
    let map = crate::eval::eval_plan_file(path).map_err(|e| {
        format!(
            "{}: {} ({})\n  nothing was recorded",
            path.display(),
            e.message(),
            e.kind()
        )
    })?;
    let canon = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let sem = map
        .get(&canon)
        .or_else(|| map.get(path))
        .ok_or_else(|| "the module did not export a plan".to_string())?;

    // Classify each version import (CMP.API-R05). An import of a version of the
    // SAME plan is a predecessor, resolved against this plan's store and made a
    // parent of the new version. An import of ANOTHER plan's version is a
    // cross-plan reference: it must resolve against that plan's store (the other
    // version must be admitted), but it is not a predecessor and does not make
    // this commit "have an uncommitted predecessor".
    let store = catalog::load_plan(root, &plan)?;
    let mut parents: Vec<String> = Vec::new();
    for spec in crate::eval::import_specifiers(&source_str, path)
        .map_err(|e| format!("cannot read imports: {}", e.message()))?
    {
        let file = spec.rsplit('/').next().unwrap_or(&spec);
        let Some((_seq, prefix)) = crate::model::parse_filename(file) else {
            continue; // e.g. the `compass` prelude — not a version reference.
        };
        // A target plan that differs from the current one is a cross-plan
        // reference; anything else (a sibling, or a version of this same plan)
        // is a predecessor.
        match crate::eval::import_target_plan(path, &spec) {
            Some(other) if other != plan => {
                let other_store = catalog::load_plan(root, &other)?;
                if other_store.resolve_hash(&prefix).is_none() {
                    return Err(format!(
                        "cross-plan reference {prefix} is not committed in {other}; \
                         nothing was recorded"
                    ));
                }
            }
            _ => match store.resolve_hash(&prefix) {
                Some(a) => parents.push(a.hash.clone()),
                None => {
                    return Err(format!(
                        "predecessor {prefix} is not committed in {plan}; nothing was recorded"
                    ))
                }
            },
        }
    }
    parents.sort();
    parents.dedup();

    let parent_admitted: Vec<&Admitted> = parents.iter().filter_map(|h| store.version(h)).collect();
    let seq = chain::next_seq(&parent_admitted);

    let version = Version::from_sem(&plan, seq, parents.clone(), sem);
    version
        .validate()
        .map_err(|e| format!("{e}; nothing was recorded"))?;

    // A criterion that contradicts itself is refused when written (0013).
    for s in &version.steps {
        if let Some(atom) = s.accept.self_contradiction() {
            return Err(format!(
                "step {} has an acceptance criterion that contradicts itself: it both requires \
                 and forbids {atom}; nothing was recorded",
                s.id
            ));
        }
    }

    let hash = crate::sha256::sha256_hex(&source);
    let filename = crate::model::filename_for(seq, &hash);
    let target = catalog::versions_dir(root, &plan).join(&filename);

    // A repeat is a repeat: identical content already committed is a no-op success.
    if target.exists() {
        let text = format!(
            "{} {} already committed as {}\n",
            style::dim("="),
            style::bold(&plan),
            style::short(&hash)
        );
        return Ok(Output::ok(
            text,
            receipt_json("already-committed", &plan, &hash, seq, &parents),
        ));
    }

    // New content that revises nothing is refused, with a distinct message.
    if parent_admitted.len() == 1 {
        if let Ok(pv) = evaluate(parent_admitted[0]) {
            if version.changes_nothing_from(&pv) {
                return Err(
                    "this changes nothing: a revision must alter a step or the goal \
                     (CMP.DM-R07b); nothing was recorded"
                        .to_string(),
                );
            }
        }
    }

    let (out_path, hash, _created) = catalog::write_version(root, &plan, seq, &source)?;

    // A draft authored in place is superseded by its hash-named form.
    if path.parent() == Some(catalog::versions_dir(root, &plan).as_path())
        && path.file_name() != out_path.file_name()
    {
        let _ = std::fs::remove_file(path);
    }

    let kind = if parents.is_empty() {
        "created"
    } else if parents.len() > 1 {
        "reconciled"
    } else {
        "revised"
    };
    let text = format!(
        "{} {} {} as {} (seq {})\n{}\n",
        style::green("committed"),
        kind,
        style::bold(&plan),
        style::short(&hash),
        seq,
        style::dim(&format!("  {}", out_path.display()))
    );
    Ok(Output::ok(
        text,
        receipt_json(kind, &plan, &hash, seq, &parents),
    ))
}

fn receipt_json(kind: &str, plan: &str, hash: &str, seq: u64, parents: &[String]) -> Json {
    Json::obj(vec![
        ("command", Json::str("commit")),
        ("result", Json::str(kind)),
        ("plan", Json::str(plan)),
        ("version", Json::str(hash)),
        ("seq", Json::num(seq as i64)),
        ("parents", Json::strs(parents.iter().cloned())),
    ])
}

/// Infer the plan ref from a path under `plans/<plan>/versions/`.
fn infer_plan(path: &Path) -> Option<String> {
    let versions = path.parent()?;
    if versions.file_name()?.to_str()? != "versions" {
        return None;
    }
    let plan_dir = versions.parent()?;
    Some(plan_dir.file_name()?.to_str()?.to_string())
}

// ---------------------------------------------------------------------------
// show
// ---------------------------------------------------------------------------

fn cmd_show(root: &Path, plan: &str) -> Result<Output, String> {
    let store = load(root, plan)?;
    if store.versions.is_empty() && store.rejected.is_empty() {
        return Ok(not_found(plan));
    }
    let an = chain::analyze(&store);
    let c = Convergence::probe();

    // Evaluate against the first head so the whole lineage is available; a read
    // failure at the frontier is rendered distinctly.
    let mut text = String::new();
    let mut heads_json: Vec<Json> = Vec::new();

    for (i, h) in an.head.iter().enumerate() {
        let orphan = an.is_orphan(&h.hash);
        match evaluate(h) {
            Ok(v) => {
                let label = if an.head.len() > 1 {
                    format!("{} head {}/{} ", style::yellow("◆"), i + 1, an.head.len())
                } else {
                    String::new()
                };
                text.push_str(&format!(
                    "{}{} {} seq {} by {}{}\n",
                    label,
                    style::bold(&style::short(&h.hash)),
                    if orphan {
                        style::red("(orphan)")
                    } else {
                        String::new()
                    },
                    v.seq,
                    v.author,
                    if v.retired {
                        style::red(" [retired]")
                    } else {
                        String::new()
                    },
                ));
                text.push_str(&format!("  goal: {}\n", v.goal));
                text.push_str(&style::wrapped_block(&v.why, "  ", 78));
                for s in &v.steps {
                    let mark = if s.retired {
                        style::dim("⊘")
                    } else {
                        style::dim("•")
                    };
                    text.push_str(&format!(
                        "  {} {} — {}\n",
                        mark,
                        style::bold(&s.id),
                        style::truncate(&s.work, 64)
                    ));
                }
                heads_json.push(head_json(&h.hash, orphan, &v));
            }
            Err(e) => {
                text.push_str(&read_failure(plan, &e).text);
                heads_json.push(Json::obj(vec![
                    ("head", Json::str(&h.hash)),
                    ("status", Json::str(e.kind())),
                    ("message", Json::str(e.message())),
                ]));
            }
        }
        text.push('\n');
    }

    text.push_str(&divergence_report(&an));
    text.push_str(&convergence_line(&c));
    text.push('\n');

    let json = Json::obj(vec![
        ("command", Json::str("show")),
        ("plan", Json::str(plan)),
        ("heads", Json::arr(heads_json)),
        ("diverged", Json::Bool(an.diverged())),
        convergence_json(&c),
    ]);
    Ok(Output::ok(text, json))
}

fn head_json(hash: &str, orphan: bool, v: &Version) -> Json {
    Json::obj(vec![
        ("version", Json::str(hash)),
        ("seq", Json::num(v.seq as i64)),
        ("author", Json::str(&v.author)),
        ("why", Json::str(&v.why)),
        ("goal", Json::str(&v.goal)),
        ("retired", Json::Bool(v.retired)),
        ("orphan", Json::Bool(orphan)),
        ("steps", Json::arr(v.steps.iter().map(step_json).collect())),
    ])
}

fn step_json(s: &crate::model::Step) -> Json {
    Json::obj(vec![
        ("id", Json::str(&s.id)),
        ("work", Json::str(&s.work)),
        ("dependsOn", Json::strs(s.depends_on.iter().cloned())),
        (
            "supersedes",
            match &s.supersedes {
                Some(x) => Json::str(x),
                None => Json::Null,
            },
        ),
        ("accept", Json::str(s.accept.to_string())),
        ("retired", Json::Bool(s.retired)),
    ])
}

fn divergence_report(an: &Analysis) -> String {
    let mut out = String::new();
    for o in &an.orphans {
        out.push_str(&format!(
            "{} {} is an orphan: predecessor {} has not arrived — wait\n",
            style::warning(),
            style::short(&o.version.hash),
            o.missing
                .iter()
                .map(|m| style::short(m))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    for d in an.open_divergences() {
        out.push_str(&format!(
            "{} open divergence: {} head members share a predecessor — reconcile by authoring \
             a version importing both\n",
            style::warning(),
            d.children.len()
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// history
// ---------------------------------------------------------------------------

fn cmd_history(root: &Path, plan: &str) -> Result<Output, String> {
    let store = load(root, plan)?;
    if store.versions.is_empty() {
        return Ok(not_found(plan));
    }
    let an = chain::analyze(&store);
    let c = Convergence::probe();

    let mut text = String::new();
    let mut entries: Vec<Json> = Vec::new();

    for h in &an.head {
        let graph = match graph_from(&store, h) {
            Ok(g) => g,
            Err(e) => {
                return Ok(read_failure(plan, &e));
            }
        };
        if an.head.len() > 1 {
            text.push_str(&format!(
                "{} lineage of head {}\n",
                style::yellow("◆"),
                style::short(&h.hash)
            ));
        }
        for a in chain::lineage(&store, &h.hash) {
            let Some(v) = graph.get(&a.hash) else {
                continue;
            };
            text.push_str(&format!(
                "  {} seq {} by {}\n",
                style::bold(&style::short(&a.hash)),
                v.seq,
                v.author
            ));
            text.push_str(&style::wrapped_block(&v.why, "    ", 78));
            entries.push(Json::obj(vec![
                ("version", Json::str(&a.hash)),
                ("seq", Json::num(v.seq as i64)),
                ("author", Json::str(&v.author)),
                ("why", Json::str(&v.why)),
            ]));
        }
    }
    text.push_str(&convergence_line(&c));
    text.push('\n');

    let json = Json::obj(vec![
        ("command", Json::str("history")),
        ("plan", Json::str(plan)),
        ("rationale", Json::arr(entries)),
        convergence_json(&c),
    ]);
    Ok(Output::ok(text, json))
}

// ---------------------------------------------------------------------------
// ready
// ---------------------------------------------------------------------------

fn cmd_ready(root: &Path, plan: &str) -> Result<Output, String> {
    let store = load(root, plan)?;
    if store.versions.is_empty() {
        return Ok(not_found(plan));
    }
    let an = chain::analyze(&store);
    let c = Convergence::probe();

    let mut text = String::new();
    let mut heads_json: Vec<Json> = Vec::new();

    for h in &an.head {
        let orphan = an.is_orphan(&h.hash);
        let v = match evaluate(h) {
            Ok(v) => v,
            Err(e) => return Ok(read_failure(plan, &e)),
        };
        let r = readiness::for_head(&store.events, &v, &h.hash, orphan);
        if an.head.len() > 1 {
            text.push_str(&format!(
                "{} head {} (by {})\n",
                style::yellow("◆"),
                style::short(&h.hash),
                r.author
            ));
        }
        if orphan {
            text.push_str(&format!(
                "  {} provisional: this head is an orphan\n",
                style::dim("note:")
            ));
        }
        render_readiness(&mut text, &r);
        heads_json.push(readiness_json(&r, orphan));
    }
    text.push_str(&convergence_line(&c));
    text.push('\n');

    let json = Json::obj(vec![
        ("command", Json::str("ready")),
        ("plan", Json::str(plan)),
        ("heads", Json::arr(heads_json)),
        convergence_json(&c),
    ]);
    Ok(Output::ok(text, json))
}

fn render_readiness(text: &mut String, r: &HeadReadiness) {
    let ready = r.count(StepState::Ready);
    if ready == 0 {
        text.push_str(&format!("  {}\n", style::dim("nothing is ready right now")));
    }
    for s in &r.steps {
        let mark = match s.state {
            StepState::Ready => style::green("▶"),
            StepState::Blocked => style::dim("·"),
            StepState::Accepted => style::green("✓"),
            StepState::Retired => style::dim("⊘"),
        };
        text.push_str(&format!(
            "  {} {} {} — {}\n",
            mark,
            style::bold(&s.step),
            style::dim(s.state.as_str()),
            s.reason
        ));
    }
}

fn readiness_json(r: &HeadReadiness, orphan: bool) -> Json {
    Json::obj(vec![
        ("head", Json::str(&r.head)),
        ("seq", Json::num(r.seq as i64)),
        ("author", Json::str(&r.author)),
        ("orphan", Json::Bool(orphan)),
        (
            "steps",
            Json::arr(
                r.steps
                    .iter()
                    .map(|s| {
                        Json::obj(vec![
                            ("step", Json::str(&s.step)),
                            ("state", Json::str(s.state.as_str())),
                            ("reason", Json::str(&s.reason)),
                            ("blockedBy", Json::strs(s.blocked_by.iter().cloned())),
                            ("accept", Json::str(&s.accept)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

// ---------------------------------------------------------------------------
// status
// ---------------------------------------------------------------------------

fn cmd_status(root: &Path) -> Result<Output, String> {
    let c = Convergence::probe();
    let plans = if catalog::exists(root) {
        catalog::list_plans(root)?
    } else {
        Vec::new()
    };
    let mut text = String::new();
    let mut rows: Vec<Json> = Vec::new();

    if plans.is_empty() {
        text.push_str(&format!("{}\n", style::dim("no plans yet")));
    }
    for plan in &plans {
        let store = catalog::load_plan(root, plan)?;
        let an = chain::analyze(&store);
        text.push_str(&format!(
            "{}  {}  {}, {}\n",
            style::bold(plan),
            an.state(),
            style::count(store.versions.len(), "version"),
            style::count(an.head.len(), "head"),
        ));
        rows.push(Json::obj(vec![
            ("plan", Json::str(plan)),
            ("state", Json::str(an.state())),
            ("versions", Json::num(store.versions.len() as i64)),
            ("heads", Json::num(an.head.len() as i64)),
            ("diverged", Json::Bool(an.diverged())),
        ]));
    }
    text.push_str(&convergence_line(&c));
    text.push('\n');

    let json = Json::obj(vec![
        ("command", Json::str("status")),
        ("plans", Json::arr(rows)),
        convergence_json(&c),
    ]);
    Ok(Output::ok(text, json))
}

// ---------------------------------------------------------------------------
// verify
// ---------------------------------------------------------------------------

fn cmd_verify(root: &Path, plan: Option<&str>, all: bool) -> Result<Output, String> {
    let plans = if all {
        catalog::list_plans(root)?
    } else {
        vec![plan.unwrap().to_string()]
    };
    let mut problems: Vec<Json> = Vec::new();
    let mut text = String::new();
    let mut clean = true;

    for plan in &plans {
        let store = load(root, plan)?;
        let an = chain::analyze(&store);
        for r in &store.rejected {
            clean = false;
            text.push_str(&format!(
                "{} {}: {} — {}\n",
                style::critical(),
                plan,
                file_name(&r.path),
                r.reason
            ));
            problems.push(problem_json(
                plan,
                "rejected",
                &file_name(&r.path),
                &r.reason,
            ));
        }
        for o in &an.orphans {
            clean = false;
            let reason = format!(
                "predecessor {} not present",
                o.missing
                    .iter()
                    .map(|m| style::short(m))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            text.push_str(&format!(
                "{} {}: {} is an orphan — {}\n",
                style::warning(),
                plan,
                style::short(&o.version.hash),
                reason
            ));
            problems.push(problem_json(
                plan,
                "orphan",
                &style::short(&o.version.hash),
                &reason,
            ));
        }
        // A frontier that will not evaluate is damage the chain cannot show.
        for h in &an.head {
            if let Err(e) = evaluate(h) {
                if matches!(
                    e,
                    EvalError::Unresolved(_) | EvalError::Failed(_) | EvalError::Stopped(_)
                ) {
                    clean = false;
                    text.push_str(&format!(
                        "{} {}: head {} is {} — {}\n",
                        style::critical(),
                        plan,
                        style::short(&h.hash),
                        e.kind(),
                        e.message()
                    ));
                    problems.push(problem_json(
                        plan,
                        e.kind(),
                        &style::short(&h.hash),
                        e.message(),
                    ));
                }
            }
        }
    }
    if clean {
        text.push_str(&format!("{} everything verifies\n", style::green("ok")));
    }
    let json = Json::obj(vec![
        ("command", Json::str("verify")),
        ("clean", Json::Bool(clean)),
        ("problems", Json::arr(problems)),
    ]);
    Ok(Output::coded(
        text,
        json,
        if clean { 0 } else { crate::cli::EXIT_FAILURE },
    ))
}

fn problem_json(plan: &str, kind: &str, subject: &str, reason: &str) -> Json {
    Json::obj(vec![
        ("plan", Json::str(plan)),
        ("kind", Json::str(kind)),
        ("subject", Json::str(subject)),
        ("reason", Json::str(reason)),
    ])
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_string()
}

// ---------------------------------------------------------------------------
// repair — distinct from verify (04-cli): it authors permanent content.
// ---------------------------------------------------------------------------

fn cmd_repair(root: &Path, plan: &str) -> Result<Output, String> {
    let store = load(root, plan)?;
    let an = chain::analyze(&store);

    let damaged = !store.rejected.is_empty() || an.head.iter().any(|h| evaluate(h).is_err());
    if !damaged {
        // Repair refuses when verification finds nothing wrong, so the
        // irreversible operation is never one keystroke from the safe one.
        return Err(format!(
            "nothing to repair in {plan}: verification reports no damage. \
             Run `compass verify {plan}` to confirm."
        ));
    }

    // Identify the last intact predecessor to continue from.
    let intact: Vec<&Admitted> = store
        .versions
        .iter()
        .filter(|a| evaluate(a).is_ok())
        .collect();
    let base = intact.iter().max_by_key(|a| a.seq);

    let mut text = format!(
        "{} {} has damage that must be repaired by authoring, not editing\n",
        style::critical(),
        style::bold(plan)
    );
    for r in &store.rejected {
        text.push_str(&format!(
            "  unverifiable: {} — {}\n",
            file_name(&r.path),
            r.reason
        ));
    }
    match base {
        Some(b) => {
            let rel = crate::model::filename_for(b.seq, &b.hash);
            text.push_str(&format!(
                "\nAuthor a damage-recording version continuing from the last intact predecessor \
                 ({}):\n\n  import prior from \"./{}\"\n  export default prior.revise({{\n    \
                 author: \"you\",\n    why: \"Records the damage to <hash> and continues.\",\n  \
                 }})\n\nthen `compass commit` it. Verification stays read-only.\n",
                style::short(&b.hash),
                rel
            ));
        }
        None => {
            text.push_str(
                "\nNo intact predecessor remains; author a fresh plan recording what is known \
                 of the lost intent.\n",
            );
        }
    }
    let json = Json::obj(vec![
        ("command", Json::str("repair")),
        ("plan", Json::str(plan)),
        ("damaged", Json::Bool(true)),
        (
            "lastIntact",
            match base {
                Some(b) => Json::str(&b.hash),
                None => Json::Null,
            },
        ),
    ]);
    Ok(Output::ok(text, json))
}

// ---------------------------------------------------------------------------
// progress / evidence
// ---------------------------------------------------------------------------

/// Find the single head whose version contains `step`, evaluated.
fn head_for_step<'a>(an: &Analysis<'a>, step: &str) -> Result<(&'a Admitted, Version), String> {
    let mut hits = Vec::new();
    for h in &an.head {
        if let Ok(v) = evaluate(h) {
            if v.step(step).is_some() {
                hits.push((*h, v));
            }
        }
    }
    match hits.len() {
        0 => Err(format!("no head of this plan has a step named `{step}`")),
        1 => Ok(hits.into_iter().next().unwrap()),
        n => Err(format!(
            "step `{step}` appears in {n} divergent heads; reconcile first so a record is \
             unambiguous"
        )),
    }
}

fn cmd_progress(
    root: &Path,
    plan: &str,
    step: &str,
    kind: &str,
    note: Option<&str>,
    author: &str,
) -> Result<Output, String> {
    let store = load(root, plan)?;
    let an = chain::analyze(&store);
    let (head, _v) = head_for_step(&an, step)?;

    let ekind = EventKind::parse(kind).ok_or_else(|| format!("unknown progress kind `{kind}`"))?;
    let event = Event {
        id: crate::refs::event_id()?,
        at: store.next_event_at(),
        wall: crate::event::now_wall(),
        plan: plan.to_string(),
        step: step.to_string(),
        version: head.hash.clone(),
        actor: author.to_string(),
        kind: ekind,
        note: note.map(|s| s.to_string()),
        evidence_kind: None,
        attrs: vec![],
    };
    let path = catalog::write_event(root, &event)?;
    let text = format!(
        "{} {} {} on {} (against {})\n",
        style::green("recorded"),
        kind,
        style::bold(step),
        style::bold(plan),
        style::short(&head.hash)
    );
    let json = Json::obj(vec![
        ("command", Json::str("progress")),
        ("plan", Json::str(plan)),
        ("step", Json::str(step)),
        ("kind", Json::str(kind)),
        ("actor", Json::str(author)),
        ("version", Json::str(&head.hash)),
        ("event", Json::str(&event.id)),
        ("path", Json::str(path.to_string_lossy())),
    ]);
    Ok(Output::ok(text, json))
}

fn cmd_evidence(
    root: &Path,
    plan: &str,
    step: &str,
    kind: &str,
    attrs: &[(String, String)],
    author: &str,
) -> Result<Output, String> {
    let store = load(root, plan)?;
    let an = chain::analyze(&store);
    let (head, version) = head_for_step(&an, step)?;

    // Recorded fields are reserved: an attribute shadowing one is refused (0008).
    if let Some(k) = Event::shadowing_attr(&sorted(attrs)) {
        return Err(format!(
            "attribute `{k}` shadows the recorded field `{k}`, which a predicate term reads as \
             what Compass recorded (decision 0008); nothing was recorded"
        ));
    }

    // Cross-check against the step's own criterion, and report a likely mistake
    // without refusing (decision 0013).
    let mut warning = None;
    if let Some(s) = version.step(step) {
        warning = crate::predicate::cross_check(kind, attrs, &[&s.accept]);
    }

    let event = Event {
        id: crate::refs::event_id()?,
        at: store.next_event_at(),
        wall: crate::event::now_wall(),
        plan: plan.to_string(),
        step: step.to_string(),
        version: head.hash.clone(),
        actor: author.to_string(),
        kind: EventKind::Evidence,
        note: None,
        evidence_kind: Some(kind.to_string()),
        attrs: sorted(attrs),
    };
    let path = catalog::write_event(root, &event)?;

    let mut text = format!(
        "{} evidence {} on {} (against {})\n",
        style::green("recorded"),
        style::bold(step),
        style::bold(plan),
        style::short(&head.hash)
    );
    if let Some(w) = &warning {
        text.push_str(&format!("  {} {}\n", style::yellow("note:"), w));
    }
    let json = Json::obj(vec![
        ("command", Json::str("evidence")),
        ("plan", Json::str(plan)),
        ("step", Json::str(step)),
        ("evidenceKind", Json::str(kind)),
        ("actor", Json::str(author)),
        ("version", Json::str(&head.hash)),
        ("event", Json::str(&event.id)),
        (
            "attrs",
            Json::Obj(
                sorted(attrs)
                    .into_iter()
                    .map(|(k, v)| (k, Json::str(v)))
                    .collect(),
            ),
        ),
        (
            "warning",
            match &warning {
                Some(w) => Json::str(w),
                None => Json::Null,
            },
        ),
        ("path", Json::str(path.to_string_lossy())),
    ]);
    Ok(Output::ok(text, json))
}

fn sorted(attrs: &[(String, String)]) -> Vec<(String, String)> {
    let mut v = attrs.to_vec();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}
