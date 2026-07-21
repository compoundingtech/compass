//! Plan Versions and Steps — the documents Compass owns (CMP.INT-R02).
//!
//! A Plan Version is immutable (CMP-R02), carries a required Rationale
//! (CMP-R03), records its author and logical time (CMP-R09), and names each
//! predecessor by content hash. Its identity *is* the hash of its rendered
//! bytes, so rendering must be canonical: same intent, same bytes, same hash,
//! on every machine.

use crate::block::{parse as parse_block, Block, Doc, ParseError};
use crate::predicate::Pred;
use crate::sha256::sha256_hex;

/// File extension for both versions and events (DQ02 — provisional).
pub const EXT: &str = "cmp";

/// Number of hex characters of the content hash embedded in a filename.
///
/// The full hash is the identity; this prefix only makes the file findable and
/// human-referable. Admission always checks the full content, never the prefix
/// alone.
pub const HASH_PREFIX_LEN: usize = 12;

/// A unit of intended work within a Plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// Minted StepRef — never derived from content (decision 0004).
    pub id: String,
    pub work: String,
    /// Sorted StepRefs this step depends on.
    pub depends_on: Vec<String>,
    /// The StepRef this step replaces, when intended work changed identity.
    pub supersedes: Option<String>,
    /// Machine-checkable acceptance (CMP-R11). Required: a step whose
    /// acceptance cannot be evaluated could never complete, and would block
    /// every dependent forever.
    pub accept: Pred,
    pub retired: bool,
}

impl Step {
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
}

/// An immutable, content-addressed snapshot of a Plan's structural intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub plan: String,
    /// Position along this version's lineage. A reading aid, never a key:
    /// divergent versions may share a `seq` and nothing resolves on it.
    pub seq: u64,
    /// Content hash of each predecessor, sorted. Empty for the first version,
    /// one ordinarily, several for a Reconciliation.
    pub parents: Vec<String>,
    pub author: String,
    /// Logical time (CMP-R09). Lamport-style: max(seen) + 1, never wall clock.
    pub at: u64,
    /// Required Rationale (CMP-R03).
    pub why: String,
    pub goal: String,
    pub retired: bool,
    pub steps: Vec<Step>,
}

impl Version {
    /// Render canonically. These bytes are what gets hashed and written.
    ///
    /// Field order is fixed and follows the spec's field table. Set-valued
    /// keys (`parent`, `depends_on`) are sorted, because they carry no order.
    /// Step order is *preserved* as authored: a plan is read top to bottom and
    /// reordering steps is a change to the document, not noise to normalise.
    pub fn render(&self) -> String {
        let mut doc = Doc::new();

        let mut v = Block::new("version", None);
        v.set("plan", &self.plan);
        v.set("seq", self.seq.to_string());
        let mut parents = self.parents.clone();
        parents.sort();
        v.set_many("parent", parents);
        v.set("author", &self.author);
        v.set("at", self.at.to_string());
        v.set("why", &self.why);
        v.set("goal", &self.goal);
        if self.retired {
            v.set("retired", "true");
        }
        doc.push(v);

        for step in &self.steps {
            let mut s = Block::new("step", Some(step.id.clone()));
            s.set("work", &step.work);
            let mut deps = step.depends_on.clone();
            deps.sort();
            s.set_many("depends_on", deps);
            s.set_opt("supersedes", step.supersedes.clone());
            s.set("accept", step.accept.to_string());
            if step.retired {
                s.set("retired", "true");
            }
            doc.push(s);
        }

        doc.render()
    }

    /// The content hash: the identity of this version.
    pub fn hash(&self) -> String {
        sha256_hex(self.render().as_bytes())
    }

    /// Parse a version from its serialized bytes.
    pub fn parse(text: &str) -> Result<Version, ParseError> {
        let doc = parse_block(text)?;

        let vb = doc
            .first("version")
            .ok_or_else(|| ParseError::new("no `@version` block"))?;

        let plan = vb.require("plan")?.to_string();
        let seq = parse_u64(vb.require("seq")?, "seq")?;
        let author = vb.require("author")?.to_string();
        let at = parse_u64(vb.require("at")?, "at")?;
        let why = vb.require("why")?.to_string();
        let goal = vb.require("goal")?.to_string();
        let retired = parse_flag(vb.get("retired"), "version retired")?;

        let mut parents: Vec<String> = vb
            .all("parent")
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        parents.sort();
        parents.dedup();

        let mut steps = Vec::new();
        for sb in doc.of_kind("step") {
            let id = sb
                .arg
                .clone()
                .ok_or_else(|| ParseError::new("`@step` block has no StepRef argument"))?;
            let work = sb.require("work")?.to_string();
            let accept_src = sb.require("accept")?;
            let accept = crate::predicate::parse(accept_src)
                .map_err(|e| ParseError::new(format!("step {id}: cannot parse `accept`: {e}")))?;
            let mut depends_on: Vec<String> = sb
                .all("depends_on")
                .into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            depends_on.sort();
            depends_on.dedup();
            let supersedes = sb.get("supersedes").map(|s| s.to_string());
            let retired = parse_flag(sb.get("retired"), "step retired")?;

            steps.push(Step {
                id,
                work,
                depends_on,
                supersedes,
                accept,
                retired,
            });
        }

        let version = Version {
            plan,
            seq,
            parents,
            author,
            at,
            why,
            goal,
            retired,
            steps,
        };
        version.validate()?;
        Ok(version)
    }

    /// Structural checks that parsing alone does not cover.
    fn validate(&self) -> Result<(), ParseError> {
        let mut seen: Vec<&str> = Vec::new();
        for s in &self.steps {
            if seen.contains(&s.id.as_str()) {
                return Err(ParseError::new(format!(
                    "step {} appears more than once in one version",
                    s.id
                )));
            }
            seen.push(&s.id);
        }
        for s in &self.steps {
            for d in &s.depends_on {
                if d == &s.id {
                    return Err(ParseError::new(format!("step {} depends on itself", s.id)));
                }
                if !seen.contains(&d.as_str()) {
                    return Err(ParseError::new(format!(
                        "step {} depends on {d}, which is not a step of this version",
                        s.id
                    )));
                }
            }
        }
        self.reject_dependency_cycles()?;
        Ok(())
    }

    /// A cycle in `depends_on` makes every step in it permanently unready, and
    /// readiness would explain each step by pointing at the next one forever.
    /// That is a defined answer but a useless one, so a cyclic plan is refused
    /// at the door rather than admitted and then reported as stuck.
    fn reject_dependency_cycles(&self) -> Result<(), ParseError> {
        // Iterative depth-first search with an explicit colour map.
        #[derive(Clone, Copy, PartialEq)]
        enum Mark {
            Unvisited,
            InProgress,
            Done,
        }
        let mut marks: Vec<Mark> = vec![Mark::Unvisited; self.steps.len()];
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
                                return Err(ParseError::new(format!(
                                    "dependency cycle: step {} and {} depend on each other, \
                                     directly or transitively",
                                    self.steps[next].id, self.steps[node].id
                                )));
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

    /// Look up a step by ref.
    pub fn step(&self, id: &str) -> Option<&Step> {
        self.steps.iter().find(|s| s.id == id)
    }

    /// The filename this version is stored under: `<seq>-<hash12>.<ext>`.
    pub fn filename(&self) -> String {
        filename_for(self.seq, &self.hash())
    }
}

/// Build the storage filename for a version.
pub fn filename_for(seq: u64, hash: &str) -> String {
    format!(
        "{:03}-{}.{}",
        seq,
        &hash[..HASH_PREFIX_LEN.min(hash.len())],
        EXT
    )
}

/// Split a version filename into its sequence and hash prefix.
pub fn parse_filename(name: &str) -> Option<(u64, String)> {
    let stem = name.strip_suffix(&format!(".{EXT}"))?;
    let (seq, hash) = stem.split_once('-')?;
    if hash.len() != HASH_PREFIX_LEN || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some((seq.parse().ok()?, hash.to_string()))
}

fn parse_u64(s: &str, field: &str) -> Result<u64, ParseError> {
    s.trim().parse().map_err(|_| {
        ParseError::new(format!(
            "`{field}` must be a non-negative integer, got `{s}`"
        ))
    })
}

fn parse_flag(v: Option<&str>, what: &str) -> Result<bool, ParseError> {
    match v.map(str::trim) {
        None => Ok(false),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(other) => Err(ParseError::new(format!(
            "`{what}` must be `true` or `false`, got `{other}`"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicate::parse as pred;

    fn sample() -> Version {
        let mut a = Step::new(
            "st_A000000001",
            "Reproduce with a failing test",
            pred("test(name=parser::nested_groups, status=fail)").unwrap(),
        );
        a.retired = false;
        let mut b = Step::new(
            "st_B000000002",
            "Fix the tokenizer",
            pred("test(name=parser::nested_groups, status=pass)").unwrap(),
        );
        b.depends_on = vec!["st_A000000001".into()];
        Version {
            plan: "pl_7000000000".into(),
            seq: 1,
            parents: vec![],
            author: "cos".into(),
            at: 1,
            why: "Initial plan. Parser rejects nested groups.".into(),
            goal: "Nested groups parse correctly".into(),
            retired: false,
            steps: vec![a, b],
        }
    }

    #[test]
    fn round_trips_through_render_and_parse() {
        let v = sample();
        let parsed = Version::parse(&v.render()).unwrap();
        assert_eq!(parsed, v);
        assert_eq!(parsed.render(), v.render());
        assert_eq!(parsed.hash(), v.hash());
    }

    #[test]
    fn round_trips_a_reconciliation_with_several_parents() {
        let mut v = sample();
        v.parents = vec!["c3d4".repeat(16), "a1b2".repeat(16)];
        v.seq = 4;
        let parsed = Version::parse(&v.render()).unwrap();
        // Parents come back sorted, so identity does not depend on the order
        // the operator happened to name them.
        assert_eq!(parsed.parents, {
            let mut p = v.parents.clone();
            p.sort();
            p
        });
        assert_eq!(parsed.hash(), v.hash());
    }

    #[test]
    fn round_trips_a_multiline_rationale() {
        let mut v = sample();
        v.why =
            "Reproduction showed the tokenizer,\nnot the grammar, drops it.\n\nRetargeting.".into();
        let parsed = Version::parse(&v.render()).unwrap();
        assert_eq!(parsed.why, v.why);
        assert_eq!(parsed.hash(), v.hash());
    }

    #[test]
    fn round_trips_retirement_flags() {
        let mut v = sample();
        v.retired = true;
        v.steps[0].retired = true;
        v.steps[1].supersedes = Some("st_A000000001".into());
        let parsed = Version::parse(&v.render()).unwrap();
        assert_eq!(parsed, v);
    }

    #[test]
    fn hash_is_stable_across_equal_intent() {
        assert_eq!(sample().hash(), sample().hash());
    }

    #[test]
    fn hash_changes_when_any_field_changes() {
        let base = sample().hash();
        let mut v = sample();
        v.why = "Different reason.".into();
        assert_ne!(v.hash(), base);

        let mut v = sample();
        v.at = 2;
        assert_ne!(v.hash(), base);

        let mut v = sample();
        v.steps[0].work = "Reworded".into();
        assert_ne!(v.hash(), base);
    }

    #[test]
    fn parent_order_does_not_affect_identity() {
        let mut a = sample();
        a.parents = vec!["aa".repeat(32), "bb".repeat(32)];
        let mut b = sample();
        b.parents = vec!["bb".repeat(32), "aa".repeat(32)];
        assert_eq!(a.hash(), b.hash());
    }

    #[test]
    fn attribute_order_in_accept_does_not_affect_identity() {
        let mut a = sample();
        a.steps[0].accept = pred("test(status=fail, name=parser::nested_groups)").unwrap();
        assert_eq!(a.hash(), sample().hash());
    }

    #[test]
    fn requires_a_rationale() {
        let v = sample();
        let text = v
            .render()
            .replace("why = Initial plan. Parser rejects nested groups.\n", "");
        let err = Version::parse(&text).unwrap_err();
        assert!(err.message.contains("why"), "{err}");
    }

    #[test]
    fn requires_each_mandatory_field() {
        for field in ["plan", "seq", "author", "at", "goal"] {
            let v = sample();
            let text: String = v
                .render()
                .lines()
                .filter(|l| !l.starts_with(&format!("{field} = ")))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                Version::parse(&text).is_err(),
                "missing `{field}` should be rejected"
            );
        }
    }

    #[test]
    fn rejects_a_step_without_machine_checkable_acceptance() {
        let v = sample();
        let text: String = v
            .render()
            .lines()
            .filter(|l| !l.starts_with("accept = "))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(Version::parse(&text).is_err());
    }

    #[test]
    fn rejects_a_dependency_on_an_unknown_step() {
        let mut v = sample();
        v.steps[1].depends_on = vec!["st_Z000000000".into()];
        let err = Version::parse(&v.render()).unwrap_err();
        assert!(err.message.contains("not a step"), "{err}");
    }

    #[test]
    fn rejects_a_self_dependency() {
        let mut v = sample();
        v.steps[1].depends_on = vec!["st_B000000002".into()];
        assert!(Version::parse(&v.render()).is_err());
    }

    #[test]
    fn rejects_a_direct_dependency_cycle() {
        let mut v = sample();
        v.steps[0].depends_on = vec!["st_B000000002".into()];
        v.steps[1].depends_on = vec!["st_A000000001".into()];
        let err = Version::parse(&v.render()).unwrap_err();
        assert!(err.message.contains("cycle"), "{err}");
    }

    #[test]
    fn rejects_a_transitive_dependency_cycle() {
        let mut v = sample();
        v.steps.push(Step::new(
            "st_C000000003",
            "Third",
            pred("test(status=pass)").unwrap(),
        ));
        // A -> C -> B -> A
        v.steps[0].depends_on = vec!["st_C000000003".into()];
        v.steps[1].depends_on = vec!["st_A000000001".into()];
        v.steps[2].depends_on = vec!["st_B000000002".into()];
        assert!(Version::parse(&v.render()).is_err());
    }

    #[test]
    fn accepts_a_diamond_which_is_not_a_cycle() {
        let mut v = sample();
        v.steps.push(Step::new(
            "st_C000000003",
            "Third",
            pred("test(status=pass)").unwrap(),
        ));
        v.steps.push(Step::new(
            "st_D000000004",
            "Fourth",
            pred("test(status=pass)").unwrap(),
        ));
        // B and C both depend on A; D depends on both B and C.
        v.steps[1].depends_on = vec!["st_A000000001".into()];
        v.steps[2].depends_on = vec!["st_A000000001".into()];
        v.steps[3].depends_on = vec!["st_B000000002".into(), "st_C000000003".into()];
        assert!(Version::parse(&v.render()).is_ok());
    }

    #[test]
    fn rejects_duplicate_steps() {
        let mut v = sample();
        v.steps[1].id = v.steps[0].id.clone();
        assert!(Version::parse(&v.render()).is_err());
    }

    #[test]
    fn rejects_a_non_boolean_retired_flag() {
        let v = sample();
        let text = v.render().replace("goal = ", "retired = yes\ngoal = ");
        assert!(Version::parse(&text).is_err());
    }

    #[test]
    fn filenames_embed_seq_and_hash_prefix() {
        let v = sample();
        let name = v.filename();
        let (seq, prefix) = parse_filename(&name).unwrap();
        assert_eq!(seq, 1);
        assert_eq!(prefix, v.hash()[..HASH_PREFIX_LEN]);
        assert!(name.ends_with(".cmp"));
        assert!(name.starts_with("001-"));
    }

    #[test]
    fn filename_parsing_rejects_foreign_names() {
        assert!(parse_filename("README.md").is_none());
        assert!(parse_filename("001-nothex000000.cmp").is_none());
        assert!(parse_filename("001-abc.cmp").is_none(), "short hash");
        assert!(parse_filename("noseparator.cmp").is_none());
    }

    #[test]
    fn a_version_with_no_steps_is_valid() {
        let mut v = sample();
        v.steps.clear();
        assert_eq!(Version::parse(&v.render()).unwrap(), v);
    }
}
