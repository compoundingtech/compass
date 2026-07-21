//! Command implementations.
//!
//! Every command produces its human rendering and its `--json` rendering from
//! the same computed values, so the two cannot drift.
//!
//! Every command reports convergence state alongside its answer (CMP-R17),
//! and every command that reports Head handles a Head set larger than one
//! without erroring and labels the members. `help` is the sole exception: it
//! answers about the CLI itself and touches no catalog.

use crate::catalog::{self, Admitted, PlanStore};
use crate::chain::{self, Analysis};
use crate::change::{self, Basis, StepChange, VersionChange};
use crate::cli::{Command, Invocation, StepEdit, EXIT_FAILURE};
use crate::convergence::Convergence;
use crate::event::{Event, EventKind};
use crate::json::Json;
use crate::model::{Step, Version};
use crate::predicate;
use crate::readiness::{self, HeadReadiness, StepState};
use crate::refs::{self, RefKind};
use crate::style as s;
use crate::version as build;
use std::path::{Path, PathBuf};

#[derive(Debug)]
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
}

/// Run one invocation.
pub fn execute(inv: &Invocation) -> Result<Output, String> {
    let root = match &inv.catalog {
        Some(p) => p.clone(),
        None => catalog::root()?,
    };
    let author = inv.author.clone().unwrap_or_else(catalog::author);

    match &inv.command {
        Command::Help { topic } => Ok(Output::ok(
            crate::cli::help(topic.as_deref()),
            Json::obj(vec![(
                "help",
                Json::str(crate::cli::help(topic.as_deref())),
            )]),
        )),
        Command::Version => Ok(cmd_version()),
        Command::Init => cmd_init(&root),
        Command::New { goal, why, steps } => cmd_new(&root, &author, goal, why, steps),
        Command::Revise {
            plan,
            why,
            goal,
            retire,
            steps,
        } => cmd_revise(&root, &author, plan, why, goal.as_deref(), *retire, steps),
        Command::Show { plan } => cmd_show(&root, plan),
        Command::Ready { plan } => cmd_ready(&root, plan),
        Command::Progress {
            plan,
            step,
            kind,
            note,
        } => cmd_progress(&root, &author, plan, step, kind, note.as_deref()),
        Command::Evidence {
            plan,
            step,
            kind,
            attrs,
        } => cmd_evidence(&root, &author, plan, step, kind, attrs),
        Command::Status => cmd_status(&root),
        Command::Reconcile {
            plan,
            why,
            from,
            steps,
        } => cmd_reconcile(&root, &author, plan, why, from.as_deref(), steps),
        Command::Verify { plan, all } => cmd_verify(&root, plan.as_deref(), *all),
    }
}

// ---------------------------------------------------------------------------
// Shared pieces
// ---------------------------------------------------------------------------

fn convergence_json(c: &Convergence) -> Json {
    Json::obj(vec![
        ("state", Json::str(c.state())),
        ("reason", Json::str(c.reason())),
        ("converged", Json::Bool(c.is_converged())),
    ])
}

fn convergence_line(c: &Convergence) -> String {
    format!("{} {}", s::dim("convergence:"), s::dim(&c.describe()))
}

fn load(root: &Path, plan: &str) -> Result<PlanStore, String> {
    if !catalog::exists(root) {
        return Err(format!(
            "no catalog at {}\n  fix: compass init",
            root.display()
        ));
    }
    if !catalog::plan_dir(root, plan).is_dir() {
        return Err(format!(
            "no plan {plan} in {}\n  fix: compass status",
            root.display()
        ));
    }
    catalog::load_plan(root, plan)
}

/// Describe a head member on one line.
fn head_line(a: &Admitted, index: usize, total: usize, orphan: bool) -> String {
    let label = if total > 1 {
        format!("head {}/{}", index + 1, total)
    } else {
        "head".to_string()
    };
    let mark = if orphan {
        format!(" {}", s::red("orphan"))
    } else {
        String::new()
    };
    format!(
        "  {} {}  {}  {}{}",
        s::bold(&label),
        s::bold(&s::short(&a.hash)),
        a.version.author,
        s::dim(&format!("seq={}", a.version.seq)),
        mark
    )
}

fn head_json(a: &Admitted, orphan: bool) -> Json {
    Json::obj(vec![
        ("version", Json::str(&a.hash)),
        ("plan", Json::str(&a.version.plan)),
        ("seq", Json::num(a.version.seq as i64)),
        ("author", Json::str(&a.version.author)),
        ("why", Json::str(&a.version.why)),
        ("goal", Json::str(&a.version.goal)),
        ("retired", Json::Bool(a.version.retired)),
        ("orphan", Json::Bool(orphan)),
        ("parent", Json::strs(a.version.parents.clone())),
    ])
}

/// A head member including its step graph.
///
/// `--json` must carry the same fields as the human rendering, and the human
/// rendering of `show` lists the steps at each head member — so the JSON has
/// to as well, or an agent cannot discover a StepRef without scraping text.
fn head_json_with_steps(a: &Admitted, orphan: bool) -> Json {
    let Json::Obj(mut fields) = head_json(a, orphan) else {
        unreachable!("head_json builds an object")
    };
    fields.push((
        "step".to_string(),
        Json::arr(a.version.steps.iter().map(step_json).collect()),
    ));
    Json::Obj(fields)
}

/// The problems block: rejected files, orphans, divergence.
///
/// Problems come first and each carries its own repair. Crucially, the repair
/// offered for an Orphan is *waiting*, never reconciliation — reconciling
/// around a version that is merely in flight writes permanent intent to fix a
/// transient condition.
fn problems_block(plan: &str, store: &PlanStore, an: &Analysis) -> String {
    let mut out = String::new();

    if !store.rejected.is_empty() || !store.bad_events.is_empty() {
        out.push_str(&format!(
            "{}  {} file(s) rejected — not adopted as state\n\n",
            s::critical(),
            store.rejected.len() + store.bad_events.len()
        ));
        for r in store.rejected.iter().chain(store.bad_events.iter()).take(5) {
            out.push_str(&format!(
                "  {} {}\n",
                s::bold(&file_name(&r.path)),
                s::dim(&r.reason)
            ));
            out.push_str(&s::note(
                "a file becomes state only in its expected location with a name matching its content (CMP-R22)",
            ));
            out.push('\n');
        }
        let extra = (store.rejected.len() + store.bad_events.len()).saturating_sub(5);
        if extra > 0 {
            out.push_str(&s::dim(&format!("  + {extra} more\n")));
        }
        out.push('\n');
    }

    if !an.orphans.is_empty() {
        out.push_str(&format!(
            "{}  {} orphaned version(s) — a predecessor has not arrived\n\n",
            s::warning(),
            an.orphans.len()
        ));
        for o in an.orphans.iter().take(5) {
            out.push_str(&format!(
                "  {} {}\n",
                s::bold(&s::short(&o.version.hash)),
                s::dim(&format!(
                    "missing predecessor {}",
                    o.missing
                        .iter()
                        .map(|h| s::short(h))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            ));
            out.push_str(&s::note(
                "an orphan is replication being incomplete, not intent disagreeing; it is repaired by waiting, not by reconciling",
            ));
            out.push('\n');
        }
        out.push('\n');
    }

    if an.diverged() {
        out.push_str(&format!(
            "{}  intent diverged — {} share a predecessor\n\n",
            s::warning(),
            s::count(
                an.divergences
                    .iter()
                    .map(|d| d.children.len())
                    .sum::<usize>(),
                "version"
            )
        ));
        for d in an.divergences.iter().take(3) {
            let parent = d
                .parent
                .as_deref()
                .map(s::short)
                .unwrap_or_else(|| "(no predecessor — several origins)".to_string());
            out.push_str(&format!("  {} {}\n", s::bold("from"), s::dim(&parent)));
            // The Rationale is shown in full, wrapped. Under divergence this
            // text is what the operator reads to decide the reconciliation,
            // so eliding it would hide the reasoning precisely where the
            // decision is being made (CMP-R04).
            for c in d.children.iter().take(5) {
                out.push_str(&format!("    {} {}\n", s::short(&c.hash), c.version.author,));
                out.push_str(&s::wrapped_block(&c.version.why, "      ", 78));
            }
            out.push_str(&s::fix(&format!("compass reconcile {plan} --why <text>")));
            out.push('\n');
        }
        out.push('\n');
    }

    out
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<unnamed>")
        .to_string()
}

fn problems_json(store: &PlanStore, an: &Analysis) -> Vec<(&'static str, Json)> {
    vec![
        (
            "rejected",
            Json::arr(
                store
                    .rejected
                    .iter()
                    .chain(store.bad_events.iter())
                    .map(|r| {
                        Json::obj(vec![
                            ("path", Json::str(r.path.display().to_string())),
                            ("reason", Json::str(&r.reason)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "orphans",
            Json::arr(
                an.orphans
                    .iter()
                    .map(|o| {
                        Json::obj(vec![
                            ("version", Json::str(&o.version.hash)),
                            ("missing_parent", Json::strs(o.missing.clone())),
                            ("repair", Json::str("wait for replication")),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "divergences",
            Json::arr(
                an.divergences
                    .iter()
                    .map(|d| {
                        Json::obj(vec![
                            (
                                "parent",
                                match &d.parent {
                                    Some(p) => Json::str(p),
                                    None => Json::Null,
                                },
                            ),
                            (
                                "children",
                                Json::arr(
                                    d.children
                                        .iter()
                                        .map(|c| {
                                            Json::obj(vec![
                                                ("version", Json::str(&c.hash)),
                                                ("author", Json::str(&c.version.author)),
                                                ("why", Json::str(&c.version.why)),
                                            ])
                                        })
                                        .collect(),
                                ),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
    ]
}

// ---------------------------------------------------------------------------
// Step edits
// ---------------------------------------------------------------------------

/// Resolve a `--depends-on` value.
///
/// Steps added in this same invocation do not have refs the operator could
/// know in advance, since refs are minted rather than derived. `@N` names the
/// Nth step added by this command, 1-based.
fn resolve_dep(value: &str, minted: &[String]) -> Result<String, String> {
    if refs::is_valid(value, RefKind::Step) {
        return Ok(value.to_string());
    }
    let index = value.strip_prefix('@').unwrap_or(value);
    if let Ok(n) = index.parse::<usize>() {
        if n >= 1 && n <= minted.len() {
            return Ok(minted[n - 1].clone());
        }
        return Err(format!(
            "`--depends-on {value}` names step #{n}, but this command adds {} step(s)",
            minted.len()
        ));
    }
    Err(format!(
        "`--depends-on {value}` is neither a StepRef (st_…) nor `@N` naming a step added here"
    ))
}

/// Apply step edits to a base step list, producing the next version's steps.
fn apply_edits(base: &[Step], edits: &[StepEdit]) -> Result<Vec<Step>, String> {
    // Refs must be minted up front so `@N` can be resolved while building.
    let mut minted: Vec<String> = Vec::new();
    for e in edits {
        if matches!(e, StepEdit::Add { .. }) {
            minted.push(refs::mint(RefKind::Step)?);
        }
    }

    let mut steps: Vec<Step> = base.to_vec();
    let mut next_minted = minted.iter();

    for e in edits {
        match e {
            StepEdit::Add {
                work,
                accept,
                depends_on,
                supersedes,
            } => {
                let id = next_minted.next().expect("minted one ref per Add").clone();
                let accept_src = accept.as_deref().ok_or_else(|| {
                    format!(
                        "step `{work}` needs `--accept <predicate>`\n  \
                         acceptance must be machine-checkable (CMP-R11); see `compass help predicates`"
                    )
                })?;
                let accept = predicate::parse(accept_src)
                    .map_err(|e| format!("cannot parse `--accept {accept_src}`: {e}"))?;
                let mut deps = Vec::new();
                for d in depends_on {
                    deps.push(resolve_dep(d, &minted)?);
                }
                deps.sort();
                deps.dedup();
                steps.push(Step {
                    id,
                    work: work.clone(),
                    depends_on: deps,
                    supersedes: supersedes.clone(),
                    accept,
                    retired: false,
                });
            }
            StepEdit::Edit {
                id,
                work,
                accept,
                depends_on,
                supersedes,
            } => {
                let target = steps
                    .iter_mut()
                    .find(|s| s.id == *id)
                    .ok_or_else(|| format!("no step {id} at head"))?;
                if let Some(w) = work {
                    target.work = w.clone();
                }
                if let Some(a) = accept {
                    target.accept = predicate::parse(a)
                        .map_err(|e| format!("cannot parse `--accept {a}`: {e}"))?;
                }
                if let Some(d) = depends_on {
                    let mut deps = Vec::new();
                    for one in d {
                        deps.push(resolve_dep(one, &minted)?);
                    }
                    deps.sort();
                    deps.dedup();
                    target.depends_on = deps;
                }
                if let Some(sup) = supersedes {
                    target.supersedes = Some(sup.clone());
                }
            }
            StepEdit::Retire { id } => {
                let target = steps
                    .iter_mut()
                    .find(|s| s.id == *id)
                    .ok_or_else(|| format!("no step {id} at head"))?;
                target.retired = true;
            }
        }
    }

    Ok(steps)
}

/// Write a version and render the standard "wrote a version" answer.
fn report_write(
    root: &Path,
    v: &Version,
    action: &str,
    conv: &Convergence,
) -> Result<Output, String> {
    let (path, created) = catalog::write_version(root, v)?;
    let hash = v.hash();

    let mut text = String::new();
    text.push_str(&format!(
        "{} {}  {}\n",
        s::green(action),
        s::bold(&v.plan),
        s::dim(&s::truncate(&v.goal, 60))
    ));
    text.push_str(&format!(
        "  {} {}  {}\n",
        s::bold("version"),
        s::bold(&s::short(&hash)),
        s::dim(&format!("seq={} author={}", v.seq, v.author))
    ));
    if !v.parents.is_empty() {
        text.push_str(&format!(
            "  {} {}\n",
            s::dim("parent"),
            s::dim(
                &v.parents
                    .iter()
                    .map(|p| s::short(p))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        ));
    }
    if !created {
        text.push_str(&s::note(
            "identical content already existed; nothing rewritten",
        ));
        text.push('\n');
    }
    if !v.steps.is_empty() {
        text.push_str(&format!(
            "\n  {}:\n",
            s::bold(&format!("steps({})", v.steps.len()))
        ));
        for st in v.steps.iter().take(10) {
            let mark = if st.retired {
                format!(" {}", s::dim("retired"))
            } else {
                String::new()
            };
            text.push_str(&format!(
                "    {}  {}{}\n",
                s::bold(&st.id),
                s::truncate(&st.work, 56),
                mark
            ));
        }
        if v.steps.len() > 10 {
            text.push_str(&s::dim(&format!("    + {} more\n", v.steps.len() - 10)));
        }
    }
    text.push('\n');
    text.push_str(&convergence_line(conv));
    text.push('\n');

    let json = Json::obj(vec![
        ("command", Json::str(action)),
        ("plan", Json::str(&v.plan)),
        ("version", Json::str(&hash)),
        ("seq", Json::num(v.seq as i64)),
        ("author", Json::str(&v.author)),
        ("why", Json::str(&v.why)),
        ("goal", Json::str(&v.goal)),
        ("retired", Json::Bool(v.retired)),
        ("parent", Json::strs(v.parents.clone())),
        ("path", Json::str(path.display().to_string())),
        ("created", Json::Bool(created)),
        ("step", Json::arr(v.steps.iter().map(step_json).collect())),
        ("convergence", convergence_json(conv)),
    ]);

    Ok(Output::ok(text, json))
}

fn step_json(st: &Step) -> Json {
    Json::obj(vec![
        ("step", Json::str(&st.id)),
        ("work", Json::str(&st.work)),
        ("depends_on", Json::strs(st.depends_on.clone())),
        (
            "supersedes",
            match &st.supersedes {
                Some(x) => Json::str(x),
                None => Json::Null,
            },
        ),
        ("accept", Json::str(st.accept.to_string())),
        ("retired", Json::Bool(st.retired)),
    ])
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Build identity.
///
/// Convergence is reported here too. It says nothing about a build, but the
/// contract is "every command reports convergence state alongside its answer",
/// and a uniform rule is worth more than an exemption a reader has to know
/// about.
fn cmd_version() -> Output {
    let id = build::identity();
    let conv = Convergence::probe();
    let mut text = format!("{} {}\n", s::bold("compass"), id.display_version);
    text.push_str(&format!("  {} {}\n", s::dim("machine"), id.machine_version));
    text.push_str(&format!("  {} {}\n", s::dim("source "), id.source_kind));
    if let Some(rev) = &id.rev {
        text.push_str(&format!(
            "  {} {}{}\n",
            s::dim("rev    "),
            rev,
            if id.dirty {
                format!(" {}", s::yellow("dirty"))
            } else {
                String::new()
            }
        ));
    }
    text.push('\n');
    text.push_str(&convergence_line(&conv));
    text.push('\n');

    let Json::Obj(mut fields) = id.to_json() else {
        unreachable!("build identity is an object")
    };
    fields.push(("convergence".to_string(), convergence_json(&conv)));
    Output::ok(text, Json::Obj(fields))
}

fn cmd_init(root: &Path) -> Result<Output, String> {
    let existed = catalog::exists(root);
    catalog::init(root)?;
    let conv = Convergence::probe();

    let text = format!(
        "{} {}\n\n{}\n",
        s::green(if existed {
            "catalog exists"
        } else {
            "catalog created"
        }),
        s::bold(&root.display().to_string()),
        convergence_line(&conv)
    );
    let json = Json::obj(vec![
        ("command", Json::str("init")),
        ("catalog", Json::str(root.display().to_string())),
        ("created", Json::Bool(!existed)),
        ("convergence", convergence_json(&conv)),
    ]);
    Ok(Output::ok(text, json))
}

fn cmd_new(
    root: &Path,
    author: &str,
    goal: &str,
    why: &str,
    edits: &[StepEdit],
) -> Result<Output, String> {
    catalog::init(root)?;
    let plan = refs::mint(RefKind::Plan)?;
    let steps = apply_edits(&[], edits)?;

    let v = Version {
        plan,
        seq: 1,
        parents: vec![],
        author: author.to_string(),
        why: why.to_string(),
        goal: goal.to_string(),
        retired: false,
        steps,
    };
    // Round-trip through the parser so structural rules (cycles, unknown
    // dependencies) are enforced on the way in, not discovered on the way out.
    Version::parse(&v.render())
        .map_err(|e| format!("refusing to write an invalid version: {e}"))?;

    report_write(root, &v, "created", &Convergence::probe())
}

fn cmd_revise(
    root: &Path,
    author: &str,
    plan: &str,
    why: &str,
    goal: Option<&str>,
    retire: bool,
    edits: &[StepEdit],
) -> Result<Output, String> {
    let store = load(root, plan)?;
    let an = chain::analyze(&store);

    if an.head.is_empty() {
        return Err(format!("plan {plan} has no admitted versions to revise"));
    }
    if an.head.len() > 1 {
        return Err(ambiguous_head_message(plan, &an));
    }

    let head = an.head[0];
    let base = &head.version;
    let steps = apply_edits(&base.steps, edits)?;

    let v = Version {
        plan: plan.to_string(),
        seq: base.seq + 1,
        parents: vec![head.hash.clone()],
        author: author.to_string(),
        why: why.to_string(),
        goal: goal.unwrap_or(&base.goal).to_string(),
        retired: retire || base.retired,
        steps,
    };
    if v.changes_nothing_from(base) {
        return Err(empty_revision_message(plan, &head.hash));
    }
    Version::parse(&v.render())
        .map_err(|e| format!("refusing to write an invalid version: {e}"))?;

    let mut out = report_write(root, &v, "revised", &Convergence::probe())?;
    if an.is_orphan(&head.hash) {
        out.text = format!(
            "{}  revising from an orphaned head — its predecessor has not arrived\n\n{}",
            s::warning(),
            out.text
        );
    }
    Ok(out)
}

/// The error for a revision that would record no change of intent
/// (CMP.DM-R07b).
///
/// The likeliest way to arrive here is a *correct* retry: the earlier attempt
/// landed, this one re-read Head, and re-applying the same edits to a version
/// that already carries them changes nothing. So the message says where that
/// work went, rather than only refusing.
fn empty_revision_message(plan: &str, head: &str) -> String {
    format!(
        "refusing to revise {plan}: this changes no Step and no goal\n\n  \
         head {} already carries exactly this intent.\n\n  \
         A revision must change a Step or the goal. A revision that only restates why is\n  \
         deliberately not expressible: nothing distinguishes it from a duplicate of the\n  \
         version it descends from, and under no-delete replication a duplicate is permanent.\n\n  \
         If this is a retry, the earlier attempt did land — its version is at head.\n  \
         fix: compass show {plan}\n",
        s::short(head)
    )
}

/// The error for a command needing one head when there are several.
///
/// The two causes need different repairs and must never be conflated.
fn ambiguous_head_message(plan: &str, an: &Analysis) -> String {
    let mut m = format!("plan {plan} has {} head members\n\n", an.head.len());
    for (i, h) in an.head.iter().enumerate() {
        m.push_str(&format!(
            "  {}/{} {}  {}  seq={}  {}\n",
            i + 1,
            an.head.len(),
            s::short(&h.hash),
            h.version.author,
            h.version.seq,
            s::truncate(&h.version.why, 50)
        ));
    }
    m.push('\n');
    if an.diverged() {
        m.push_str(&format!(
            "  intent diverged.\n  fix: compass reconcile {plan} --why <text>\n"
        ));
    }
    if !an.orphans.is_empty() {
        m.push_str(
            "  some head members are orphans: a predecessor has not arrived.\n  \
             fix: wait for replication — do not reconcile an orphan\n",
        );
    }
    m
}

// ---------------------------------------------------------------------------
// Structural change reporting
// ---------------------------------------------------------------------------

/// The marker for a change. Never colour alone — the symbol carries it.
fn change_mark(c: &StepChange) -> &'static str {
    match c {
        StepChange::Added { .. } => "+",
        StepChange::Superseded { .. } => ">",
        StepChange::Retired { .. } => "-",
        StepChange::Edited { .. } => "~",
        StepChange::Dropped { .. } => "x",
    }
}

/// What each version did, rendered under its Rationale.
fn changes_block(c: &VersionChange) -> String {
    let indent = "        ";
    let mut out = String::new();

    let header = match &c.basis {
        Basis::Root => "states the initial plan".to_string(),
        Basis::Parent(h) => format!("changes vs {}", s::short(h)),
        Basis::Agreed(n) => format!(
            "reconciles {}, which agreed on a step graph; changes vs that graph",
            s::count(*n, "predecessor")
        ),
        Basis::Unrecoverable(n) => format!(
            "reconciles {}; which side's step graph it carried forward is not recorded, so no \
             change can be derived",
            s::count(*n, "predecessor")
        ),
    };
    out.push_str(&s::wrapped_block(&header, indent, 80));

    if matches!(c.basis, Basis::Unrecoverable(_)) {
        out.push_str(&format!(
            "{indent}  {} {}\n",
            s::dim("now carries"),
            if c.resulting.is_empty() {
                s::dim("no steps")
            } else {
                c.resulting.join(", ")
            }
        ));
        return out;
    }

    if let Some(before) = &c.goal_before {
        out.push_str(&format!(
            "{indent}  {} {}  {}\n",
            s::bold("!"),
            "goal",
            s::dim(&format!("was {}", s::truncate(before, 56)))
        ));
    }

    for st in &c.steps {
        let detail = match st {
            StepChange::Superseded { old, .. } => format!("replaces {old}"),
            StepChange::Edited { fields, .. } => fields.join(", "),
            _ => s::truncate(st.work(), 48),
        };
        out.push_str(&format!(
            "{indent}  {} {:<11} {}  {}\n",
            s::bold(change_mark(st)),
            st.verb(),
            st.id(),
            s::dim(&detail)
        ));
    }

    if c.is_empty() && !matches!(c.basis, Basis::Root) {
        out.push_str(&format!(
            "{indent}  {}\n",
            s::dim("no change to steps or goal")
        ));
    }

    out
}

fn changes_json(c: &VersionChange) -> Json {
    let (basis, predecessors) = match &c.basis {
        Basis::Root => ("root", 0),
        Basis::Parent(_) => ("parent", 1),
        Basis::Agreed(n) => ("agreed_predecessors", *n),
        Basis::Unrecoverable(n) => ("unrecoverable", *n),
    };
    let base = match &c.basis {
        Basis::Parent(h) => Json::str(h),
        _ => Json::Null,
    };
    Json::obj(vec![
        ("basis", Json::str(basis)),
        ("predecessors", Json::num(predecessors as i64)),
        ("base", base),
        (
            "derivable",
            Json::Bool(!matches!(c.basis, Basis::Unrecoverable(_))),
        ),
        (
            "goal_before",
            match &c.goal_before {
                Some(g) => Json::str(g),
                None => Json::Null,
            },
        ),
        (
            "steps",
            Json::arr(
                c.steps
                    .iter()
                    .map(|st| {
                        Json::obj(vec![
                            ("change", Json::str(st.verb())),
                            ("step", Json::str(st.id())),
                            ("work", Json::str(st.work())),
                            (
                                "supersedes",
                                match st {
                                    StepChange::Superseded { old, .. } => Json::str(old),
                                    _ => Json::Null,
                                },
                            ),
                            (
                                "fields",
                                match st {
                                    StepChange::Edited { fields, .. } => Json::strs(
                                        fields.iter().map(|f| f.to_string()).collect::<Vec<_>>(),
                                    ),
                                    _ => Json::arr(vec![]),
                                },
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("resulting_steps", Json::strs(c.resulting.clone())),
    ])
}

fn cmd_show(root: &Path, plan: &str) -> Result<Output, String> {
    let store = load(root, plan)?;
    let an = chain::analyze(&store);
    let conv = Convergence::probe();

    let mut text = problems_block(plan, &store, &an);

    let goal = an
        .head
        .first()
        .map(|h| h.version.goal.clone())
        .unwrap_or_default();
    text.push_str(&format!(
        "{}  {}\n\n",
        s::bold(plan),
        s::truncate(&goal, 60)
    ));

    for (i, h) in an.head.iter().enumerate() {
        text.push_str(&head_line(h, i, an.head.len(), an.is_orphan(&h.hash)));
        text.push('\n');
    }
    if an.head.is_empty() {
        text.push_str(&s::dim("  no admitted versions\n"));
    }

    // The Rationale chain: the durable planning record.
    let mut chains: Vec<Json> = Vec::new();
    for (i, h) in an.head.iter().enumerate() {
        let line = chain::lineage(&store, &h.hash);
        let label = if an.head.len() > 1 {
            format!("lineage of head {}/{}", i + 1, an.head.len())
        } else {
            "lineage".to_string()
        };
        text.push_str(&format!(
            "\n  {} {}\n",
            s::bold(&format!("{label}({})", line.len())),
            s::dim("oldest first")
        ));
        for a in &line {
            text.push_str(&format!(
                "    {:03} {}  {}\n",
                a.version.seq,
                s::bold(&s::short(&a.hash)),
                a.version.author,
            ));
            // Wrapped, never elided: the lineage of Rationales is the durable
            // planning record, and it is read to understand a decision.
            text.push_str(&s::wrapped_block(&a.version.why, "        ", 80));
            // The Rationale and the change it explains belong together: read
            // apart, neither can be checked against the other.
            text.push_str(&changes_block(&change::of(&store, &a.version)));
            text.push('\n');
        }
        chains.push(Json::obj(vec![
            ("head", Json::str(&h.hash)),
            (
                "lineage",
                Json::arr(
                    line.iter()
                        .map(|a| {
                            Json::obj(vec![
                                ("version", Json::str(&a.hash)),
                                ("seq", Json::num(a.version.seq as i64)),
                                ("author", Json::str(&a.version.author)),
                                ("why", Json::str(&a.version.why)),
                                ("parent", Json::strs(a.version.parents.clone())),
                                ("changed", changes_json(&change::of(&store, &a.version))),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]));
    }

    // Steps at each head member.
    for (i, h) in an.head.iter().enumerate() {
        let label = if an.head.len() > 1 {
            format!("steps at head {}/{}", i + 1, an.head.len())
        } else {
            "steps".to_string()
        };
        text.push_str(&format!(
            "\n  {}:\n",
            s::bold(&format!("{label}({})", h.version.steps.len()))
        ));
        for st in &h.version.steps {
            text.push_str(&format!(
                "    {}  {}{}\n",
                s::bold(&st.id),
                s::truncate(&st.work, 56),
                if st.retired {
                    format!(" {}", s::dim("retired"))
                } else {
                    String::new()
                }
            ));
            text.push_str(&format!("        {}\n", s::dim(&st.accept.to_string())));
            if !st.depends_on.is_empty() {
                text.push_str(&format!(
                    "        {}\n",
                    s::dim(&format!("depends on {}", st.depends_on.join(", ")))
                ));
            }
        }
    }

    text.push_str(&format!(
        "\n{}\n",
        s::dim(&format!(
            "{} · {} · {} · {}",
            s::count(store.versions.len(), "version"),
            s::count_of(an.head.len(), "head", "heads"),
            s::count(store.events.len(), "event"),
            an.state()
        ))
    ));
    text.push('\n');
    text.push_str(&convergence_line(&conv));
    text.push('\n');

    let mut fields = vec![
        ("command", Json::str("show")),
        ("plan", Json::str(plan)),
        ("goal", Json::str(&goal)),
        ("state", Json::str(an.state())),
        (
            "head",
            Json::arr(
                an.head
                    .iter()
                    .map(|h| head_json_with_steps(h, an.is_orphan(&h.hash)))
                    .collect(),
            ),
        ),
        ("lineages", Json::arr(chains)),
        ("version_count", Json::num(store.versions.len() as i64)),
        ("event_count", Json::num(store.events.len() as i64)),
    ];
    fields.extend(problems_json(&store, &an));
    fields.push(("convergence", convergence_json(&conv)));

    Ok(Output::ok(text, Json::obj(fields)))
}

fn cmd_ready(root: &Path, plan: &str) -> Result<Output, String> {
    let store = load(root, plan)?;
    let an = chain::analyze(&store);
    let conv = Convergence::probe();
    let all = readiness::for_plan(&store, &an);

    let mut text = problems_block(plan, &store, &an);
    let goal = an
        .head
        .first()
        .map(|h| h.version.goal.clone())
        .unwrap_or_default();
    text.push_str(&format!("{}  {}\n", s::bold(plan), s::truncate(&goal, 60)));

    if all.is_empty() {
        text.push_str(&s::dim("\n  no admitted versions — nothing is ready\n"));
    }

    for (i, r) in all.iter().enumerate() {
        text.push('\n');
        let label = if all.len() > 1 {
            format!("head {}/{}", i + 1, all.len())
        } else {
            "head".to_string()
        };
        text.push_str(&format!(
            "  {} {}  {}  {}{}\n",
            s::bold(&label),
            s::bold(&s::short(&r.head)),
            r.author,
            s::dim(&format!("seq={}", r.seq)),
            if r.orphan {
                format!(" {}", s::red("orphan"))
            } else {
                String::new()
            }
        ));
        if r.orphan {
            text.push_str(&s::note(
                "this answer is provisional: a predecessor has not arrived",
            ));
            text.push('\n');
        }

        for (state, marker, colour) in [
            (StepState::Ready, "*", 1),
            (StepState::Blocked, "✗", 2),
            (StepState::Accepted, "✓", 3),
            (StepState::Retired, "-", 0),
        ] {
            let rows: Vec<_> = r.steps.iter().filter(|x| x.state == state).collect();
            if rows.is_empty() {
                continue;
            }
            text.push_str(&format!(
                "\n    {}:\n",
                s::bold(&format!("{}({})", state.as_str(), rows.len()))
            ));
            for row in rows.iter().take(10) {
                let m = match colour {
                    1 => s::yellow(marker),
                    2 => s::red(marker),
                    3 => s::green(marker),
                    _ => s::dim(marker),
                };
                text.push_str(&format!(
                    "      {} {}  {}\n",
                    m,
                    s::bold(&row.step),
                    s::truncate(&row.work, 52)
                ));
                text.push_str(&format!("          {}\n", s::dim(&row.reason)));
            }
            if rows.len() > 10 {
                text.push_str(&s::dim(&format!("      + {} more\n", rows.len() - 10)));
            }
        }

        text.push_str(&format!(
            "\n  {}\n",
            s::dim(&format!(
                "{} ready · {} blocked · {} accepted · {} retired",
                r.count(StepState::Ready),
                r.count(StepState::Blocked),
                r.count(StepState::Accepted),
                r.count(StepState::Retired),
            ))
        ));
    }

    text.push('\n');
    text.push_str(&convergence_line(&conv));
    text.push('\n');

    let mut fields = vec![
        ("command", Json::str("ready")),
        ("plan", Json::str(plan)),
        ("goal", Json::str(&goal)),
        ("state", Json::str(an.state())),
        ("diverged", Json::Bool(an.diverged())),
        ("head", Json::arr(all.iter().map(readiness_json).collect())),
    ];
    fields.extend(problems_json(&store, &an));
    fields.push(("convergence", convergence_json(&conv)));

    Ok(Output::ok(text, Json::obj(fields)))
}

fn readiness_json(r: &HeadReadiness) -> Json {
    Json::obj(vec![
        ("version", Json::str(&r.head)),
        ("seq", Json::num(r.seq as i64)),
        ("author", Json::str(&r.author)),
        ("orphan", Json::Bool(r.orphan)),
        (
            "step",
            Json::arr(
                r.steps
                    .iter()
                    .map(|x| {
                        Json::obj(vec![
                            ("step", Json::str(&x.step)),
                            ("work", Json::str(&x.work)),
                            ("state", Json::str(x.state.as_str())),
                            ("reason", Json::str(&x.reason)),
                            ("blocked_by", Json::strs(x.blocked_by.clone())),
                            ("accept", Json::str(&x.accept)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "summary",
            Json::obj(vec![
                ("ready", Json::num(r.count(StepState::Ready) as i64)),
                ("blocked", Json::num(r.count(StepState::Blocked) as i64)),
                ("accepted", Json::num(r.count(StepState::Accepted) as i64)),
                ("retired", Json::num(r.count(StepState::Retired) as i64)),
            ]),
        ),
    ])
}

/// Choose the version an event is observed against.
///
/// Prefer a head member that actually carries the step. When several do —
/// normal under divergence — pick deterministically and disclose the choice
/// rather than appearing to have had only one option.
fn observed_against<'a>(an: &Analysis<'a>, step: &str) -> Result<(&'a Admitted, bool), String> {
    if an.head.is_empty() {
        return Err("plan has no admitted versions to record progress against".to_string());
    }
    let carrying: Vec<&&Admitted> = an
        .head
        .iter()
        .filter(|h| h.version.step(step).is_some())
        .collect();

    match carrying.len() {
        0 => Err(format!(
            "no step {step} at head\n  fix: compass show <plan>"
        )),
        1 => Ok((carrying[0], false)),
        _ => {
            let chosen = carrying
                .iter()
                .max_by(|a, b| (a.version.seq, &a.hash).cmp(&(b.version.seq, &b.hash)))
                .expect("non-empty");
            Ok((chosen, true))
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn record_event(
    root: &Path,
    store: &PlanStore,
    an: &Analysis,
    author: &str,
    plan: &str,
    step: &str,
    kind: EventKind,
    note: Option<&str>,
    evidence_kind: Option<String>,
    attrs: Vec<(String, String)>,
) -> Result<Output, String> {
    // Refused, not sanitised (decision 0008, CMP.DM-R13b). A predicate term
    // naming a recorded field binds what Compass recorded, so an attribute of
    // the same name could never mean what its author intended — accepting it
    // silently would let a plan be authored against a criterion that can never
    // hold the way it reads.
    if let Some(k) = Event::shadowing_attr(&attrs) {
        return Err(format!(
            "refusing to record evidence: attribute `{k}` shadows the recorded field `{k}`\n  \
             Compass records `{k}` itself, and an `accept` term naming `{k}` binds that recorded \
             value — never an attribute.\n  \
             reserved: {}\n  \
             fix: drop `{k}=…`, or name the claim something else",
            crate::event::RECORDED_FIELDS.join(", ")
        ));
    }

    let (against, ambiguous) = observed_against(an, step)?;
    let conv = Convergence::probe();

    let mut sorted = attrs;
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    let e = Event {
        id: refs::mint(RefKind::Event)?,
        at: store.next_event_at(),
        wall: crate::event::now_wall(),
        plan: plan.to_string(),
        step: step.to_string(),
        version: against.hash.clone(),
        actor: author.to_string(),
        kind,
        note: note.map(|n| n.to_string()),
        evidence_kind,
        attrs: sorted,
    };
    let path = catalog::write_event(root, &e)?;

    let mut text = String::new();
    text.push_str(&format!(
        "{} {}  {} {}\n",
        s::green("recorded"),
        s::bold(e.kind.as_str()),
        s::bold(&e.step),
        s::dim(&format!("in {plan}"))
    ));
    text.push_str(&format!(
        "  {} {}  {}\n",
        s::dim("against version"),
        s::short(&e.version),
        s::dim(&format!("at={} actor={}", e.at, e.actor))
    ));
    if let Some(k) = &e.evidence_kind {
        text.push_str(&format!(
            "  {} {}{}\n",
            s::dim("evidence"),
            s::bold(k),
            if e.attrs.is_empty() {
                String::new()
            } else {
                format!(
                    "  {}",
                    s::dim(
                        &e.attrs
                            .iter()
                            .map(|(k, v)| format!("{k}={v}"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                )
            }
        ));
    }
    if ambiguous {
        text.push_str(&s::note(
            "head is divergent and several members carry this step; recorded against the deepest in the lineage",
        ));
        text.push('\n');
    }
    if matches!(kind, EventKind::Done) {
        text.push_str(&s::note(
            "`done` records that you finished; it does not accept the step — acceptance is judged from evidence (CMP-R14)",
        ));
        text.push('\n');
    }
    text.push('\n');
    text.push_str(&convergence_line(&conv));
    text.push('\n');

    let json = Json::obj(vec![
        (
            "command",
            Json::str(if kind == EventKind::Evidence {
                "evidence"
            } else {
                "progress"
            }),
        ),
        ("plan", Json::str(plan)),
        ("step", Json::str(step)),
        ("event", Json::str(&e.id)),
        ("kind", Json::str(e.kind.as_str())),
        ("at", Json::num(e.at as i64)),
        ("wall", Json::num(e.wall as i64)),
        ("actor", Json::str(&e.actor)),
        ("observed_against", Json::str(&e.version)),
        ("head_ambiguous", Json::Bool(ambiguous)),
        (
            "evidence_kind",
            match &e.evidence_kind {
                Some(k) => Json::str(k),
                None => Json::Null,
            },
        ),
        (
            "attrs",
            Json::Obj(
                e.attrs
                    .iter()
                    .map(|(k, v)| (k.clone(), Json::str(v)))
                    .collect(),
            ),
        ),
        (
            "note",
            match &e.note {
                Some(n) => Json::str(n),
                None => Json::Null,
            },
        ),
        ("path", Json::str(path.display().to_string())),
        ("accepts_step", Json::Bool(false)),
        ("convergence", convergence_json(&conv)),
    ]);

    Ok(Output::ok(text, json))
}

fn cmd_progress(
    root: &Path,
    author: &str,
    plan: &str,
    step: &str,
    kind: &str,
    note: Option<&str>,
) -> Result<Output, String> {
    let store = load(root, plan)?;
    let an = chain::analyze(&store);
    let kind = EventKind::parse(kind).ok_or_else(|| format!("unknown progress kind `{kind}`"))?;
    record_event(
        root,
        &store,
        &an,
        author,
        plan,
        step,
        kind,
        note,
        None,
        vec![],
    )
}

fn cmd_evidence(
    root: &Path,
    author: &str,
    plan: &str,
    step: &str,
    kind: &str,
    attrs: &[(String, String)],
) -> Result<Output, String> {
    let store = load(root, plan)?;
    let an = chain::analyze(&store);
    record_event(
        root,
        &store,
        &an,
        author,
        plan,
        step,
        EventKind::Evidence,
        None,
        Some(kind.to_string()),
        attrs.to_vec(),
    )
}

fn cmd_status(root: &Path) -> Result<Output, String> {
    if !catalog::exists(root) {
        return Err(format!(
            "no catalog at {}\n  fix: compass init",
            root.display()
        ));
    }
    let conv = Convergence::probe();
    let plans = catalog::list_plans(root)?;

    let mut rows: Vec<(String, PlanStore)> = Vec::new();
    for p in &plans {
        rows.push((p.clone(), catalog::load_plan(root, p)?));
    }

    let mut text = String::new();
    let mut entries: Vec<Json> = Vec::new();
    let (mut diverged, mut orphaned, mut rejected) = (0usize, 0usize, 0usize);

    // Problems first, across all plans.
    let mut problems = String::new();
    for (plan, store) in &rows {
        let an = chain::analyze(store);
        if an.diverged() {
            diverged += 1;
        }
        if !an.orphans.is_empty() {
            orphaned += 1;
        }
        rejected += store.rejected.len() + store.bad_events.len();
        problems.push_str(&problems_block(plan, store, &an));
    }
    text.push_str(&problems);

    for (plan, store) in &rows {
        let an = chain::analyze(store);
        let goal = an
            .head
            .first()
            .map(|h| h.version.goal.clone())
            .unwrap_or_default();
        let marker = if an.diverged() {
            s::red("↕")
        } else if !an.orphans.is_empty() {
            s::yellow("*")
        } else if an.head.len() == 1 {
            s::green("✓")
        } else {
            s::dim("-")
        };
        let retired = an.head.first().is_some_and(|h| h.version.retired);
        text.push_str(&format!(
            "{} {}  {}  {}\n",
            marker,
            s::bold(plan),
            s::truncate(&goal, 44),
            s::dim(&format!(
                "{}{} · heads({}) · {}",
                an.state(),
                if retired { " · retired" } else { "" },
                an.head.len(),
                s::count(store.versions.len(), "version")
            ))
        ));

        let mut fields = vec![
            ("plan", Json::str(plan)),
            ("goal", Json::str(&goal)),
            ("state", Json::str(an.state())),
            ("retired", Json::Bool(retired)),
            ("diverged", Json::Bool(an.diverged())),
            (
                "head",
                Json::arr(
                    an.head
                        .iter()
                        .map(|h| head_json(h, an.is_orphan(&h.hash)))
                        .collect(),
                ),
            ),
            ("version_count", Json::num(store.versions.len() as i64)),
            ("event_count", Json::num(store.events.len() as i64)),
        ];
        fields.extend(problems_json(store, &an));
        entries.push(Json::obj(fields));
    }

    if rows.is_empty() {
        text.push_str(&s::dim("no plans\n"));
        text.push_str(&s::fix("compass new --goal <text> --why <text>"));
        text.push('\n');
    }

    text.push_str(&format!(
        "\n{}\n",
        s::dim(&format!(
            "{} · {} diverged · {} orphaned · {} rejected",
            s::count(rows.len(), "plan"),
            diverged,
            orphaned,
            rejected
        ))
    ));
    text.push('\n');
    text.push_str(&convergence_line(&conv));
    text.push('\n');

    let json = Json::obj(vec![
        ("command", Json::str("status")),
        ("catalog", Json::str(root.display().to_string())),
        ("plan", Json::arr(entries)),
        (
            "summary",
            Json::obj(vec![
                ("plans", Json::num(rows.len() as i64)),
                ("diverged", Json::num(diverged as i64)),
                ("orphaned", Json::num(orphaned as i64)),
                ("rejected", Json::num(rejected as i64)),
            ]),
        ),
        ("convergence", convergence_json(&conv)),
    ]);

    Ok(Output::ok(text, json))
}

fn cmd_reconcile(
    root: &Path,
    author: &str,
    plan: &str,
    why: &str,
    from: Option<&str>,
    edits: &[StepEdit],
) -> Result<Output, String> {
    let store = load(root, plan)?;
    let an = chain::analyze(&store);

    if an.head.len() < 2 {
        return Err(format!(
            "plan {plan} has {} head member(s); there is nothing to reconcile",
            an.head.len()
        ));
    }

    // An orphan is not a divergence and must never be reconciled.
    if !an.orphans.is_empty() {
        let orphan_heads: Vec<&str> = an
            .head
            .iter()
            .filter(|h| an.is_orphan(&h.hash))
            .map(|h| h.hash.as_str())
            .collect();
        if !orphan_heads.is_empty() {
            return Err(format!(
                "refusing to reconcile: {} head member(s) are orphans, not divergent\n\n{}\n\n  \
                 An orphan is a version whose predecessor has not arrived. Reconciling around it \n  \
                 would write permanent intent to repair a transient condition.\n  \
                 fix: wait for replication, then re-check with `compass verify {plan}`",
                orphan_heads.len(),
                orphan_heads
                    .iter()
                    .map(|h| format!("    {} missing a predecessor", s::short(h)))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ));
        }
    }

    if !an.diverged() {
        return Err(format!(
            "plan {plan} has several head members but none share a predecessor, so this is not a \
             divergence\n  fix: compass verify {plan}"
        ));
    }

    // Which side's step graph carries forward. Compass never picks for you.
    let base_steps: Vec<Step> = match from {
        Some(prefix) => {
            let chosen = store.resolve_hash(prefix).ok_or_else(|| {
                format!("`--from {prefix}` matches no version, or matches more than one")
            })?;
            if !an.head.iter().any(|h| h.hash == chosen.hash) {
                return Err(format!(
                    "`--from {prefix}` is not a head member; reconciliation carries forward one \
                     of the diverged sides"
                ));
            }
            chosen.version.steps.clone()
        }
        None => {
            let first = &an.head[0].version.steps;
            let identical = an.head.iter().all(|h| h.version.steps == *first);
            if !identical {
                let mut m = String::from(
                    "the diverged sides carry different step graphs, so Compass cannot choose \
                     between them\n\n",
                );
                for (i, h) in an.head.iter().enumerate() {
                    m.push_str(&format!(
                        "  {}/{} {}  {}  {} step(s)  {}\n",
                        i + 1,
                        an.head.len(),
                        s::short(&h.hash),
                        h.version.author,
                        h.version.steps.len(),
                        s::truncate(&h.version.why, 44)
                    ));
                }
                m.push_str(&format!(
                    "\n  fix: compass reconcile {plan} --from <version> --why <text>\n  \
                     then adjust with --add-step / --edit-step / --retire-step\n"
                ));
                return Err(m);
            }
            first.clone()
        }
    };

    let steps = apply_edits(&base_steps, edits)?;
    let parents: Vec<String> = an.head.iter().map(|h| h.hash.clone()).collect();
    let goal = an.head[0].version.goal.clone();

    let v = Version {
        plan: plan.to_string(),
        seq: chain::next_seq(&an.head),
        parents,
        author: author.to_string(),
        why: why.to_string(),
        goal,
        retired: false,
        steps,
    };
    Version::parse(&v.render())
        .map_err(|e| format!("refusing to write an invalid version: {e}"))?;

    report_write(root, &v, "reconciled", &Convergence::probe())
}

fn cmd_verify(root: &Path, plan: Option<&str>, all: bool) -> Result<Output, String> {
    if !catalog::exists(root) {
        return Err(format!(
            "no catalog at {}\n  fix: compass init",
            root.display()
        ));
    }
    let conv = Convergence::probe();
    let plans: Vec<String> = if all {
        catalog::list_plans(root)?
    } else {
        vec![plan
            .expect("parser guarantees one of plan or --all")
            .to_string()]
    };

    let mut text = String::new();
    let mut entries: Vec<Json> = Vec::new();
    let mut failures = 0usize;

    for p in &plans {
        let store = if all {
            catalog::load_plan(root, p)?
        } else {
            load(root, p)?
        };
        let an = chain::analyze(&store);
        let bad = store.rejected.len() + store.bad_events.len();
        failures += bad;

        text.push_str(&problems_block(p, &store, &an));

        // Chain integrity: every named predecessor is present, or the version
        // is an orphan and reported as such.
        let marker = if bad > 0 {
            s::red("✗")
        } else if !an.orphans.is_empty() {
            s::yellow("*")
        } else {
            s::green("✓")
        };
        text.push_str(&format!(
            "{} {}  {}\n",
            marker,
            s::bold(p),
            s::dim(&format!(
                "{} admitted · {} rejected · {} · {} · {}",
                store.versions.len(),
                bad,
                s::count_of(an.head.len(), "head", "heads"),
                s::count(an.orphans.len(), "orphan"),
                an.state()
            ))
        ));

        let mut fields = vec![
            ("plan", Json::str(p)),
            ("state", Json::str(an.state())),
            ("admitted", Json::num(store.versions.len() as i64)),
            ("rejected_count", Json::num(bad as i64)),
            ("ok", Json::Bool(bad == 0)),
            (
                "head",
                Json::arr(
                    an.head
                        .iter()
                        .map(|h| head_json(h, an.is_orphan(&h.hash)))
                        .collect(),
                ),
            ),
        ];
        fields.extend(problems_json(&store, &an));
        entries.push(Json::obj(fields));
    }

    text.push_str(&format!(
        "\n{}\n",
        s::dim(&format!(
            "{} · {} rejected",
            s::count(plans.len(), "plan"),
            s::count(failures, "file")
        ))
    ));
    text.push('\n');
    text.push_str(&convergence_line(&conv));
    text.push('\n');

    let json = Json::obj(vec![
        ("command", Json::str("verify")),
        ("plan", Json::arr(entries)),
        ("ok", Json::Bool(failures == 0)),
        ("rejected_count", Json::num(failures as i64)),
        ("convergence", convergence_json(&conv)),
    ]);

    Ok(Output {
        text,
        json,
        code: if failures == 0 { 0 } else { EXIT_FAILURE },
    })
}

/// Expose the resolved catalog root for diagnostics.
pub fn resolved_root(inv: &Invocation) -> Result<PathBuf, String> {
    match &inv.catalog {
        Some(p) => Ok(p.clone()),
        None => catalog::root(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A scratch catalog that cleans itself up.
    struct Scratch {
        root: PathBuf,
    }

    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let base = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
            let unique = refs::mint(RefKind::Event).unwrap();
            let root = PathBuf::from(base).join(format!("compass-cmd-{tag}-{unique}"));
            catalog::init(&root).unwrap();
            Scratch { root }
        }

        /// Run one invocation against this catalog, as the CLI would.
        fn run(&self, args: &[&str]) -> Result<Output, String> {
            let mut argv = vec!["--catalog".to_string(), self.root.display().to_string()];
            argv.extend(args.iter().map(|s| s.to_string()));
            execute(&crate::cli::parse(&argv)?)
        }

        fn plan(&self) -> String {
            catalog::list_plans(&self.root).unwrap().remove(0)
        }

        fn store(&self) -> PlanStore {
            catalog::load_plan(&self.root, &self.plan()).unwrap()
        }

        fn version_count(&self) -> usize {
            self.store().versions.len()
        }

        /// The single head member. Panics if the plan has diverged.
        fn head(&self) -> Version {
            let store = self.store();
            let an = chain::analyze(&store);
            assert_eq!(an.head.len(), 1, "expected one head member");
            an.head[0].version.clone()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            fn writable(p: &Path) -> std::io::Result<()> {
                if p.is_dir() {
                    for e in fs::read_dir(p)? {
                        writable(&e?.path())?;
                    }
                } else if p.is_file() {
                    let mut perms = fs::metadata(p)?.permissions();
                    #[allow(clippy::permissions_set_readonly_false)]
                    perms.set_readonly(false);
                    fs::set_permissions(p, perms)?;
                }
                Ok(())
            }
            let _ = writable(&self.root);
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// A plan with one step, ready to be revised.
    fn a_plan(tag: &str) -> (Scratch, String, String) {
        let s = Scratch::new(tag);
        s.run(&[
            "new",
            "--goal",
            "Nested groups parse correctly",
            "--why",
            "Parser rejects nested groups.",
            "--add-step",
            "Reproduce with a failing test",
            "--accept",
            "test(name=nested, status=fail)",
        ])
        .unwrap();
        let plan = s.plan();
        let step = s.head().steps[0].id.clone();
        (s, plan, step)
    }

    /// The regression decision 0007 exists for: an agent revises, loses its
    /// process before it can read the answer, and — correctly — retries.
    ///
    /// The retry re-reads the catalog and sees its own landed write, so it
    /// cannot produce the same bytes as its first attempt: head has moved, so
    /// the candidate names a different predecessor. What stops the duplicate
    /// is that re-applying the same edits to the version that already carries
    /// them changes no Step and no goal, and such a revision is refused. The
    /// plan ends with exactly one new version and one stated reason for it.
    #[test]
    fn an_identical_revise_issued_twice_records_one_version() {
        let (s, plan, step) = a_plan("crash-retry");
        let before = s.version_count();

        let attempt = [
            "revise",
            &plan,
            "--why",
            "Reproduction showed the tokenizer, not the grammar, drops it.",
            "--edit-step",
            &step,
            "--work",
            "Fix the tokenizer",
        ];

        s.run(&attempt).expect("the first attempt lands");
        assert_eq!(s.version_count(), before + 1);
        let after_first = s.head();

        // The response never reached the agent. It retries the same mutation.
        let err = s
            .run(&attempt)
            .expect_err("the retry must not record a second version");
        assert!(
            err.contains("changes no Step and no goal"),
            "the refusal must say what is missing: {err}"
        );
        assert!(
            err.contains("did land"),
            "and must say where the earlier attempt went: {err}"
        );

        assert_eq!(
            s.version_count(),
            before + 1,
            "a correct retry must leave one version, not two"
        );
        assert_eq!(s.head(), after_first, "and head must be unmoved");
        assert_eq!(
            s.store()
                .versions
                .iter()
                .filter(|a| a.version.why.contains("not the grammar"))
                .count(),
            1,
            "the rationale must appear once in the record, not twice"
        );
    }

    /// The case no derived value can catch: the retry's bodies genuinely
    /// differ, because the rationale was reworded. Only the rule refuses it.
    #[test]
    fn a_revision_differing_only_in_its_rationale_is_refused() {
        let (s, plan, step) = a_plan("reworded-retry");
        s.run(&[
            "revise",
            &plan,
            "--why",
            "The tokenizer drops it.",
            "--edit-step",
            &step,
            "--work",
            "Fix the tokenizer",
        ])
        .unwrap();
        let before = s.version_count();

        // Same mutation, different words for why.
        let err = s
            .run(&[
                "revise",
                &plan,
                "--why",
                "It is the tokenizer that drops it, not the grammar.",
                "--edit-step",
                &step,
                "--work",
                "Fix the tokenizer",
            ])
            .expect_err("a rationale-only revision is not expressible");
        assert!(err.contains("changes no Step and no goal"), "{err}");
        assert!(
            err.contains("only restates why"),
            "the refusal must name the case it is closing: {err}"
        );

        // And with no step edit at all — a bare change of mind.
        let err = s
            .run(&[
                "revise",
                &plan,
                "--why",
                "We reconsidered and are keeping this plan.",
            ])
            .expect_err("a deliberate non-change is deliberately not recordable");
        assert!(err.contains("changes no Step and no goal"), "{err}");

        assert_eq!(s.version_count(), before);
    }

    /// Retirement changes the plan, so the empty-revision rule must let it
    /// through — of the plan and of a single step alike.
    #[test]
    fn retirement_is_not_an_empty_revision() {
        let (s, plan, step) = a_plan("retire");
        let before = s.version_count();

        s.run(&[
            "revise",
            &plan,
            "--why",
            "Superseded by the rewrite.",
            "--retire",
        ])
        .expect("retiring the plan changes it");
        assert_eq!(s.version_count(), before + 1);
        assert!(s.head().retired);

        s.run(&[
            "revise",
            &plan,
            "--why",
            "This step is no longer worth doing.",
            "--retire-step",
            &step,
        ])
        .expect("retiring a step changes it");
        assert_eq!(s.version_count(), before + 2);
        assert!(s.head().steps[0].retired);

        // Retiring what is already retired changes nothing, and is refused —
        // which is the rule working, not an exception to it.
        let err = s
            .run(&[
                "revise",
                &plan,
                "--why",
                "Retiring it again.",
                "--retire-step",
                &step,
            ])
            .expect_err("a second retirement of the same step changes nothing");
        assert!(err.contains("changes no Step and no goal"), "{err}");
        assert_eq!(s.version_count(), before + 2);
    }

    /// A Reconciliation always changes the predecessor set, so it is never an
    /// empty revision — even when both sides carry an identical step graph and
    /// goal, which is precisely when the naive rule would misfire.
    #[test]
    fn reconciliation_is_not_an_empty_revision() {
        let (s, plan, _step) = a_plan("reconcile");
        let base = s.head();
        let base_hash = base.hash();

        // Two machines revise the same parent, differing only in the words —
        // so the two sides agree on every step and on the goal.
        for why in ["The tokenizer drops it.", "The grammar is fine."] {
            let side = Version {
                plan: plan.clone(),
                seq: base.seq + 1,
                parents: vec![base_hash.clone()],
                author: "cos".into(),
                why: why.into(),
                goal: base.goal.clone(),
                retired: false,
                steps: base.steps.clone(),
            };
            catalog::write_version(&s.root, &side).unwrap();
        }
        let before = s.version_count();
        assert!(chain::analyze(&s.store()).diverged());

        s.run(&[
            "reconcile",
            &plan,
            "--why",
            "Both sides said the same thing.",
        ])
        .expect("a reconciliation joins predecessors, so it always changes something");

        assert_eq!(s.version_count(), before + 1);
        let head = s.head();
        assert_eq!(head.parents.len(), 2, "it names both sides");
        assert_eq!(head.steps, base.steps, "carrying the agreed graph forward");
    }

    // --- evidence is a claim (decision 0008) -----------------------------

    /// A plan whose one step demands approval from a named actor.
    fn a_plan_needing_an_editor(tag: &str) -> (Scratch, String, String) {
        let s = Scratch::new(tag);
        s.run(&[
            "new",
            "--goal",
            "Publish the piece",
            "--why",
            "It is drafted and needs a second reader.",
            "--add-step",
            "Get the draft approved",
            "--accept",
            "review(actor=editor, verdict=approved)",
        ])
        .unwrap();
        let plan = s.plan();
        let step = s.head().steps[0].id.clone();
        (s, plan, step)
    }

    fn accepted(s: &Scratch) -> bool {
        let store = s.store();
        let an = chain::analyze(&store);
        let r = readiness::for_head(&store, an.head[0], false);
        r.steps[0].state == StepState::Accepted
    }

    #[test]
    fn a_writer_cannot_forge_an_approval_by_claiming_the_actor() {
        let (s, plan, step) = a_plan_needing_an_editor("forge");

        // The forgery: the writer names someone else in the attributes.
        let err = s
            .run(&[
                "--author",
                "writer",
                "evidence",
                &plan,
                &step,
                "review",
                "actor=editor",
                "verdict=approved",
            ])
            .expect_err("an attribute shadowing a recorded field must be refused");
        assert!(err.contains("actor"), "{err}");
        assert!(err.contains("shadows"), "{err}");

        // Nothing was written, so nothing was accepted.
        assert!(s.store().events.is_empty());
        assert!(!accepted(&s));

        // Even the attributes the writer *may* record do not satisfy it,
        // because the recorded actor is not the editor.
        s.run(&[
            "--author",
            "writer",
            "evidence",
            &plan,
            &step,
            "review",
            "verdict=approved",
        ])
        .unwrap();
        assert!(
            !accepted(&s),
            "a criterion naming an approver must not be satisfiable by anyone else"
        );

        // The genuine article: recorded by the editor.
        s.run(&[
            "--author",
            "editor",
            "evidence",
            &plan,
            &step,
            "review",
            "verdict=approved",
        ])
        .unwrap();
        assert!(accepted(&s), "a genuine editor approval must satisfy it");
    }

    #[test]
    fn every_recorded_field_is_refused_as_an_evidence_attribute() {
        let (s, plan, step) = a_plan_needing_an_editor("shadow-all");
        for field in crate::event::RECORDED_FIELDS {
            let err = s
                .run(&["evidence", &plan, &step, "review", &format!("{field}=x")])
                .expect_err(&format!("attribute `{field}` shadows a recorded field"));
            assert!(err.contains(field), "{err}");
        }
    }

    #[test]
    fn an_ordinary_attribute_is_still_free() {
        let (s, plan, step) = a_plan_needing_an_editor("free-attr");
        s.run(&["evidence", &plan, &step, "review", "reviewer=editor"])
            .expect("`reviewer` is not a recorded field and stays available");
        assert_eq!(s.store().events.len(), 1);
    }

    // --- structural change reporting -------------------------------------

    #[test]
    fn show_reports_what_each_version_changed() {
        let (s, plan, step) = a_plan("show-changes");
        s.run(&[
            "revise",
            &plan,
            "--why",
            "Adding the fix once the repro exists.",
            "--add-step",
            "Fix the parser",
            "--accept",
            "test(name=nested, status=pass)",
        ])
        .unwrap();
        s.run(&[
            "revise",
            &plan,
            "--why",
            "The repro is no longer the shape we need.",
            "--retire-step",
            &step,
        ])
        .unwrap();

        let out = s.run(&["show", &plan]).unwrap();
        assert!(out.text.contains("states the initial plan"), "{}", out.text);
        assert!(out.text.contains("changes vs "), "{}", out.text);
        assert!(out.text.contains("added"), "{}", out.text);
        assert!(out.text.contains("retired"), "{}", out.text);
    }

    #[test]
    fn a_divergent_rationale_is_never_elided() {
        let (s, plan, _step) = a_plan("full-why");
        let long = "One of the three changed its storage engine mid-benchmark, so the numbers \
                    are not comparable and the conclusion drawn from them does not stand.";
        let base = s.head();
        for author in ["a", "b"] {
            let side = Version {
                plan: plan.clone(),
                seq: base.seq + 1,
                parents: vec![base.hash()],
                author: author.into(),
                why: long.to_string(),
                goal: base.goal.clone(),
                retired: false,
                steps: base.steps.clone(),
            };
            catalog::write_version(&s.root, &side).unwrap();
        }
        assert!(chain::analyze(&s.store()).diverged());

        let out = s.run(&["status"]).unwrap();
        assert!(
            !out.text.contains('…'),
            "a rationale was elided:\n{}",
            out.text
        );
        // Every word survives, even though the line does not.
        for word in long.split_whitespace() {
            assert!(
                out.text.contains(word),
                "`{word}` missing from:\n{}",
                out.text
            );
        }
    }

    #[test]
    fn counts_agree_with_their_nouns() {
        let (s, plan, step) = a_plan("plurals");
        s.run(&["progress", &plan, &step, "start"]).unwrap();
        let out = s.run(&["show", &plan]).unwrap();
        assert!(out.text.contains("1 event"), "{}", out.text);
        assert!(!out.text.contains("1 events"), "{}", out.text);
        assert!(out.text.contains("1 head"), "{}", out.text);
        assert!(!out.text.contains("1 heads"), "{}", out.text);

        s.run(&["progress", &plan, &step, "update"]).unwrap();
        let out = s.run(&["show", &plan]).unwrap();
        assert!(out.text.contains("2 events"), "{}", out.text);
    }

    /// Nothing a command reports may name a field the model no longer has.
    #[test]
    fn no_command_reports_a_logical_clock_for_a_version() {
        let (s, plan, _step) = a_plan("no-clock");
        for args in [
            vec!["show", &plan],
            vec!["ready", &plan],
            vec!["status"],
            vec!["verify", &plan],
        ] {
            let out = s.run(&args).unwrap();
            assert!(
                !out.text.contains("at="),
                "`{}` still reports a version clock:\n{}",
                args.join(" "),
                out.text
            );
        }
    }
}
