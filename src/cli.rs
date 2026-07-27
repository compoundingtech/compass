//! Command-line parsing.
//!
//! Hand-rolled. The spelling is this implementation's; the mapping to surface
//! operations is fixed (04-cli): every command is one operation, no command
//! combines a read with a write, `--json` is available everywhere and carries
//! the same fields as the human rendering, and every read of Plan state reports
//! convergence.

use std::path::PathBuf;

/// Exit code for a usage error, distinct from an operational failure.
pub const EXIT_USAGE: i32 = 2;
/// Exit code for an operational failure.
pub const EXIT_FAILURE: i32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Scaffold a runnable starter module (CMP-R11): one command, no docs, no
    /// identifier — a Plan's identity is derived from its origin at commit
    /// (decision 0017).
    Start {
        goal: Option<String>,
    },
    /// Evaluate a module and store it under its derived PlanId (decision 0017).
    Commit {
        path: PathBuf,
    },
    /// The lineage and current intent.
    Show {
        plan: String,
    },
    /// The Rationale chain — first-class (04-cli).
    History {
        plan: String,
    },
    /// What work is available now.
    Ready {
        plan: String,
    },
    /// Every plan, with convergence state.
    Status,
    /// Check admission and the chain. Read-only.
    Verify {
        plan: Option<String>,
        all: bool,
    },
    /// Author a damage-recording version. Distinct from verify (04-cli).
    Repair {
        plan: String,
    },
    /// Record start | update | handoff | done against a step.
    Progress {
        plan: String,
        step: String,
        kind: String,
        note: Option<String>,
    },
    /// Record evidence — the only thing acceptance evaluates.
    Evidence {
        plan: String,
        step: String,
        kind: String,
        attrs: Vec<(String, String)>,
    },
    /// Build identity.
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
        "start" => parse_start(rest)?,
        "commit" => parse_commit(rest)?,
        "show" => Command::Show {
            plan: one_positional(rest, "show", "<plan>")?,
        },
        "history" => Command::History {
            plan: one_positional(rest, "history", "<plan>")?,
        },
        "ready" => Command::Ready {
            plan: one_positional(rest, "ready", "<plan>")?,
        },
        "status" => {
            expect_empty(&rest, "status")?;
            Command::Status
        }
        "verify" => parse_verify(rest)?,
        "repair" => Command::Repair {
            plan: one_positional(rest, "repair", "<plan>")?,
        },
        "progress" => parse_progress(rest)?,
        "evidence" => parse_evidence(rest)?,
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

fn parse_start(rest: Vec<String>) -> Result<Command, String> {
    let mut goal = None;
    let mut it = rest.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--goal" => goal = Some(it.next().ok_or("`--goal` needs text")?),
            other => {
                return Err(format!(
                    "`start` takes no plan name — identity is derived at commit (decision 0017); \
                     unexpected argument `{other}`\n  usage: compass start [--goal <text>]"
                ))
            }
        }
    }
    Ok(Command::Start { goal })
}

fn parse_commit(rest: Vec<String>) -> Result<Command, String> {
    let mut path = None;
    for a in rest {
        match a.as_str() {
            "--plan" => {
                return Err(
                    "`commit` no longer takes `--plan`: a Plan's identity is derived from \
                            its origin at commit (decision 0017)"
                        .into(),
                )
            }
            other if other.starts_with('-') => {
                return Err(format!("`commit`: unexpected argument `{other}`"))
            }
            other => {
                if path.is_some() {
                    return Err("`commit` takes one <module.ts>".into());
                }
                path = Some(PathBuf::from(other));
            }
        }
    }
    Ok(Command::Commit {
        path: path.ok_or("usage: compass commit <module.ts>")?,
    })
}

fn parse_verify(rest: Vec<String>) -> Result<Command, String> {
    let mut plan = None;
    let mut all = false;
    for a in rest {
        match a.as_str() {
            "--all" => all = true,
            other if other.starts_with('-') => {
                return Err(format!("`verify`: unexpected argument `{other}`"))
            }
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
            other => return Err(format!("`progress`: unexpected argument `{other}`")),
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

/// Help text for a topic, or the overview.
pub fn help(topic: Option<&str>) -> String {
    match topic {
        Some("commit") => "\
compass commit <module.ts>

  Evaluate a module and store it under its derived PlanId (decision 0017).
  A version is the authored module, stored unchanged (decision 0014); a Plan's
  identity is the content hash of its origin, derived here — you name nothing.

  - Committing content already present is a no-op success.
  - New content that revises nothing is refused, with a distinct message.
  - A module uses plan() for a first version, prior.revise({...}) for a
    revision, or reconcile({revises:[...]}) for a reconciliation. Predecessors
    are the version files it imports; the Plan is derived from the origin they
    descend from.
"
        .to_string(),
        Some("start") => "\
compass start [--goal <text>]

  Scaffold a runnable starter module, then print where it is. Edit it and
  `compass commit` it. Nothing to import, configure, name, or look up — a Plan's
  identity is derived from its origin when you commit (decision 0017).
"
        .to_string(),
        Some("evidence") => "\
compass evidence <plan> <step> <kind> [k=v ...]

  Record an evidence event. Evidence is the only thing acceptance evaluates.
  Example:
    compass evidence pl_x fix test name=parser::nested status=pass
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

  start [--goal <t>]                   scaffold a starter module
  commit <module.ts>                   evaluate a module and store it
  show <plan>                          lineage and current intent
  history <plan>                       the Rationale chain
  ready <plan>                         what work is available now
  status                               every plan, with convergence state
  verify <plan> | --all                check admission and the chain
  repair <plan>                        author a damage-recording version
  progress <plan> <step> <kind>        record start|update|handoff|done
  evidence <plan> <step> <kind> k=v    record evidence acceptance evaluates
  version                              build identity

A <plan> is addressed by its PlanId (the origin's hash) or, when unambiguous,
by its goal. A Plan is never named: its identity is derived (decision 0017).

global options:
  --json                machine-readable output, same fields as the human form
  --catalog <path>      override the catalog root
  --author <name>       override the recorded author
  -h, --help [topic]    help; topics: start, commit, evidence

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
        assert_eq!(p(&["status"]).unwrap().command, Command::Status);
        assert_eq!(p(&["version"]).unwrap().command, Command::Version);
        assert_eq!(p(&[]).unwrap().command, Command::Help { topic: None });
    }

    #[test]
    fn start_and_commit() {
        // start names no plan (identity is derived at commit, decision 0017).
        assert_eq!(
            p(&["start", "--goal", "Ship"]).unwrap().command,
            Command::Start {
                goal: Some("Ship".into())
            }
        );
        assert_eq!(
            p(&["start"]).unwrap().command,
            Command::Start { goal: None }
        );
        // a stray positional is refused — there is no plan name to give.
        assert!(p(&["start", "pl_x"]).is_err());
        assert_eq!(
            p(&["commit", "a.ts"]).unwrap().command,
            Command::Commit {
                path: PathBuf::from("a.ts"),
            }
        );
        // `--plan` is gone.
        assert!(p(&["commit", "a.ts", "--plan", "pl_x"]).is_err());
        assert!(p(&["commit"]).is_err());
    }

    #[test]
    fn global_flags_are_position_independent() {
        let a = p(&["--json", "status"]).unwrap();
        let b = p(&["status", "--json"]).unwrap();
        assert!(a.json && b.json);
        let c = p(&["--catalog", "/tmp/x", "--author", "cos", "status"]).unwrap();
        assert_eq!(c.catalog, Some(PathBuf::from("/tmp/x")));
        assert_eq!(c.author.as_deref(), Some("cos"));
    }

    #[test]
    fn verify_takes_a_plan_or_all_but_not_both() {
        assert_eq!(
            p(&["verify", "pl_x"]).unwrap().command,
            Command::Verify {
                plan: Some("pl_x".into()),
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
        assert!(p(&["verify", "pl_x", "--all"]).is_err());
    }

    #[test]
    fn progress_rejects_evidence_as_a_kind() {
        assert!(p(&["progress", "pl_x", "s", "start"]).is_ok());
        assert!(p(&["progress", "pl_x", "s", "evidence"]).is_err());
        assert!(p(&["progress", "pl_x", "s", "nope"]).is_err());
    }

    #[test]
    fn evidence_parses_attributes_including_equals_in_values() {
        let cmd = p(&["evidence", "pl_x", "s", "link", "url=http://x/?a=b"])
            .unwrap()
            .command;
        let Command::Evidence { attrs, .. } = cmd else {
            panic!()
        };
        assert_eq!(attrs[0], ("url".into(), "http://x/?a=b".into()));
        assert!(p(&["evidence", "pl_x", "s", "test", "noequals"]).is_err());
    }

    #[test]
    fn unknown_commands_are_rejected() {
        assert!(p(&["frobnicate"]).unwrap_err().contains("unknown command"));
    }

    #[test]
    fn help_renders() {
        assert!(help(None).contains("compass"));
        for t in ["start", "commit", "evidence"] {
            assert!(help(Some(t)).len() > 40);
        }
    }
}
