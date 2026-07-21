//! Command-line parsing.
//!
//! Hand-rolled, because Compass takes no external crates. The spelling is open
//! per the spec; the contract is not:
//!
//! - every command reports convergence state alongside its answer;
//! - every command that reports Head handles a Head set larger than one
//!   without erroring, and labels the members;
//! - `--json` is available for every command and carries the same fields as
//!   the human rendering.
//!
//! Step edits use a positional-grouping grammar: `--add-step` or `--edit-step`
//! opens a step context, and `--work`, `--accept`, `--depends-on` and
//! `--supersedes` attach to the most recently opened one.

use std::path::PathBuf;

/// Exit code for a usage error, distinct from an operational failure.
pub const EXIT_USAGE: i32 = 2;
/// Exit code for an operational failure.
pub const EXIT_FAILURE: i32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepEdit {
    Add {
        work: String,
        accept: Option<String>,
        depends_on: Vec<String>,
        supersedes: Option<String>,
    },
    Edit {
        id: String,
        work: Option<String>,
        accept: Option<String>,
        depends_on: Option<Vec<String>>,
        supersedes: Option<String>,
    },
    Retire {
        id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Init,
    New {
        goal: String,
        why: String,
        steps: Vec<StepEdit>,
    },
    Revise {
        plan: String,
        why: String,
        goal: Option<String>,
        retire: bool,
        steps: Vec<StepEdit>,
    },
    Show {
        plan: String,
    },
    Ready {
        plan: String,
    },
    Progress {
        plan: String,
        step: String,
        kind: String,
        note: Option<String>,
    },
    Evidence {
        plan: String,
        step: String,
        kind: String,
        attrs: Vec<(String, String)>,
    },
    Status,
    Reconcile {
        plan: String,
        why: String,
        from: Option<String>,
        steps: Vec<StepEdit>,
    },
    Verify {
        plan: Option<String>,
        all: bool,
    },
    Version,
    Help {
        topic: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub command: Command,
    pub json: bool,
    pub catalog: Option<PathBuf>,
    pub author: Option<String>,
}

/// Parse an argument vector (excluding argv[0]).
pub fn parse(args: &[String]) -> Result<Invocation, String> {
    let mut json = false;
    let mut catalog = None;
    let mut author = None;
    let mut rest: Vec<String> = Vec::new();

    // Global flags may appear anywhere, so they are lifted out first. Values
    // after `--` are never treated as flags.
    let mut it = args.iter().cloned();
    let mut literal = false;
    while let Some(a) = it.next() {
        if literal {
            rest.push(a);
            continue;
        }
        match a.as_str() {
            "--" => literal = true,
            "--json" => json = true,
            "--catalog" => {
                catalog = Some(PathBuf::from(it.next().ok_or("`--catalog` needs a path")?))
            }
            "--author" => author = Some(it.next().ok_or("`--author` needs a name")?),
            _ => rest.push(a),
        }
    }

    if rest.is_empty() {
        return Ok(Invocation {
            command: Command::Help { topic: None },
            json,
            catalog,
            author,
        });
    }

    let name = rest.remove(0);
    let command = match name.as_str() {
        "init" => {
            expect_empty(&rest, "init")?;
            Command::Init
        }
        "new" => parse_new(rest)?,
        "revise" => parse_revise(rest)?,
        "show" => Command::Show {
            plan: one_positional(rest, "show", "<plan>")?,
        },
        "ready" => Command::Ready {
            plan: one_positional(rest, "ready", "<plan>")?,
        },
        "progress" => parse_progress(rest)?,
        "evidence" => parse_evidence(rest)?,
        "status" => {
            expect_empty(&rest, "status")?;
            Command::Status
        }
        "reconcile" => parse_reconcile(rest)?,
        "verify" => parse_verify(rest)?,
        "version" | "--version" | "-V" => Command::Version,
        "help" | "--help" | "-h" => Command::Help {
            topic: rest.first().cloned(),
        },
        other => {
            return Err(format!(
                "unknown command `{other}`\n  run `compass help` for the command list"
            ))
        }
    };

    Ok(Invocation {
        command,
        json,
        catalog,
        author,
    })
}

fn expect_empty(rest: &[String], cmd: &str) -> Result<(), String> {
    match rest.first() {
        None => Ok(()),
        Some(a) => Err(format!("`{cmd}` takes no arguments, got `{a}`")),
    }
}

fn one_positional(rest: Vec<String>, cmd: &str, spec: &str) -> Result<String, String> {
    let mut it = rest.into_iter();
    let first = it
        .next()
        .ok_or_else(|| format!("`{cmd}` needs {spec}\n  usage: compass {cmd} {spec}"))?;
    if first.starts_with('-') {
        return Err(format!("`{cmd}` needs {spec}, got flag `{first}`"));
    }
    match it.next() {
        None => Ok(first),
        Some(extra) => Err(format!("`{cmd}` takes only {spec}, got `{extra}`")),
    }
}

/// Accumulates step edits from the positional-grouping grammar.
#[derive(Default)]
struct StepEditor {
    steps: Vec<StepEdit>,
}

impl StepEditor {
    fn open_add(&mut self, work: String) {
        self.steps.push(StepEdit::Add {
            work,
            accept: None,
            depends_on: Vec::new(),
            supersedes: None,
        });
    }

    fn open_edit(&mut self, id: String) {
        self.steps.push(StepEdit::Edit {
            id,
            work: None,
            accept: None,
            depends_on: None,
            supersedes: None,
        });
    }

    fn retire(&mut self, id: String) {
        self.steps.push(StepEdit::Retire { id });
    }

    fn set_work(&mut self, v: String) -> Result<(), String> {
        match self.steps.last_mut() {
            Some(StepEdit::Edit { work, .. }) => {
                *work = Some(v);
                Ok(())
            }
            Some(StepEdit::Add { work, .. }) => {
                *work = v;
                Ok(())
            }
            _ => Err("`--work` must follow `--add-step` or `--edit-step`".into()),
        }
    }

    fn set_accept(&mut self, v: String) -> Result<(), String> {
        match self.steps.last_mut() {
            Some(StepEdit::Add { accept, .. }) | Some(StepEdit::Edit { accept, .. }) => {
                *accept = Some(v);
                Ok(())
            }
            _ => Err("`--accept` must follow `--add-step` or `--edit-step`".into()),
        }
    }

    fn add_dep(&mut self, v: String) -> Result<(), String> {
        match self.steps.last_mut() {
            Some(StepEdit::Add { depends_on, .. }) => {
                depends_on.push(v);
                Ok(())
            }
            Some(StepEdit::Edit { depends_on, .. }) => {
                depends_on.get_or_insert_with(Vec::new).push(v);
                Ok(())
            }
            _ => Err("`--depends-on` must follow `--add-step` or `--edit-step`".into()),
        }
    }

    fn set_supersedes(&mut self, v: String) -> Result<(), String> {
        match self.steps.last_mut() {
            Some(StepEdit::Add { supersedes, .. }) | Some(StepEdit::Edit { supersedes, .. }) => {
                *supersedes = Some(v);
                Ok(())
            }
            _ => Err("`--supersedes` must follow `--add-step` or `--edit-step`".into()),
        }
    }

    /// Handle a step-editing flag. Returns false if the flag is not one.
    fn try_flag(
        &mut self,
        flag: &str,
        it: &mut std::vec::IntoIter<String>,
    ) -> Result<bool, String> {
        let value = |it: &mut std::vec::IntoIter<String>| -> Result<String, String> {
            it.next().ok_or_else(|| format!("`{flag}` needs a value"))
        };
        match flag {
            "--add-step" => self.open_add(value(it)?),
            "--edit-step" => self.open_edit(value(it)?),
            "--retire-step" => self.retire(value(it)?),
            "--work" => self.set_work(value(it)?)?,
            "--accept" => self.set_accept(value(it)?)?,
            "--depends-on" => self.add_dep(value(it)?)?,
            "--supersedes" => self.set_supersedes(value(it)?)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

fn parse_new(rest: Vec<String>) -> Result<Command, String> {
    let mut goal = None;
    let mut why = None;
    let mut editor = StepEditor::default();
    let mut it = rest.into_iter();

    while let Some(a) = it.next() {
        match a.as_str() {
            "--goal" => goal = Some(it.next().ok_or("`--goal` needs text")?),
            "--why" => why = Some(it.next().ok_or("`--why` needs text")?),
            flag if editor.try_flag(flag, &mut it)? => {}
            other => return Err(unexpected("new", other)),
        }
    }

    Ok(Command::New {
        goal: goal.ok_or("`new` needs `--goal <text>`")?,
        why: why.ok_or("`new` needs `--why <text>`: every version states why (CMP-R03)")?,
        steps: editor.steps,
    })
}

fn parse_revise(mut rest: Vec<String>) -> Result<Command, String> {
    let plan = take_plan(&mut rest, "revise")?;
    let mut why = None;
    let mut goal = None;
    let mut retire = false;
    let mut editor = StepEditor::default();
    let mut it = rest.into_iter();

    while let Some(a) = it.next() {
        match a.as_str() {
            "--why" => why = Some(it.next().ok_or("`--why` needs text")?),
            "--goal" => goal = Some(it.next().ok_or("`--goal` needs text")?),
            "--retire" => retire = true,
            flag if editor.try_flag(flag, &mut it)? => {}
            other => return Err(unexpected("revise", other)),
        }
    }

    Ok(Command::Revise {
        plan,
        why: why.ok_or("`revise` needs `--why <text>`: every version states why (CMP-R03)")?,
        goal,
        retire,
        steps: editor.steps,
    })
}

fn parse_reconcile(mut rest: Vec<String>) -> Result<Command, String> {
    let plan = take_plan(&mut rest, "reconcile")?;
    let mut why = None;
    let mut from = None;
    let mut editor = StepEditor::default();
    let mut it = rest.into_iter();

    while let Some(a) = it.next() {
        match a.as_str() {
            "--why" => why = Some(it.next().ok_or("`--why` needs text")?),
            "--from" => from = Some(it.next().ok_or("`--from` needs a version hash")?),
            flag if editor.try_flag(flag, &mut it)? => {}
            other => return Err(unexpected("reconcile", other)),
        }
    }

    Ok(Command::Reconcile {
        plan,
        why: why
            .ok_or("`reconcile` needs `--why <text>`: a reconciliation states why (CMP-R06)")?,
        from,
        steps: editor.steps,
    })
}

fn parse_progress(mut rest: Vec<String>) -> Result<Command, String> {
    let plan = take_plan(&mut rest, "progress")?;
    if rest.len() < 2 {
        return Err(
            "usage: compass progress <plan> <step> start|update|handoff|done [--note <text>]"
                .into(),
        );
    }
    let step = rest.remove(0);
    let kind = rest.remove(0);
    if !crate::event::EventKind::PROGRESS_KINDS.contains(&kind.as_str()) {
        return Err(format!(
            "unknown progress kind `{kind}`; expected one of {}\n  \
             (`evidence` has its own command, because it carries attributes)",
            crate::event::EventKind::PROGRESS_KINDS.join(", ")
        ));
    }

    let mut note = None;
    let mut it = rest.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--note" => note = Some(it.next().ok_or("`--note` needs text")?),
            other => return Err(unexpected("progress", other)),
        }
    }

    Ok(Command::Progress {
        plan,
        step,
        kind,
        note,
    })
}

fn parse_evidence(mut rest: Vec<String>) -> Result<Command, String> {
    let plan = take_plan(&mut rest, "evidence")?;
    if rest.len() < 2 {
        return Err("usage: compass evidence <plan> <step> <kind> [k=v ...]".into());
    }
    let step = rest.remove(0);
    let kind = rest.remove(0);

    let mut attrs = Vec::new();
    for a in rest {
        let Some((k, v)) = a.split_once('=') else {
            return Err(format!("evidence attribute `{a}` is not `key=value`"));
        };
        if k.is_empty() {
            return Err(format!("evidence attribute `{a}` has an empty key"));
        }
        attrs.push((k.to_string(), v.to_string()));
    }

    Ok(Command::Evidence {
        plan,
        step,
        kind,
        attrs,
    })
}

fn parse_verify(rest: Vec<String>) -> Result<Command, String> {
    let mut plan = None;
    let mut all = false;
    for a in rest {
        match a.as_str() {
            "--all" => all = true,
            other if other.starts_with('-') => return Err(unexpected("verify", other)),
            other => {
                if plan.is_some() {
                    return Err("`verify` takes one plan, or `--all`".into());
                }
                plan = Some(other.to_string());
            }
        }
    }
    if plan.is_none() && !all {
        return Err("usage: compass verify <plan> | compass verify --all".into());
    }
    if plan.is_some() && all {
        return Err("`verify` takes a plan or `--all`, not both".into());
    }
    Ok(Command::Verify { plan, all })
}

fn take_plan(rest: &mut Vec<String>, cmd: &str) -> Result<String, String> {
    if rest.is_empty() {
        return Err(format!("`{cmd}` needs <plan>"));
    }
    if rest[0].starts_with('-') {
        return Err(format!(
            "`{cmd}` needs <plan> first, got flag `{}`",
            rest[0]
        ));
    }
    Ok(rest.remove(0))
}

fn unexpected(cmd: &str, arg: &str) -> String {
    format!("`{cmd}`: unexpected argument `{arg}`\n  run `compass help {cmd}` for usage")
}

/// The help text for a topic, or the overview.
pub fn help(topic: Option<&str>) -> String {
    match topic {
        Some("new") => "\
compass new --goal <text> --why <text> [step edits]

  Create a Plan and its first Version.

  --goal <text>          the intent being pursued
  --why <text>           required Rationale for this version

  Step edits:
    --add-step <work>    open a new step
    --accept <pred>      its acceptance predicate (required per step)
    --depends-on <ref>   repeatable
    --supersedes <ref>   the StepRef this replaces
"
        .to_string(),
        Some("revise") => "\
compass revise <plan> --why <text> [step edits]

  Write a new Version continuing from the current Head.
  Fails when Head has more than one member; reconcile first.

  --why <text>           required Rationale
  --goal <text>          restate the goal
  --retire               retire the whole plan

  Step edits:
    --add-step <work> [--accept <pred>] [--depends-on <ref>]... [--supersedes <ref>]
    --edit-step <ref> [--work <text>] [--accept <pred>] [--depends-on <ref>]...
    --retire-step <ref>
"
        .to_string(),
        Some("reconcile") => "\
compass reconcile <plan> --why <text> [--from <version>] [step edits]

  Resolve a Divergence with an ordinary Version naming every Head member
  as a predecessor.

  --why <text>           required Rationale
  --from <version>       which Head member's step graph to carry forward.
                         Required when the sides differ: Compass never picks
                         a side for you.

  Reconciliation is never offered as the repair for an Orphan.
"
        .to_string(),
        Some("evidence") => "\
compass evidence <plan> <step> <kind> [k=v ...]

  Record an evidence event. Evidence is the only thing acceptance evaluates.

  Example:
    compass evidence pl_ABC st_XYZ test name=parser::nested status=pass
"
        .to_string(),
        Some("predicates") | Some("accept") => "\
Acceptance predicates

  atom      kind(k=v, ...)   an evidence event of `kind` with all these attrs
  all(...)  every argument holds
  any(...)  at least one argument holds
  not(p)    p does not hold

  Examples:
    test(name=parser::nested, status=pass)
    all(test(status=pass), review(by=cos))
    not(test(status=fail))
"
        .to_string(),
        Some(other) => format!("no help topic `{other}`\n\n{}", overview()),
        None => overview(),
    }
}

fn overview() -> String {
    "\
compass — durable planning intent for coding agents

usage: compass <command> [options]

  init                                 create the catalog
  new --goal <t> --why <t>             create a Plan
  revise <plan> --why <t>              write a new Version from Head
  show <plan>                          lineage and the Rationale chain
  ready <plan>                         what work is available now
  progress <plan> <step> <kind>        record start|update|handoff|done
  evidence <plan> <step> <kind> k=v    record evidence acceptance evaluates
  status                               every plan, with convergence state
  reconcile <plan> --why <t>           resolve a Divergence
  verify <plan> | --all                check admission and the chain
  version                              build identity

global options:
  --json                machine-readable output, same fields as the human form
  --catalog <path>      override the catalog root
  --author <name>       override the recorded author
  -h, --help [topic]    help; topics: new, revise, reconcile, evidence, predicates

The catalog root is $COMPASS_CATALOG, else $XDG_STATE_HOME/compass/catalog,
else ~/.local/state/compass/catalog.
"
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Invocation, String> {
        parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn parses_bare_commands() {
        assert_eq!(p(&["init"]).unwrap().command, Command::Init);
        assert_eq!(p(&["status"]).unwrap().command, Command::Status);
        assert_eq!(p(&["version"]).unwrap().command, Command::Version);
    }

    #[test]
    fn no_arguments_shows_help() {
        assert_eq!(p(&[]).unwrap().command, Command::Help { topic: None });
    }

    #[test]
    fn global_flags_are_position_independent() {
        let a = p(&["--json", "status"]).unwrap();
        let b = p(&["status", "--json"]).unwrap();
        assert!(a.json && b.json);
        assert_eq!(a.command, b.command);

        let c = p(&["--catalog", "/tmp/x", "--author", "cos", "status"]).unwrap();
        assert_eq!(c.catalog, Some(PathBuf::from("/tmp/x")));
        assert_eq!(c.author.as_deref(), Some("cos"));
    }

    #[test]
    fn new_requires_goal_and_rationale() {
        assert!(p(&["new", "--goal", "g"]).is_err(), "why is required");
        assert!(p(&["new", "--why", "w"]).is_err(), "goal is required");
        assert_eq!(
            p(&["new", "--goal", "g", "--why", "w"]).unwrap().command,
            Command::New {
                goal: "g".into(),
                why: "w".into(),
                steps: vec![]
            }
        );
    }

    #[test]
    fn step_flags_attach_to_the_most_recent_step() {
        let cmd = p(&[
            "new",
            "--goal",
            "g",
            "--why",
            "w",
            "--add-step",
            "first",
            "--accept",
            "test(status=pass)",
            "--depends-on",
            "st_1",
            "--depends-on",
            "st_2",
            "--add-step",
            "second",
            "--accept",
            "review(by=cos)",
        ])
        .unwrap()
        .command;

        let Command::New { steps, .. } = cmd else {
            panic!()
        };
        assert_eq!(steps.len(), 2);
        assert_eq!(
            steps[0],
            StepEdit::Add {
                work: "first".into(),
                accept: Some("test(status=pass)".into()),
                depends_on: vec!["st_1".into(), "st_2".into()],
                supersedes: None,
            }
        );
        assert_eq!(
            steps[1],
            StepEdit::Add {
                work: "second".into(),
                accept: Some("review(by=cos)".into()),
                depends_on: vec![],
                supersedes: None,
            }
        );
    }

    #[test]
    fn a_step_flag_without_an_open_step_is_an_error() {
        assert!(p(&["new", "--goal", "g", "--why", "w", "--accept", "x()"]).is_err());
        assert!(p(&["revise", "pl_1", "--why", "w", "--depends-on", "st_1"]).is_err());
    }

    #[test]
    fn parses_step_edits_and_retirements() {
        let cmd = p(&[
            "revise",
            "pl_1",
            "--why",
            "w",
            "--edit-step",
            "st_1",
            "--work",
            "new wording",
            "--retire-step",
            "st_2",
        ])
        .unwrap()
        .command;
        let Command::Revise { steps, plan, .. } = cmd else {
            panic!()
        };
        assert_eq!(plan, "pl_1");
        assert_eq!(
            steps[0],
            StepEdit::Edit {
                id: "st_1".into(),
                work: Some("new wording".into()),
                accept: None,
                depends_on: None,
                supersedes: None,
            }
        );
        assert_eq!(steps[1], StepEdit::Retire { id: "st_2".into() });
    }

    #[test]
    fn revise_requires_a_rationale() {
        assert!(p(&["revise", "pl_1"]).is_err());
        assert!(p(&["revise", "--why", "w"]).is_err(), "plan comes first");
    }

    #[test]
    fn parses_progress_and_rejects_evidence_as_a_progress_kind() {
        assert_eq!(
            p(&["progress", "pl_1", "st_1", "start", "--note", "n"])
                .unwrap()
                .command,
            Command::Progress {
                plan: "pl_1".into(),
                step: "st_1".into(),
                kind: "start".into(),
                note: Some("n".into()),
            }
        );
        for k in ["update", "handoff", "done"] {
            assert!(p(&["progress", "pl_1", "st_1", k]).is_ok());
        }
        assert!(
            p(&["progress", "pl_1", "st_1", "evidence"]).is_err(),
            "evidence has its own command"
        );
        assert!(p(&["progress", "pl_1", "st_1", "invented"]).is_err());
    }

    #[test]
    fn parses_evidence_attributes() {
        let cmd = p(&[
            "evidence",
            "pl_1",
            "st_1",
            "test",
            "name=parser::nested",
            "status=pass",
        ])
        .unwrap()
        .command;
        assert_eq!(
            cmd,
            Command::Evidence {
                plan: "pl_1".into(),
                step: "st_1".into(),
                kind: "test".into(),
                attrs: vec![
                    ("name".into(), "parser::nested".into()),
                    ("status".into(), "pass".into()),
                ],
            }
        );
    }

    #[test]
    fn evidence_values_may_contain_equals_signs() {
        let cmd = p(&["evidence", "pl_1", "st_1", "link", "url=http://x/?a=b"])
            .unwrap()
            .command;
        let Command::Evidence { attrs, .. } = cmd else {
            panic!()
        };
        assert_eq!(attrs[0].1, "http://x/?a=b");
    }

    #[test]
    fn rejects_malformed_evidence_attributes() {
        assert!(p(&["evidence", "pl_1", "st_1", "test", "noequals"]).is_err());
        assert!(p(&["evidence", "pl_1", "st_1", "test", "=novalue"]).is_err());
        assert!(p(&["evidence", "pl_1"]).is_err());
    }

    #[test]
    fn verify_takes_a_plan_or_all_but_not_both() {
        assert_eq!(
            p(&["verify", "pl_1"]).unwrap().command,
            Command::Verify {
                plan: Some("pl_1".into()),
                all: false
            }
        );
        assert_eq!(
            p(&["verify", "--all"]).unwrap().command,
            Command::Verify {
                plan: None,
                all: true
            }
        );
        assert!(p(&["verify"]).is_err());
        assert!(p(&["verify", "pl_1", "--all"]).is_err());
    }

    #[test]
    fn reconcile_requires_a_rationale_and_accepts_a_side() {
        assert!(p(&["reconcile", "pl_1"]).is_err());
        assert_eq!(
            p(&["reconcile", "pl_1", "--why", "w", "--from", "abc123"])
                .unwrap()
                .command,
            Command::Reconcile {
                plan: "pl_1".into(),
                why: "w".into(),
                from: Some("abc123".into()),
                steps: vec![],
            }
        );
    }

    #[test]
    fn double_dash_stops_flag_interpretation() {
        let cmd = p(&["new", "--goal", "g", "--why", "--", "--json"]).unwrap();
        // `--` protects a literal value that looks like a flag.
        assert!(!cmd.json);
    }

    #[test]
    fn unknown_commands_and_flags_are_rejected_with_guidance() {
        let e = p(&["frobnicate"]).unwrap_err();
        assert!(e.contains("unknown command"), "{e}");
        let e = p(&["status", "--nope"]).unwrap_err();
        assert!(
            e.contains("unexpected") || e.contains("takes no arguments"),
            "{e}"
        );
    }

    #[test]
    fn help_topics_render_something_useful() {
        assert!(help(None).contains("compass"));
        for t in ["new", "revise", "reconcile", "evidence", "predicates"] {
            assert!(help(Some(t)).len() > 40, "topic {t} is thin");
        }
        assert!(help(Some("nonexistent")).contains("no help topic"));
    }

    #[test]
    fn single_positional_commands_reject_extra_arguments() {
        assert!(p(&["show", "pl_1", "extra"]).is_err());
        assert!(p(&["ready"]).is_err());
        assert!(p(&["init", "extra"]).is_err());
    }
}
