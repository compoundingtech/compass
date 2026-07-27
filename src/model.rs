//! The domain model of a Plan Version, recovered by evaluation.
//!
//! A version is a module (decision 0014): its identity is the SHA-256 of its
//! authored source bytes, and reading it means evaluating it (see [`crate::eval`]).
//! This module holds the *evaluated* shape — goal, rationale, author, and the
//! Steps with their declared identities — plus the structural checks a commit
//! must pass. Nothing here parses or renders a stored form: there is no stored
//! form but the authored source.

use crate::eval::{SemStep, SemVersion};
use crate::predicate::Pred;

/// File extension for a version module.
pub const VERSION_EXT: &str = "ts";
/// File extension for a progress event.
pub const EVENT_EXT: &str = "cmp";
/// Hex characters of the content hash embedded in a filename. The full hash is
/// the identity; this prefix only makes a file findable and referable.
pub const HASH_PREFIX_LEN: usize = 12;

/// A unit of intended work within a Plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// StepRef — the name the step was declared under (decision 0012).
    pub id: String,
    pub work: String,
    /// StepRefs this step depends on, sorted.
    pub depends_on: Vec<String>,
    /// The StepRef this step replaces, when intended work changed identity.
    pub supersedes: Option<String>,
    /// Machine-checkable acceptance (decision 0006).
    pub accept: Pred,
    pub retired: bool,
}

impl Step {
    /// A step with no dependencies, superseding nothing — mostly for tests and
    /// for building a starter version.
    pub fn new(id: impl Into<String>, work: impl Into<String>, accept: Pred) -> Step {
        Step {
            id: id.into(),
            work: work.into(),
            depends_on: Vec::new(),
            supersedes: None,
            accept,
            retired: false,
        }
    }

    fn from_sem(s: &SemStep) -> Step {
        Step {
            id: s.name.clone(),
            work: s.work.clone(),
            depends_on: s.depends_on.clone(),
            supersedes: s.supersedes.clone(),
            accept: s.accept.clone(),
            retired: s.retired,
        }
    }
}

/// An immutable snapshot of a Plan's structural intent, as evaluated.
///
/// `plan`, `seq`, and `parents` are supplied by the catalog (the plan is the
/// directory, the parents are the imported predecessor files, the seq is a
/// reading aid). Everything else is declared by the module and recovered by
/// evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub plan: String,
    /// Position along this version's lineage. A reading aid, never a key.
    pub seq: u64,
    /// Content hashes of each predecessor (resolved), sorted.
    pub parents: Vec<String>,
    pub author: String,
    /// Required Rationale (CMP-R03).
    pub why: String,
    pub goal: String,
    pub retired: bool,
    pub steps: Vec<Step>,
}

impl Version {
    /// Assemble a domain version from an evaluated module plus the catalog-side
    /// facts (which plan, which lineage position, which predecessors).
    pub fn from_sem(plan: &str, seq: u64, parents: Vec<String>, sem: &SemVersion) -> Version {
        let mut parents = parents;
        parents.sort();
        Version {
            plan: plan.to_string(),
            seq,
            parents,
            author: sem.author.clone(),
            why: sem.why.clone(),
            goal: sem.goal.clone(),
            retired: sem.retired,
            steps: sem.steps.iter().map(Step::from_sem).collect(),
        }
    }

    /// Look up a step by ref.
    pub fn step(&self, id: &str) -> Option<&Step> {
        self.steps.iter().find(|s| s.id == id)
    }

    /// Whether this version would record no change of intent against `parent`.
    /// A revision must alter a Step or the goal (CMP.DM-R07b); the rationale and
    /// author are deliberately not compared.
    pub fn changes_nothing_from(&self, parent: &Version) -> bool {
        self.steps == parent.steps && self.goal == parent.goal && self.retired == parent.retired
    }

    /// Structural checks a committed version must pass: no duplicate step, every
    /// dependency present, and no dependency cycle (which would make readiness
    /// unexplainable). These run against an evaluated module at commit time.
    pub fn validate(&self) -> Result<(), String> {
        // A goal is required on every version and is the Plan's human handle
        // (CMP.DM-R18, decision 0017). It is checked on the evaluated value, so a
        // revision that inherits its predecessor's goal passes, and one that
        // states an empty goal is refused.
        if self.goal.trim().is_empty() {
            return Err(
                "a version must state a non-empty goal: it is the human handle for the \
                        plan (CMP.DM-R18)"
                    .to_string(),
            );
        }
        let mut seen: Vec<&str> = Vec::new();
        for s in &self.steps {
            if seen.contains(&s.id.as_str()) {
                return Err(format!("step {} appears more than once", s.id));
            }
            seen.push(&s.id);
        }
        for s in &self.steps {
            for d in &s.depends_on {
                if d == &s.id {
                    return Err(format!("step {} depends on itself", s.id));
                }
                if !seen.contains(&d.as_str()) {
                    return Err(format!(
                        "step {} depends on {d}, which is not a step of this version",
                        s.id
                    ));
                }
            }
        }
        self.reject_dependency_cycles()
    }

    /// A cycle in `depends_on` makes every step in it permanently unready, so a
    /// cyclic plan is refused rather than admitted and then reported as stuck.
    fn reject_dependency_cycles(&self) -> Result<(), String> {
        #[derive(Clone, Copy, PartialEq)]
        enum Mark {
            Unvisited,
            InProgress,
            Done,
        }
        let mut marks = vec![Mark::Unvisited; self.steps.len()];
        let index = |id: &str| self.steps.iter().position(|s| s.id == id);

        for start in 0..self.steps.len() {
            if marks[start] != Mark::Unvisited {
                continue;
            }
            let mut stack: Vec<(usize, usize)> = vec![(start, 0)];
            marks[start] = Mark::InProgress;
            while let Some((node, edge)) = stack.pop() {
                if edge < self.steps[node].depends_on.len() {
                    stack.push((node, edge + 1));
                    let dep = &self.steps[node].depends_on[edge];
                    if let Some(next) = index(dep) {
                        match marks[next] {
                            Mark::InProgress => {
                                return Err(format!(
                                    "dependency cycle: step {} and {} depend on each other",
                                    self.steps[next].id, self.steps[node].id
                                ));
                            }
                            Mark::Unvisited => {
                                marks[next] = Mark::InProgress;
                                stack.push((next, 0));
                            }
                            Mark::Done => {}
                        }
                    }
                } else {
                    marks[node] = Mark::Done;
                }
            }
        }
        Ok(())
    }
}

/// Build the storage filename for a version: `<seq>-<hash12>.ts`.
pub fn filename_for(seq: u64, hash: &str) -> String {
    format!(
        "{:03}-{}.{}",
        seq,
        &hash[..HASH_PREFIX_LEN.min(hash.len())],
        VERSION_EXT
    )
}

/// Split a version filename into its sequence and hash prefix.
pub fn parse_filename(name: &str) -> Option<(u64, String)> {
    let stem = name.strip_suffix(&format!(".{VERSION_EXT}"))?;
    let (seq, hash) = stem.split_once('-')?;
    if hash.len() != HASH_PREFIX_LEN || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some((seq.parse().ok()?, hash.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::SemStep;
    use crate::predicate::parse as pred;

    fn sem_step(name: &str, deps: &[&str]) -> SemStep {
        SemStep {
            name: name.into(),
            work: format!("work for {name}"),
            depends_on: deps.iter().map(|d| d.to_string()).collect(),
            supersedes: None,
            accept: pred("test(status=pass)").unwrap(),
            retired: false,
        }
    }

    fn version(steps: Vec<SemStep>) -> Version {
        let sem = SemVersion {
            author: "cos".into(),
            why: "because".into(),
            goal: "Ship".into(),
            retired: false,
            steps,
        };
        Version::from_sem("pl_x", 1, vec![], &sem)
    }

    #[test]
    fn a_diamond_is_valid_but_a_cycle_is_not() {
        let ok = version(vec![
            sem_step("a", &[]),
            sem_step("b", &["a"]),
            sem_step("c", &["a"]),
            sem_step("d", &["b", "c"]),
        ]);
        assert!(ok.validate().is_ok());

        let cyclic = version(vec![sem_step("a", &["b"]), sem_step("b", &["a"])]);
        assert!(cyclic.validate().unwrap_err().contains("cycle"));
    }

    #[test]
    fn a_dependency_on_an_unknown_step_is_refused() {
        let v = version(vec![sem_step("a", &["ghost"])]);
        assert!(v.validate().unwrap_err().contains("not a step"));
    }

    #[test]
    fn changes_nothing_ignores_rationale_and_author() {
        let a = version(vec![sem_step("a", &[])]);
        let mut b = a.clone();
        b.why = "reworded".into();
        b.author = "someone".into();
        assert!(b.changes_nothing_from(&a));

        let mut c = a.clone();
        c.goal = "New goal".into();
        assert!(!c.changes_nothing_from(&a));
    }

    #[test]
    fn filenames_round_trip() {
        let name = filename_for(2, &"a".repeat(64));
        assert_eq!(name, format!("002-{}.ts", "a".repeat(12)));
        assert_eq!(parse_filename(&name), Some((2, "a".repeat(12))));
        assert!(parse_filename("README.md").is_none());
        assert!(parse_filename("001-short.ts").is_none());
    }
}
