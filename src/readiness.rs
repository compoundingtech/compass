//! Readiness — what work is available now (CMP-R12, CMP-R13, decision 0006).
//!
//! A Step is **ready** when it is not retired, its own acceptance is not yet
//! satisfied, and every step it depends on *is* accepted.
//!
//! Acceptance is judged by Compass from the Step's `accept` predicate against
//! recorded evidence (CMP-R14). A `done` progress event does **not** accept a
//! step: an external observation must never complete a Step. `done` says an
//! actor believes they finished; `accept` says Compass agrees.
//!
//! ## Readiness always explains itself
//!
//! An unexplained answer cannot be trusted or debugged, so every step carries
//! a reason — including the ready ones, which report what would accept them.
//!
//! ## Readiness under Divergence
//!
//! Reported **per head member, labelled**. It does not pick a side and it does
//! not merge the graphs, which would invent intent nobody authored. Divergence
//! is a normal state and the primary query has a defined meaning there: here
//! are the answers, and why they differ.
//!
//! ## On "gates"
//!
//! The spec names "dependencies and gates" as the two things readiness folds
//! over, but defines no gate concept anywhere. This implementation treats the
//! acceptance predicate as the gate — it is the only authored condition a step
//! carries besides its dependencies. Noted as a spec ambiguity.

use crate::catalog::{Admitted, PlanStore};
use crate::event::EventKind;
use crate::model::{Step, Version};
use crate::predicate::Evidence;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    Ready,
    Blocked,
    Accepted,
    Retired,
}

impl StepState {
    pub fn as_str(self) -> &'static str {
        match self {
            StepState::Ready => "ready",
            StepState::Blocked => "blocked",
            StepState::Accepted => "accepted",
            StepState::Retired => "retired",
        }
    }
}

#[derive(Debug, Clone)]
pub struct StepReadiness {
    pub step: String,
    pub work: String,
    pub state: StepState,
    /// Why the step is in this state. Never empty.
    pub reason: String,
    /// StepRefs whose acceptance is holding this step back.
    pub blocked_by: Vec<String>,
    /// The acceptance criterion, canonically rendered.
    pub accept: String,
}

/// Readiness computed against one head member.
#[derive(Debug, Clone)]
pub struct HeadReadiness {
    pub head: String,
    pub seq: u64,
    pub author: String,
    pub at: u64,
    /// A head member can be a head member only because its parent has not
    /// arrived. Its readiness answer is provisional and says so.
    pub orphan: bool,
    pub steps: Vec<StepReadiness>,
}

impl HeadReadiness {
    pub fn ready(&self) -> impl Iterator<Item = &StepReadiness> {
        self.steps.iter().filter(|s| s.state == StepState::Ready)
    }

    pub fn count(&self, state: StepState) -> usize {
        self.steps.iter().filter(|s| s.state == state).count()
    }
}

/// The evidence recorded for a step, following `supersedes` edges backwards.
///
/// An event recorded against a Step that a later version superseded is
/// attributed to the superseding Step, so a rename or a split does not discard
/// the evidence already gathered.
fn evidence_for(store: &PlanStore, version: &Version, step: &Step) -> Vec<Evidence> {
    // Walk the supersedes chain within this version.
    let mut refs: HashSet<&str> = HashSet::new();
    let mut cursor = Some(step);
    while let Some(s) = cursor {
        if !refs.insert(s.id.as_str()) {
            break; // defensive: a supersedes cycle
        }
        cursor = s
            .supersedes
            .as_deref()
            .and_then(|prev| version.step(prev))
            .filter(|prev| !refs.contains(prev.id.as_str()));
    }
    // A supersedes edge may name a step that no longer appears in this
    // version; accept evidence filed against it too.
    if let Some(prev) = step.supersedes.as_deref() {
        refs.insert(prev);
    }

    store
        .events
        .iter()
        .filter(|e| e.kind == EventKind::Evidence && refs.contains(e.step.as_str()))
        .filter_map(|e| e.as_evidence())
        .collect()
}

/// Compute readiness for one head member.
pub fn for_head(store: &PlanStore, head: &Admitted, orphan: bool) -> HeadReadiness {
    let version = &head.version;

    // Accepted-ness of every step, needed before dependencies can be judged.
    //
    // A retired step is never accepted for this purpose, even when evidence
    // matching its predicate exists: "an event against a retired Step is
    // retained but does not contribute to Readiness". Letting a retired step's
    // evidence satisfy a dependency would be exactly that contribution.
    //
    // The consequence is deliberate: retiring a step that others depend on
    // blocks them until the dependency is revised. That is the honest signal —
    // the plan says work must precede other work, and that work has been
    // decommissioned. Silently unblocking would let a retired step license
    // progress nobody authored.
    let accepted: Vec<(String, bool)> = version
        .steps
        .iter()
        .map(|s| {
            if s.retired {
                return (s.id.clone(), false);
            }
            let ev = evidence_for(store, version, s);
            (s.id.clone(), s.accept.eval(&ev))
        })
        .collect();
    let is_accepted = |id: &str| -> bool {
        accepted
            .iter()
            .find(|(sid, _)| sid == id)
            .map(|(_, ok)| *ok)
            .unwrap_or(false)
    };

    let steps = version
        .steps
        .iter()
        .map(|s| {
            let ev = evidence_for(store, version, s);
            let accept = s.accept.to_string();

            if s.retired {
                return StepReadiness {
                    step: s.id.clone(),
                    work: s.work.clone(),
                    state: StepState::Retired,
                    reason: "retired; excluded from readiness".to_string(),
                    blocked_by: vec![],
                    accept,
                };
            }

            if s.accept.eval(&ev) {
                return StepReadiness {
                    step: s.id.clone(),
                    work: s.work.clone(),
                    state: StepState::Accepted,
                    reason: format!("acceptance satisfied: {accept}"),
                    blocked_by: vec![],
                    accept,
                };
            }

            // Unaccepted. Ready only if every dependency is accepted.
            let unmet: Vec<String> = s
                .depends_on
                .iter()
                .filter(|d| !is_accepted(d))
                .cloned()
                .collect();

            if !unmet.is_empty() {
                let detail = unmet
                    .iter()
                    .map(|d| match version.step(d) {
                        Some(dep) if dep.retired => {
                            format!("{d} (retired; a retired step never contributes to readiness)")
                        }
                        Some(dep) => format!("{d} ({})", dep.work),
                        None => format!("{d} (not a step of this version)"),
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                return StepReadiness {
                    step: s.id.clone(),
                    work: s.work.clone(),
                    state: StepState::Blocked,
                    reason: format!("waiting on {detail}"),
                    blocked_by: unmet,
                    accept,
                };
            }

            let reason = s
                .accept
                .explain(&ev)
                .unwrap_or_else(|| format!("needs {accept}"));
            StepReadiness {
                step: s.id.clone(),
                work: s.work.clone(),
                state: StepState::Ready,
                reason,
                blocked_by: vec![],
                accept,
            }
        })
        .collect();

    HeadReadiness {
        head: head.hash.clone(),
        seq: version.seq,
        author: version.author.clone(),
        at: version.at,
        orphan,
        steps,
    }
}

/// Readiness for every head member, labelled. Never picks a side.
pub fn for_plan<'a>(
    store: &'a PlanStore,
    analysis: &crate::chain::Analysis<'a>,
) -> Vec<HeadReadiness> {
    analysis
        .head
        .iter()
        .map(|h| for_head(store, h, analysis.is_orphan(&h.hash)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Admitted;
    use crate::event::Event;
    use crate::model::Step;
    use crate::predicate::parse as pred;
    use std::path::PathBuf;

    const PLAN: &str = "pl_1000000000";
    const A: &str = "st_A000000001";
    const B: &str = "st_B000000002";

    fn admit(version: Version) -> Admitted {
        Admitted {
            hash: version.hash(),
            path: PathBuf::from("/dev/null"),
            version,
        }
    }

    /// Two steps: B depends on A.
    fn two_step_version() -> Version {
        let a = Step::new(
            A,
            "Reproduce with a failing test",
            pred("test(name=x, status=fail)").unwrap(),
        );
        let mut b = Step::new(
            B,
            "Fix the tokenizer",
            pred("test(name=x, status=pass)").unwrap(),
        );
        b.depends_on = vec![A.to_string()];
        Version {
            plan: PLAN.into(),
            seq: 1,
            parents: vec![],
            author: "cos".into(),
            at: 1,
            why: "Initial plan.".into(),
            goal: "Nested groups parse".into(),
            retired: false,
            steps: vec![a, b],
        }
    }

    fn evidence_event(step: &str, kind: &str, attrs: &[(&str, &str)], at: u64) -> Event {
        Event {
            id: format!("ev_{at:010}"),
            at,
            wall: 0,
            plan: PLAN.into(),
            step: step.into(),
            version: "a".repeat(64),
            actor: "cos".into(),
            kind: EventKind::Evidence,
            note: None,
            evidence_kind: Some(kind.into()),
            attrs: attrs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn plain_event(step: &str, kind: EventKind, at: u64) -> Event {
        Event {
            id: format!("ev_{at:010}"),
            at,
            wall: 0,
            plan: PLAN.into(),
            step: step.into(),
            version: "a".repeat(64),
            actor: "cos".into(),
            kind,
            note: None,
            evidence_kind: None,
            attrs: vec![],
        }
    }

    fn store_with(version: Version, events: Vec<Event>) -> (PlanStore, Admitted) {
        let a = admit(version);
        (
            PlanStore {
                plan: PLAN.into(),
                versions: vec![a.clone()],
                events,
                ..Default::default()
            },
            a,
        )
    }

    fn find<'a>(r: &'a HeadReadiness, id: &str) -> &'a StepReadiness {
        r.steps.iter().find(|s| s.step == id).unwrap()
    }

    #[test]
    fn with_no_evidence_only_the_unblocked_step_is_ready() {
        let (s, h) = store_with(two_step_version(), vec![]);
        let r = for_head(&s, &h, false);

        assert_eq!(find(&r, A).state, StepState::Ready);
        assert_eq!(find(&r, B).state, StepState::Blocked);
        assert_eq!(find(&r, B).blocked_by, vec![A.to_string()]);
        assert_eq!(r.count(StepState::Ready), 1);
    }

    #[test]
    fn evidence_accepts_a_step_and_unblocks_its_dependent() {
        let (s, h) = store_with(
            two_step_version(),
            vec![evidence_event(
                A,
                "test",
                &[("name", "x"), ("status", "fail")],
                1,
            )],
        );
        let r = for_head(&s, &h, false);

        assert_eq!(find(&r, A).state, StepState::Accepted);
        assert_eq!(find(&r, B).state, StepState::Ready);
    }

    #[test]
    fn a_done_event_does_not_accept_a_step() {
        // CMP-R14: an external observation must never complete a Step.
        let (s, h) = store_with(
            two_step_version(),
            vec![
                plain_event(A, EventKind::Start, 1),
                plain_event(A, EventKind::Update, 2),
                plain_event(A, EventKind::Handoff, 3),
                plain_event(A, EventKind::Done, 4),
            ],
        );
        let r = for_head(&s, &h, false);

        assert_eq!(
            find(&r, A).state,
            StepState::Ready,
            "done is an actor's claim, not Compass's judgement"
        );
        assert_eq!(find(&r, B).state, StepState::Blocked);
    }

    #[test]
    fn non_matching_evidence_does_not_accept() {
        let (s, h) = store_with(
            two_step_version(),
            vec![evidence_event(
                A,
                "test",
                &[("name", "x"), ("status", "pass")],
                1,
            )],
        );
        // Step A wants status=fail.
        assert_eq!(find(&for_head(&s, &h, false), A).state, StepState::Ready);
    }

    #[test]
    fn evidence_filed_against_another_step_does_not_leak() {
        let (s, h) = store_with(
            two_step_version(),
            vec![evidence_event(
                B,
                "test",
                &[("name", "x"), ("status", "fail")],
                1,
            )],
        );
        assert_eq!(find(&for_head(&s, &h, false), A).state, StepState::Ready);
    }

    #[test]
    fn a_retired_step_is_excluded_and_never_accepts_its_dependents() {
        let mut v = two_step_version();
        v.steps[0].retired = true;
        let (s, h) = store_with(v, vec![]);
        let r = for_head(&s, &h, false);

        assert_eq!(find(&r, A).state, StepState::Retired);
        assert_eq!(find(&r, B).state, StepState::Blocked);
        assert!(
            find(&r, B).reason.contains("retired"),
            "{}",
            find(&r, B).reason
        );
    }

    #[test]
    fn evidence_on_a_retired_step_does_not_unblock_its_dependent() {
        // Spec: an event against a retired Step is retained but does not
        // contribute to Readiness. Matching evidence must therefore not
        // satisfy a dependency on a retired step.
        let mut v = two_step_version();
        v.steps[0].retired = true;
        let (s, h) = store_with(
            v,
            vec![evidence_event(
                A,
                "test",
                &[("name", "x"), ("status", "fail")],
                1,
            )],
        );
        let r = for_head(&s, &h, false);

        assert_eq!(find(&r, A).state, StepState::Retired);
        assert_eq!(
            find(&r, B).state,
            StepState::Blocked,
            "a retired step's evidence must not license its dependent"
        );
        assert!(
            find(&r, B).reason.contains("retired"),
            "{}",
            find(&r, B).reason
        );
    }

    #[test]
    fn evidence_follows_the_supersedes_edge() {
        // B supersedes A; evidence filed against A counts toward B.
        let mut v = two_step_version();
        v.steps[1].supersedes = Some(A.to_string());
        v.steps[1].depends_on = vec![];
        v.steps[1].accept = pred("test(name=x, status=pass)").unwrap();
        let (s, h) = store_with(
            v,
            vec![evidence_event(
                A,
                "test",
                &[("name", "x"), ("status", "pass")],
                1,
            )],
        );
        assert_eq!(find(&for_head(&s, &h, false), B).state, StepState::Accepted);
    }

    #[test]
    fn every_step_carries_a_non_empty_explanation() {
        let mut v = two_step_version();
        v.steps.push({
            let mut c = Step::new("st_C000000003", "Retired work", pred("x()").unwrap());
            c.retired = true;
            c
        });
        let (s, h) = store_with(
            v,
            vec![evidence_event(
                A,
                "test",
                &[("name", "x"), ("status", "fail")],
                1,
            )],
        );
        let r = for_head(&s, &h, false);
        for step in &r.steps {
            assert!(
                !step.reason.trim().is_empty(),
                "{} has no reason",
                step.step
            );
        }
    }

    #[test]
    fn a_blocked_step_names_the_dependency_holding_it() {
        let (s, h) = store_with(two_step_version(), vec![]);
        let r = for_head(&s, &h, false);
        let b = find(&r, B);
        assert!(b.reason.contains(A), "{}", b.reason);
        assert!(b.reason.contains("Reproduce"), "{}", b.reason);
    }

    #[test]
    fn a_ready_step_explains_what_would_accept_it() {
        let (s, h) = store_with(two_step_version(), vec![]);
        let r = for_head(&s, &h, false);
        let a = find(&r, A);
        assert!(
            a.reason.contains("test(name=x, status=fail)"),
            "{}",
            a.reason
        );
    }

    #[test]
    fn readiness_is_reported_per_head_member_under_divergence() {
        let base = two_step_version();

        let mut left = base.clone();
        left.seq = 2;
        left.at = 2;
        left.why = "cos side".into();
        left.parents = vec![admit(base.clone()).hash];

        let mut right = base.clone();
        right.seq = 2;
        right.at = 2;
        right.why = "dev side".into();
        right.author = "dev".into();
        right.parents = vec![admit(base.clone()).hash];
        // The two sides carry genuinely different graphs.
        right.steps.pop();

        let store = PlanStore {
            plan: PLAN.into(),
            versions: vec![admit(base), admit(left), admit(right)],
            ..Default::default()
        };
        let analysis = crate::chain::analyze(&store);
        assert!(analysis.diverged());

        let all = for_plan(&store, &analysis);
        assert_eq!(all.len(), 2, "one answer per head member");
        let authors: HashSet<&str> = all.iter().map(|r| r.author.as_str()).collect();
        assert!(authors.contains("cos") && authors.contains("dev"));
        // The graphs are not merged.
        let sizes: HashSet<usize> = all.iter().map(|r| r.steps.len()).collect();
        assert_eq!(sizes, HashSet::from([1, 2]));
    }

    #[test]
    fn an_orphaned_head_member_is_labelled_as_such() {
        let mut v = two_step_version();
        v.parents = vec!["f".repeat(64)];
        let (s, _h) = store_with(v, vec![]);
        let analysis = crate::chain::analyze(&s);
        let all = for_plan(&s, &analysis);
        assert_eq!(all.len(), 1);
        assert!(
            all[0].orphan,
            "an orphan head must be flagged, not silently served"
        );
    }

    #[test]
    fn composite_acceptance_is_evaluated_and_explained() {
        let mut v = two_step_version();
        v.steps[0].accept = pred("all(test(status=pass), review(by=cos))").unwrap();
        let (s, h) = store_with(v, vec![evidence_event(A, "test", &[("status", "pass")], 1)]);
        let r = for_head(&s, &h, false);
        let a = find(&r, A);
        assert_eq!(a.state, StepState::Ready);
        assert!(a.reason.contains("review(by=cos)"), "{}", a.reason);
    }

    #[test]
    fn acceptance_can_be_revoked_by_later_evidence() {
        let mut v = two_step_version();
        v.steps[0].accept = pred("not(test(status=fail))").unwrap();
        let (s, h) = store_with(v.clone(), vec![]);
        assert_eq!(find(&for_head(&s, &h, false), A).state, StepState::Accepted);

        let (s2, h2) = store_with(v, vec![evidence_event(A, "test", &[("status", "fail")], 1)]);
        assert_eq!(find(&for_head(&s2, &h2, false), A).state, StepState::Ready);
    }

    #[test]
    fn a_version_with_no_steps_yields_no_readiness_rows() {
        let mut v = two_step_version();
        v.steps.clear();
        let (s, h) = store_with(v, vec![]);
        assert!(for_head(&s, &h, false).steps.is_empty());
    }
}
