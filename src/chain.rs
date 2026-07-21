//! Walking the version chain: Head, Divergence, Orphans.
//!
//! Head is **derived, never stored** (CMP-R04): the set of versions with no
//! successor. Ordinarily one member; several when a plan has diverged.
//!
//! ## Divergence is not Orphanhood
//!
//! The ontology is explicit and the distinction is the one Compass must not
//! get wrong:
//!
//! - **Divergence** — two or more versions share the same predecessor. Intent
//!   genuinely disagreed. Repaired by authoring a Reconciliation.
//! - **Orphan** — a version whose predecessor is *absent locally*. Ordinarily
//!   replication is simply incomplete. Repaired by **waiting**.
//!
//! Reconciling around a version that is merely in-flight writes permanent
//! intent to fix a transient condition, so reconciliation is never offered as
//! the repair for an orphan.
//!
//! A version can be both a head member and an orphan: decision 0002
//! Amendment 1 describes receiving versions 1, 2 and 4, where 4's parent has
//! not arrived. Head is then `{2, 4}` — and reporting that as divergence would
//! be a lie, because 2 and 4 share no predecessor.

use crate::catalog::{Admitted, PlanStore};
use std::collections::{BTreeMap, HashSet};

/// A version whose predecessor is not present locally.
#[derive(Debug, Clone)]
pub struct Orphan<'a> {
    pub version: &'a Admitted,
    /// Parent hashes that are absent.
    pub missing: Vec<String>,
}

/// Two or more versions sharing a predecessor.
#[derive(Debug, Clone)]
pub struct Divergence<'a> {
    /// The shared predecessor hash, or `None` when several root versions exist.
    pub parent: Option<String>,
    pub children: Vec<&'a Admitted>,
    /// Whether this divergence is still unresolved.
    ///
    /// A Divergence is a permanent fact of the chain — a Reconciliation does
    /// not erase it, and `show` must keep displaying it, because the argument
    /// the chain records is the point of the tool. But once a Reconciliation
    /// descends from every side, it is **history, not a problem**: continuing
    /// to prompt "fix: reconcile" would nag forever about work already done.
    ///
    /// Open means the sides have no common descendant.
    pub open: bool,
}

/// The derived state of one plan's chain.
#[derive(Debug, Clone)]
pub struct Analysis<'a> {
    /// Versions with no successor. Ordinarily one.
    pub head: Vec<&'a Admitted>,
    pub orphans: Vec<Orphan<'a>>,
    /// Every divergence in the chain, resolved or not.
    pub divergences: Vec<Divergence<'a>>,
}

impl<'a> Analysis<'a> {
    /// Divergences still awaiting a Reconciliation.
    pub fn open_divergences(&self) -> impl Iterator<Item = &Divergence<'a>> {
        self.divergences.iter().filter(|d| d.open)
    }

    /// A plan has diverged when intent disagreed and has not yet been
    /// reconciled — not merely when head has several members, which
    /// orphanhood also causes, and not merely because the chain records a
    /// divergence that was already settled.
    pub fn diverged(&self) -> bool {
        self.divergences.iter().any(|d| d.open)
    }

    /// Whether the chain records a divergence at all, open or settled.
    pub fn ever_diverged(&self) -> bool {
        !self.divergences.is_empty()
    }

    pub fn is_orphan(&self, hash: &str) -> bool {
        self.orphans.iter().any(|o| o.version.hash == hash)
    }

    /// One-word convergence-independent description of chain state.
    pub fn state(&self) -> &'static str {
        if !self.orphans.is_empty() && self.diverged() {
            "diverged+orphaned"
        } else if self.diverged() {
            "diverged"
        } else if !self.orphans.is_empty() {
            "orphaned"
        } else if self.head.len() > 1 {
            "multi-head"
        } else if self.head.is_empty() {
            "empty"
        } else {
            "converged-locally"
        }
    }
}

/// Derive head, divergence and orphans from the admitted versions.
pub fn analyze(store: &PlanStore) -> Analysis<'_> {
    let present: HashSet<&str> = store.versions.iter().map(|a| a.hash.as_str()).collect();

    // A version is a successor of each hash it names as parent.
    let mut has_successor: HashSet<&str> = HashSet::new();
    for a in &store.versions {
        for p in &a.version.parents {
            has_successor.insert(p.as_str());
        }
    }

    let head: Vec<&Admitted> = store
        .versions
        .iter()
        .filter(|a| !has_successor.contains(a.hash.as_str()))
        .collect();

    let orphans: Vec<Orphan> = store
        .versions
        .iter()
        .filter_map(|a| {
            let missing: Vec<String> = a
                .version
                .parents
                .iter()
                .filter(|p| !present.contains(p.as_str()))
                .cloned()
                .collect();
            (!missing.is_empty()).then_some(Orphan {
                version: a,
                missing,
            })
        })
        .collect();

    // Group by predecessor. Only predecessors that are actually present count:
    // two versions both naming an absent parent are two orphans, not a
    // divergence we can reason about.
    let mut by_parent: BTreeMap<&str, Vec<&Admitted>> = BTreeMap::new();
    let mut roots: Vec<&Admitted> = Vec::new();
    for a in &store.versions {
        if a.version.parents.is_empty() {
            roots.push(a);
        }
        for p in &a.version.parents {
            if present.contains(p.as_str()) {
                by_parent.entry(p.as_str()).or_default().push(a);
            }
        }
    }

    // Forward edges, to decide whether a divergence has been reconciled.
    let mut children_of: BTreeMap<&str, Vec<&Admitted>> = BTreeMap::new();
    for a in &store.versions {
        for p in &a.version.parents {
            children_of.entry(p.as_str()).or_default().push(a);
        }
    }
    let head_set: HashSet<&str> = head.iter().map(|a| a.hash.as_str()).collect();

    let mut divergences: Vec<Divergence> = by_parent
        .into_iter()
        .filter(|(_, kids)| kids.len() > 1)
        .map(|(p, children)| Divergence {
            open: is_open(&children, &children_of, &head_set),
            parent: Some(p.to_string()),
            children,
        })
        .collect();

    // Several root versions for one plan is also a genuine disagreement about
    // where the plan starts, though minting makes it rare.
    if roots.len() > 1 {
        divergences.insert(
            0,
            Divergence {
                open: is_open(&roots, &children_of, &head_set),
                parent: None,
                children: roots,
            },
        );
    }

    Analysis {
        head,
        orphans,
        divergences,
    }
}

/// Whether a set of diverged siblings still lacks a common descendant.
///
/// Each side is walked forward to the head members it reaches. If every side
/// reaches some common head, a Reconciliation has already joined them and the
/// divergence is settled. If any two sides reach disjoint heads, intent is
/// still split and the divergence is open.
fn is_open(
    children: &[&Admitted],
    children_of: &BTreeMap<&str, Vec<&Admitted>>,
    head_set: &HashSet<&str>,
) -> bool {
    let mut common: Option<HashSet<String>> = None;
    for c in children {
        let reached = reachable_heads(&c.hash, children_of, head_set);
        common = Some(match common {
            None => reached,
            Some(acc) => acc.intersection(&reached).cloned().collect(),
        });
        if common.as_ref().is_some_and(|c| c.is_empty()) {
            return true;
        }
    }
    common.is_none_or(|c| c.is_empty())
}

/// The head members reachable by walking forward from `start`.
fn reachable_heads(
    start: &str,
    children_of: &BTreeMap<&str, Vec<&Admitted>>,
    head_set: &HashSet<&str>,
) -> HashSet<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut heads: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = vec![start.to_string()];

    while let Some(h) = stack.pop() {
        if !seen.insert(h.clone()) {
            continue;
        }
        if head_set.contains(h.as_str()) {
            heads.insert(h.clone());
        }
        if let Some(kids) = children_of.get(h.as_str()) {
            for k in kids {
                stack.push(k.hash.clone());
            }
        }
    }
    heads
}

/// Every ancestor of `hash` including itself, oldest first.
///
/// This is the Rationale chain — the artifact the tool exists to preserve.
/// Ordering is by logical time, then sequence, then hash, so it is total and
/// stable even across a reconciliation of unequal-length lineages.
pub fn lineage<'a>(store: &'a PlanStore, hash: &str) -> Vec<&'a Admitted> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<&Admitted> = Vec::new();
    let mut queue: Vec<String> = vec![hash.to_string()];

    while let Some(h) = queue.pop() {
        if !seen.insert(h.clone()) {
            continue;
        }
        let Some(a) = store.version(&h) else { continue };
        out.push(a);
        for p in &a.version.parents {
            queue.push(p.clone());
        }
    }

    out.sort_by(|a, b| {
        (a.version.at, a.version.seq, &a.hash).cmp(&(b.version.at, b.version.seq, &b.hash))
    });
    out
}

/// The `seq` a new version should carry: one past the longest predecessor.
pub fn next_seq(parents: &[&Admitted]) -> u64 {
    parents.iter().map(|p| p.version.seq).max().unwrap_or(0) + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Admitted;
    use crate::model::{Step, Version};
    use crate::predicate::parse as pred;
    use std::path::PathBuf;

    fn v(plan: &str, seq: u64, at: u64, why: &str, parents: Vec<String>) -> Admitted {
        let version = Version {
            plan: plan.into(),
            seq,
            parents,
            author: "cos".into(),
            at,
            why: why.into(),
            goal: "Goal".into(),
            retired: false,
            steps: vec![Step::new(
                "st_A000000001",
                "Work",
                pred("test(status=pass)").unwrap(),
            )],
        };
        Admitted {
            hash: version.hash(),
            path: PathBuf::from("/dev/null"),
            version,
        }
    }

    fn store(versions: Vec<Admitted>) -> PlanStore {
        PlanStore {
            plan: "pl_1000000000".into(),
            versions,
            ..Default::default()
        }
    }

    #[test]
    fn a_linear_chain_has_one_head_and_no_divergence() {
        let a = v("pl_1000000000", 1, 1, "first", vec![]);
        let b = v("pl_1000000000", 2, 2, "second", vec![a.hash.clone()]);
        let c = v("pl_1000000000", 3, 3, "third", vec![b.hash.clone()]);
        let want_head = c.hash.clone();
        let s = store(vec![a, b, c]);
        let an = analyze(&s);

        assert_eq!(an.head.len(), 1);
        assert_eq!(an.head[0].hash, want_head);
        assert!(!an.diverged());
        assert!(an.orphans.is_empty());
        assert_eq!(an.state(), "converged-locally");
    }

    #[test]
    fn concurrent_revision_is_divergence_with_two_head_members() {
        let a = v("pl_1000000000", 1, 1, "first", vec![]);
        let left = v("pl_1000000000", 2, 2, "cos side", vec![a.hash.clone()]);
        let right = v("pl_1000000000", 2, 2, "dev side", vec![a.hash.clone()]);
        let parent = a.hash.clone();
        let s = store(vec![a, left, right]);
        let an = analyze(&s);

        assert_eq!(an.head.len(), 2, "both sides survive");
        assert!(an.diverged());
        assert!(an.orphans.is_empty(), "a divergence is not an orphan");
        assert_eq!(an.divergences.len(), 1);
        assert_eq!(an.divergences[0].parent.as_deref(), Some(parent.as_str()));
        assert_eq!(an.divergences[0].children.len(), 2);
        assert_eq!(an.state(), "diverged");
    }

    #[test]
    fn a_missing_predecessor_is_an_orphan_not_a_divergence() {
        // Decision 0002 Amendment 1: versions 1, 2 and 4 arrive; 3 has not.
        let a = v("pl_1000000000", 1, 1, "first", vec![]);
        let b = v("pl_1000000000", 2, 2, "second", vec![a.hash.clone()]);
        let absent = "f".repeat(64);
        let d = v("pl_1000000000", 4, 4, "fourth", vec![absent.clone()]);
        let d_hash = d.hash.clone();
        let s = store(vec![a, b, d]);
        let an = analyze(&s);

        assert_eq!(an.head.len(), 2, "2 and 4 both lack a successor");
        assert!(
            !an.diverged(),
            "2 and 4 share no predecessor, so this is not divergence"
        );
        assert_eq!(an.orphans.len(), 1);
        assert_eq!(an.orphans[0].version.hash, d_hash);
        assert_eq!(an.orphans[0].missing, vec![absent]);
        assert!(an.is_orphan(&d_hash));
        assert_eq!(an.state(), "orphaned");
    }

    #[test]
    fn two_versions_missing_the_same_parent_are_orphans_not_divergent() {
        // The shared predecessor is absent, so nothing local proves they
        // disagreed — only that replication is behind.
        let absent = "e".repeat(64);
        let a = v("pl_1000000000", 2, 2, "one", vec![absent.clone()]);
        let b = v("pl_1000000000", 2, 2, "two", vec![absent.clone()]);
        let s = store(vec![a, b]);
        let an = analyze(&s);

        assert_eq!(an.orphans.len(), 2);
        assert!(!an.diverged());
        assert_eq!(an.head.len(), 2);
    }

    #[test]
    fn a_reconciliation_closes_a_divergence() {
        let a = v("pl_1000000000", 1, 1, "first", vec![]);
        let left = v("pl_1000000000", 2, 2, "cos side", vec![a.hash.clone()]);
        let right = v("pl_1000000000", 2, 2, "dev side", vec![a.hash.clone()]);
        let merge = v(
            "pl_1000000000",
            3,
            3,
            "both were right",
            vec![left.hash.clone(), right.hash.clone()],
        );
        let merge_hash = merge.hash.clone();
        let s = store(vec![a, left, right, merge]);
        let an = analyze(&s);

        assert_eq!(an.head.len(), 1);
        assert_eq!(an.head[0].hash, merge_hash);
        // The divergence remains visible as history...
        assert!(an.ever_diverged());
        assert_eq!(an.divergences.len(), 1);
        // ...but it is settled, so nothing prompts for a reconciliation.
        assert!(
            !an.diverged(),
            "a reconciled divergence is not an open problem"
        );
        assert_eq!(an.open_divergences().count(), 0);
        assert_eq!(an.state(), "converged-locally");
        assert!(an.orphans.is_empty());
    }

    #[test]
    fn a_reconciliation_can_itself_diverge() {
        let a = v("pl_1000000000", 1, 1, "first", vec![]);
        let left = v("pl_1000000000", 2, 2, "cos", vec![a.hash.clone()]);
        let right = v("pl_1000000000", 2, 2, "dev", vec![a.hash.clone()]);
        let parents = vec![left.hash.clone(), right.hash.clone()];
        let m1 = v("pl_1000000000", 3, 3, "merge one way", parents.clone());
        let m2 = v("pl_1000000000", 3, 3, "merge another way", parents);
        let s = store(vec![a, left, right, m1, m2]);
        let an = analyze(&s);

        assert_eq!(an.head.len(), 2, "the reconciliation diverged in turn");
        // Both left and right now each have two successors.
        assert!(an.divergences.len() >= 2);
    }

    #[test]
    fn a_settled_divergence_stays_visible_but_stops_prompting() {
        // The chain records the disagreement forever — that argument is the
        // product. What must stop is treating it as outstanding work.
        let a = v("pl_1000000000", 1, 1, "first", vec![]);
        let left = v("pl_1000000000", 2, 2, "cos", vec![a.hash.clone()]);
        let right = v("pl_1000000000", 2, 2, "dev", vec![a.hash.clone()]);
        let merge = v(
            "pl_1000000000",
            3,
            3,
            "both",
            vec![left.hash.clone(), right.hash.clone()],
        );
        let s = store(vec![a, left, right, merge]);
        let an = analyze(&s);

        assert!(an.ever_diverged(), "history still records it");
        assert!(!an.diverged(), "but it is settled");
        assert_eq!(an.open_divergences().count(), 0);
    }

    #[test]
    fn a_divergence_reconciled_on_only_one_side_stays_open() {
        // A version descending from just one side does not join them.
        let a = v("pl_1000000000", 1, 1, "first", vec![]);
        let left = v("pl_1000000000", 2, 2, "cos", vec![a.hash.clone()]);
        let right = v("pl_1000000000", 2, 2, "dev", vec![a.hash.clone()]);
        let only_left = v("pl_1000000000", 3, 3, "carry on", vec![left.hash.clone()]);
        let s = store(vec![a, left, right, only_left]);
        let an = analyze(&s);

        assert!(an.diverged(), "the sides still have no common descendant");
        assert_eq!(an.open_divergences().count(), 1);
        assert_eq!(an.head.len(), 2);
    }

    #[test]
    fn several_root_versions_are_reported_as_divergence() {
        let a = v("pl_1000000000", 1, 1, "one origin", vec![]);
        let b = v("pl_1000000000", 1, 1, "another origin", vec![]);
        let s = store(vec![a, b]);
        let an = analyze(&s);
        assert!(an.diverged());
        assert_eq!(an.divergences[0].parent, None);
    }

    #[test]
    fn an_empty_plan_has_an_empty_head() {
        let s = store(vec![]);
        let an = analyze(&s);
        assert!(an.head.is_empty());
        assert_eq!(an.state(), "empty");
    }

    #[test]
    fn lineage_returns_the_rationale_chain_oldest_first() {
        let a = v("pl_1000000000", 1, 1, "first", vec![]);
        let b = v("pl_1000000000", 2, 2, "second", vec![a.hash.clone()]);
        let c = v("pl_1000000000", 3, 3, "third", vec![b.hash.clone()]);
        let tip = c.hash.clone();
        let s = store(vec![a, b, c]);

        let chain = lineage(&s, &tip);
        assert_eq!(chain.len(), 3);
        let whys: Vec<&str> = chain.iter().map(|a| a.version.why.as_str()).collect();
        assert_eq!(whys, vec!["first", "second", "third"]);
    }

    #[test]
    fn lineage_of_a_reconciliation_includes_both_sides_once() {
        let a = v("pl_1000000000", 1, 1, "first", vec![]);
        let left = v("pl_1000000000", 2, 2, "cos", vec![a.hash.clone()]);
        let right = v("pl_1000000000", 2, 3, "dev", vec![a.hash.clone()]);
        let merge = v(
            "pl_1000000000",
            3,
            4,
            "both",
            vec![left.hash.clone(), right.hash.clone()],
        );
        let tip = merge.hash.clone();
        let s = store(vec![a, left, right, merge]);

        let chain = lineage(&s, &tip);
        assert_eq!(chain.len(), 4, "the shared root appears once, not twice");
        assert_eq!(chain[0].version.why, "first");
        assert_eq!(chain[3].version.why, "both");
    }

    #[test]
    fn lineage_stops_at_an_absent_predecessor() {
        let absent = "d".repeat(64);
        let a = v("pl_1000000000", 2, 2, "orphaned", vec![absent]);
        let tip = a.hash.clone();
        let s = store(vec![a]);
        assert_eq!(lineage(&s, &tip).len(), 1);
    }

    #[test]
    fn next_seq_follows_the_longest_predecessor() {
        let short = v("pl_1000000000", 2, 2, "short", vec![]);
        let long = v("pl_1000000000", 9, 3, "long", vec![]);
        assert_eq!(next_seq(&[&short, &long]), 10);
        assert_eq!(next_seq(&[]), 1);
    }
}
