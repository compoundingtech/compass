//! What each Plan Version changed, derived by comparing it to its parents.
//!
//! The lineage says a version exists and why its author wrote it. It does not
//! say what the version *did*, and a reader left to diff two step lists by eye
//! cannot check the Rationale against the change it claims to explain. This
//! module derives the structural change so the two can be read together.
//!
//! Nothing here is stored. A version is a full snapshot of intent (CMP-R05),
//! so the change is recoverable at any time from the versions themselves, and
//! storing it would create a second record able to disagree with the first.
//!
//! Where the change is *not* derivable, this says so rather than guessing. A
//! reconciliation records which versions it succeeded but not which side's
//! step graph it carried forward, so unless the sides agreed there is no base
//! to diff against — and inventing one would be a false account of what an
//! author did.

use crate::catalog::PlanStore;
use crate::model::{Step, Version};

/// What the change is measured against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Basis {
    /// No predecessor: the version states intent rather than changing it.
    Root,
    /// Exactly one predecessor.
    Parent(String),
    /// Several predecessors that carried identical step graphs, so the graph
    /// they agreed on is an unambiguous base.
    Agreed(usize),
    /// Several predecessors that disagreed. Which one was carried forward is
    /// not recorded, so no diff exists to report.
    Unrecoverable(usize),
}

/// One structural difference between a version and its base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepChange {
    Added {
        id: String,
        work: String,
    },
    /// A new step replacing an existing one: intended work changed identity.
    Superseded {
        old: String,
        id: String,
        work: String,
    },
    Retired {
        id: String,
        work: String,
    },
    /// Same identity, different content. `fields` names what moved.
    Edited {
        id: String,
        work: String,
        fields: Vec<&'static str>,
    },
    /// Present in the base and absent here. Revision cannot do this; only a
    /// reconciliation that carried forward a side lacking the step.
    Dropped {
        id: String,
        work: String,
    },
}

impl StepChange {
    pub fn verb(&self) -> &'static str {
        match self {
            StepChange::Added { .. } => "added",
            StepChange::Superseded { .. } => "superseded",
            StepChange::Retired { .. } => "retired",
            StepChange::Edited { .. } => "edited",
            StepChange::Dropped { .. } => "dropped",
        }
    }

    pub fn id(&self) -> &str {
        match self {
            StepChange::Added { id, .. }
            | StepChange::Superseded { id, .. }
            | StepChange::Retired { id, .. }
            | StepChange::Edited { id, .. }
            | StepChange::Dropped { id, .. } => id,
        }
    }

    pub fn work(&self) -> &str {
        match self {
            StepChange::Added { work, .. }
            | StepChange::Superseded { work, .. }
            | StepChange::Retired { work, .. }
            | StepChange::Edited { work, .. }
            | StepChange::Dropped { work, .. } => work,
        }
    }
}

/// The full structural change a version introduced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionChange {
    pub basis: Basis,
    /// The previous goal, present only when this version changed it.
    pub goal_before: Option<String>,
    pub steps: Vec<StepChange>,
    /// The step set this version carries. Reported when there is no base to
    /// diff against, so the reader still learns what intent now stands.
    pub resulting: Vec<String>,
}

impl VersionChange {
    /// Whether this version altered structural intent at all.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty() && self.goal_before.is_none()
    }
}

/// Derive what `version` changed, relative to its recorded parents.
pub fn of(store: &PlanStore, version: &Version) -> VersionChange {
    let resulting: Vec<String> = version.steps.iter().map(|s| s.id.clone()).collect();

    // A parent naming a version that has not arrived is an orphan, not a
    // change of nothing. Treat it as no base rather than diffing against an
    // empty step list, which would report every step as newly added.
    let parents: Vec<&Version> = version
        .parents
        .iter()
        .filter_map(|h| store.version(h).map(|a| &a.version))
        .collect();

    if version.parents.is_empty() {
        return VersionChange {
            basis: Basis::Root,
            goal_before: None,
            steps: diff(&[], &version.steps),
            resulting,
        };
    }

    if parents.len() != version.parents.len() {
        return VersionChange {
            basis: Basis::Unrecoverable(version.parents.len()),
            goal_before: None,
            steps: Vec::new(),
            resulting,
        };
    }

    let (basis, base_steps, base_goal) = match parents.as_slice() {
        [p] => (
            Basis::Parent(version.parents[0].clone()),
            p.steps.clone(),
            Some(p.goal.clone()),
        ),
        many => {
            let first = &many[0].steps;
            if many.iter().all(|p| p.steps == *first) {
                let goal = many[0].goal.clone();
                let goal = many.iter().all(|p| p.goal == goal).then_some(goal);
                (Basis::Agreed(many.len()), first.clone(), goal)
            } else {
                return VersionChange {
                    basis: Basis::Unrecoverable(many.len()),
                    goal_before: None,
                    steps: Vec::new(),
                    resulting,
                };
            }
        }
    };

    let goal_before = base_goal.filter(|g| *g != version.goal);

    VersionChange {
        basis,
        goal_before,
        steps: diff(&base_steps, &version.steps),
        resulting,
    }
}

/// Compare two step graphs.
///
/// A superseding step is a *new* step carrying `supersedes`, and the step it
/// replaces stays in the graph — so it is reported as a supersession rather
/// than an unrelated addition, and the step it replaces is not reported as
/// having vanished.
fn diff(base: &[Step], next: &[Step]) -> Vec<StepChange> {
    let mut out = Vec::new();

    for st in next {
        match base.iter().find(|b| b.id == st.id) {
            None => {
                let replaces = st
                    .supersedes
                    .as_deref()
                    .filter(|prev| base.iter().any(|b| b.id == *prev));
                out.push(match replaces {
                    Some(old) => StepChange::Superseded {
                        old: old.to_string(),
                        id: st.id.clone(),
                        work: st.work.clone(),
                    },
                    None => StepChange::Added {
                        id: st.id.clone(),
                        work: st.work.clone(),
                    },
                });
            }
            Some(prev) => {
                if !prev.retired && st.retired {
                    out.push(StepChange::Retired {
                        id: st.id.clone(),
                        work: st.work.clone(),
                    });
                }
                // Retiring and revising in one version are two changes, and
                // reporting only the retirement would hide the other.
                let mut fields: Vec<&'static str> = Vec::new();
                if prev.work != st.work {
                    fields.push("work");
                }
                if prev.accept != st.accept {
                    fields.push("accept");
                }
                if prev.depends_on != st.depends_on {
                    fields.push("depends on");
                }
                if prev.supersedes != st.supersedes {
                    fields.push("supersedes");
                }
                if prev.retired && !st.retired {
                    fields.push("no longer retired");
                }
                if !fields.is_empty() {
                    out.push(StepChange::Edited {
                        id: st.id.clone(),
                        work: st.work.clone(),
                        fields,
                    });
                }
            }
        }
    }

    for b in base {
        if !next.iter().any(|st| st.id == b.id) {
            out.push(StepChange::Dropped {
                id: b.id.clone(),
                work: b.work.clone(),
            });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Admitted;
    use crate::predicate::parse as pred;
    use std::path::PathBuf;

    fn step(id: &str, work: &str) -> Step {
        Step::new(id, work, pred("test(status=pass)").unwrap())
    }

    fn version(seq: u64, parents: Vec<String>, goal: &str, steps: Vec<Step>) -> Version {
        Version {
            plan: "pl_1000000000".into(),
            seq,
            parents,
            author: "cos".into(),
            why: "because".into(),
            goal: goal.into(),
            retired: false,
            steps,
        }
    }

    fn store_of(versions: Vec<Version>) -> PlanStore {
        let mut store = PlanStore::default();
        for v in versions {
            store.versions.push(Admitted {
                hash: v.hash(),
                version: v,
                path: PathBuf::new(),
            });
        }
        store
    }

    #[test]
    fn a_root_version_reports_its_initial_steps() {
        let v = version(1, vec![], "Ship", vec![step("st_A000000001", "one")]);
        let c = of(&store_of(vec![v.clone()]), &v);
        assert_eq!(c.basis, Basis::Root);
        assert_eq!(c.steps.len(), 1);
        assert_eq!(c.steps[0].verb(), "added");
    }

    #[test]
    fn reports_add_edit_and_retire_against_a_single_parent() {
        let a = step("st_A000000001", "keep");
        let b = step("st_B000000002", "retire me");
        let parent = version(1, vec![], "Ship", vec![a.clone(), b.clone()]);
        let ph = parent.hash();

        let mut edited = a.clone();
        edited.work = "keep, revised".into();
        let mut retired = b.clone();
        retired.retired = true;
        let child = version(
            2,
            vec![ph.clone()],
            "Ship",
            vec![edited, retired, step("st_C000000003", "new work")],
        );

        let c = of(&store_of(vec![parent, child.clone()]), &child);
        assert_eq!(c.basis, Basis::Parent(ph));
        let verbs: Vec<&str> = c.steps.iter().map(|s| s.verb()).collect();
        assert_eq!(verbs, vec!["edited", "retired", "added"]);
        assert!(matches!(&c.steps[0], StepChange::Edited { fields, .. } if fields == &["work"]));
    }

    #[test]
    fn a_superseding_step_is_not_reported_as_a_plain_addition() {
        let old = step("st_A000000001", "old approach");
        let parent = version(1, vec![], "Ship", vec![old]);
        let ph = parent.hash();

        let mut new = step("st_B000000002", "new approach");
        new.supersedes = Some("st_A000000001".into());
        let child = version(
            2,
            vec![ph],
            "Ship",
            vec![step("st_A000000001", "old approach"), new],
        );

        let c = of(&store_of(vec![parent, child.clone()]), &child);
        assert_eq!(c.steps.len(), 1);
        assert!(matches!(
            &c.steps[0],
            StepChange::Superseded { old, .. } if old == "st_A000000001"
        ));
    }

    #[test]
    fn a_goal_change_is_reported() {
        let parent = version(1, vec![], "Old goal", vec![]);
        let ph = parent.hash();
        let child = version(2, vec![ph], "New goal", vec![]);
        let c = of(&store_of(vec![parent, child.clone()]), &child);
        assert_eq!(c.goal_before.as_deref(), Some("Old goal"));
        assert!(!c.is_empty());
    }

    #[test]
    fn a_reconciliation_of_agreeing_sides_diffs_against_the_agreed_graph() {
        let root = version(1, vec![], "Ship", vec![step("st_A000000001", "one")]);
        let rh = root.hash();
        let l = version(
            2,
            vec![rh.clone()],
            "Ship",
            vec![step("st_A000000001", "one")],
        );
        let r = version(2, vec![rh], "Ship", vec![step("st_A000000001", "one")]);
        let (lh, rh2) = (l.hash(), r.hash());

        let merged = version(
            3,
            vec![lh, rh2],
            "Ship",
            vec![step("st_A000000001", "one"), step("st_B000000002", "two")],
        );
        let c = of(&store_of(vec![root, l, r, merged.clone()]), &merged);
        assert_eq!(c.basis, Basis::Agreed(2));
        assert_eq!(c.steps.len(), 1);
        assert_eq!(c.steps[0].verb(), "added");
    }

    #[test]
    fn a_reconciliation_of_disagreeing_sides_reports_no_diff() {
        let root = version(1, vec![], "Ship", vec![step("st_A000000001", "one")]);
        let rh = root.hash();
        let l = version(
            2,
            vec![rh.clone()],
            "Ship",
            vec![step("st_A000000001", "one"), step("st_B000000002", "two")],
        );
        let r = version(
            2,
            vec![rh],
            "Ship",
            vec![step("st_A000000001", "one"), step("st_C000000003", "three")],
        );
        let (lh, rh2) = (l.hash(), r.hash());

        let merged = version(3, vec![lh, rh2], "Ship", vec![step("st_A000000001", "one")]);
        let c = of(&store_of(vec![root, l, r, merged.clone()]), &merged);
        // Which side was carried forward is not recorded, so no diff is
        // claimed — only what now stands.
        assert_eq!(c.basis, Basis::Unrecoverable(2));
        assert!(c.steps.is_empty());
        assert_eq!(c.resulting, vec!["st_A000000001".to_string()]);
    }

    #[test]
    fn a_missing_predecessor_yields_no_invented_diff() {
        let child = version(
            2,
            vec!["f".repeat(64)],
            "Ship",
            vec![step("st_A000000001", "one")],
        );
        let c = of(&store_of(vec![child.clone()]), &child);
        assert_eq!(c.basis, Basis::Unrecoverable(1));
        assert!(c.steps.is_empty());
    }
}
