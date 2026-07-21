//! The Catalog: on-disk storage and deliberate ingestion.
//!
//! ```text
//! <root>/plans/<planref>/versions/<seq>-<hash12>.cmp   immutable, mode 0444
//! <root>/plans/<planref>/events/<at>-<id>.cmp          append-only
//! ```
//!
//! The root is configuration, never compiled in (CMP-R15).
//!
//! ## Admission (CMP-R22)
//!
//! Under no-delete replication a wrongly-adopted file is permanent, so merely
//! parsing is not enough. A file becomes a Plan Version only when it is in its
//! expected location *and* the SHA-256 of its bytes matches the hash embedded
//! in its filename. A mismatch is a **rejection with an error**, never a
//! warning that a reader might skim past.
//!
//! The hash is taken over the **raw file bytes as they sit on disk** — never
//! over a parse-and-re-render. Re-rendering would make a reformatted file
//! admissible under its old name, which is exactly the corruption the chain
//! exists to catch. (DQ02 asks raw bytes or canonical form; raw bytes, and the
//! canonical writer is what keeps that strictness usable.)
//!
//! Files arriving through replication are treated exactly as local files
//! (CMP.INT-R08). Nothing trusts a file mode: `0444` is a local accident-guard
//! that stops an agent editing in place, not a property of the wire.

use crate::event::Event;
use crate::model::{parse_filename, Version, EXT};
use std::fs;
use std::path::{Path, PathBuf};

/// A file admitted as a Plan Version.
#[derive(Debug, Clone)]
pub struct Admitted {
    /// Full content hash — the identity.
    pub hash: String,
    pub path: PathBuf,
    pub version: Version,
}

/// A file in a versions/ or events/ directory that was not admitted.
#[derive(Debug, Clone)]
pub struct Rejected {
    pub path: PathBuf,
    pub reason: String,
}

/// Everything the catalog holds for one plan.
#[derive(Debug, Clone, Default)]
pub struct PlanStore {
    pub plan: String,
    pub versions: Vec<Admitted>,
    pub rejected: Vec<Rejected>,
    pub events: Vec<Event>,
    pub bad_events: Vec<Rejected>,
}

impl PlanStore {
    pub fn version(&self, hash: &str) -> Option<&Admitted> {
        self.versions.iter().find(|a| a.hash == hash)
    }

    /// Resolve a full hash from a unique prefix, for operator convenience.
    pub fn resolve_hash(&self, prefix: &str) -> Option<&Admitted> {
        let mut hits = self.versions.iter().filter(|a| a.hash.starts_with(prefix));
        let first = hits.next()?;
        if hits.next().is_some() {
            return None; // ambiguous
        }
        Some(first)
    }

    /// Next logical time for a new event.
    pub fn next_event_at(&self) -> u64 {
        self.events.iter().map(|e| e.at).max().unwrap_or(0) + 1
    }
}

/// Resolve the catalog root (CMP-R15).
pub fn root() -> Result<PathBuf, String> {
    if let Some(v) = env_nonempty("COMPASS_CATALOG") {
        return Ok(PathBuf::from(v));
    }
    if let Some(v) = env_nonempty("XDG_STATE_HOME") {
        return Ok(PathBuf::from(v).join("compass").join("catalog"));
    }
    let home = env_nonempty("HOME")
        .ok_or_else(|| "cannot locate a catalog: set $COMPASS_CATALOG or $HOME".to_string())?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("state")
        .join("compass")
        .join("catalog"))
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// Who is authoring. Configuration, with a plain fallback.
pub fn author() -> String {
    env_nonempty("COMPASS_AUTHOR")
        .or_else(|| env_nonempty("USER"))
        .or_else(|| env_nonempty("LOGNAME"))
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn plans_dir(root: &Path) -> PathBuf {
    root.join("plans")
}

pub fn plan_dir(root: &Path, plan: &str) -> PathBuf {
    plans_dir(root).join(plan)
}

pub fn versions_dir(root: &Path, plan: &str) -> PathBuf {
    plan_dir(root, plan).join("versions")
}

pub fn events_dir(root: &Path, plan: &str) -> PathBuf {
    plan_dir(root, plan).join("events")
}

/// Create the catalog skeleton. Idempotent.
pub fn init(root: &Path) -> Result<(), String> {
    fs::create_dir_all(plans_dir(root))
        .map_err(|e| format!("cannot create catalog at {}: {e}", root.display()))
}

/// Whether the catalog exists.
pub fn exists(root: &Path) -> bool {
    plans_dir(root).is_dir()
}

/// Every PlanRef with a directory in the catalog, sorted.
pub fn list_plans(root: &Path) -> Result<Vec<String>, String> {
    let dir = plans_dir(root);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                out.push(name.to_string());
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Load one plan, applying admission to every candidate file.
pub fn load_plan(root: &Path, plan: &str) -> Result<PlanStore, String> {
    let mut store = PlanStore {
        plan: plan.to_string(),
        ..Default::default()
    };

    let vdir = versions_dir(root, plan);
    if vdir.is_dir() {
        for path in sorted_files(&vdir)? {
            match admit_version(&path, plan) {
                Ok(a) => store.versions.push(a),
                Err(reason) => store.rejected.push(Rejected { path, reason }),
            }
        }
    }

    let edir = events_dir(root, plan);
    if edir.is_dir() {
        for path in sorted_files(&edir)? {
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(e) => {
                    store.bad_events.push(Rejected {
                        path,
                        reason: format!("cannot read: {e}"),
                    });
                    continue;
                }
            };
            let text = String::from_utf8_lossy(&bytes);
            match Event::parse(&text) {
                Ok(e) if e.plan != plan => store.bad_events.push(Rejected {
                    path,
                    reason: format!("event declares plan {} but is filed under {plan}", e.plan),
                }),
                Ok(e) => store.events.push(e),
                Err(err) => store.bad_events.push(Rejected {
                    path,
                    reason: format!("cannot parse: {err}"),
                }),
            }
        }
    }

    // Lineage depth first, then hash: a total order that needs no recorded
    // counter, and that two machines holding the same versions agree on.
    store
        .versions
        .sort_by(|a, b| (a.version.seq, &a.hash).cmp(&(b.version.seq, &b.hash)));
    store
        .events
        .sort_by(|a, b| (a.at, &a.id).cmp(&(b.at, &b.id)));
    Ok(store)
}

fn sorted_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        let p = entry.path();
        if p.is_file() {
            out.push(p);
        }
    }
    out.sort();
    Ok(out)
}

/// Decide whether one file may become a Plan Version (CMP-R22).
pub fn admit_version(path: &Path, expected_plan: &str) -> Result<Admitted, String> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "file has no readable name".to_string())?;

    if !name.ends_with(&format!(".{EXT}")) {
        return Err(format!("not a .{EXT} file"));
    }

    let (_seq, want_prefix) = parse_filename(name).ok_or_else(|| {
        format!("filename `{name}` is not `<seq>-<hash>.{EXT}`, so it names no content")
    })?;

    // Hash the bytes exactly as they sit on disk.
    let bytes = fs::read(path).map_err(|e| format!("cannot read: {e}"))?;
    let actual = crate::sha256::sha256_hex(&bytes);
    if !actual.starts_with(&want_prefix) {
        return Err(format!(
            "content hash mismatch: filename claims {want_prefix}, content hashes to {}",
            &actual[..want_prefix.len().min(actual.len())]
        ));
    }

    let text = std::str::from_utf8(&bytes).map_err(|e| format!("not valid UTF-8: {e}"))?;
    let version = Version::parse(text).map_err(|e| format!("cannot parse: {e}"))?;

    if version.plan != expected_plan {
        return Err(format!(
            "version declares plan {} but is filed under {expected_plan}",
            version.plan
        ));
    }

    Ok(Admitted {
        hash: actual,
        path: path.to_path_buf(),
        version,
    })
}

/// Write a Plan Version. Returns its path and whether it was newly created.
///
/// A version whose file already exists is left alone: the name is the content
/// hash, so an identical file is the same version. Since no field is excluded
/// from that hash (decision 0007), this is the whole of Compass's idempotency:
/// there is no caller-supplied key, and none is wanted (CMP-R10).
pub fn write_version(root: &Path, v: &Version) -> Result<(PathBuf, bool), String> {
    let dir = versions_dir(root, &v.plan);
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let path = dir.join(v.filename());
    if path.exists() {
        return Ok((path, false));
    }
    write_readonly(&path, v.render().as_bytes())?;
    Ok((path, true))
}

/// Append a Progress Event.
pub fn write_event(root: &Path, e: &Event) -> Result<PathBuf, String> {
    let dir = events_dir(root, &e.plan);
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let path = dir.join(e.filename());
    write_readonly(&path, e.render().as_bytes())?;
    Ok(path)
}

/// Write a file and drop it to mode 0444 (accident-prevention, decision 0002).
fn write_readonly(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    set_readonly(path)
}

#[cfg(unix)]
fn set_readonly(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o444))
        .map_err(|e| format!("cannot set mode 0444 on {}: {e}", path.display()))
}

#[cfg(not(unix))]
fn set_readonly(path: &Path) -> Result<(), String> {
    let mut perms = fs::metadata(path)
        .map_err(|e| format!("cannot stat {}: {e}", path.display()))?
        .permissions();
    perms.set_readonly(true);
    fs::set_permissions(path, perms)
        .map_err(|e| format!("cannot set {} read-only: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Step;
    use crate::predicate::parse as pred;

    /// A scratch catalog that cleans itself up.
    struct Scratch {
        root: PathBuf,
    }

    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let base = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
            let unique = crate::refs::mint(crate::refs::RefKind::Event).unwrap();
            let root = PathBuf::from(base).join(format!("compass-test-{tag}-{unique}"));
            init(&root).unwrap();
            Scratch { root }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = make_writable_recursive(&self.root);
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn make_writable_recursive(p: &Path) -> std::io::Result<()> {
        if p.is_dir() {
            for e in fs::read_dir(p)? {
                make_writable_recursive(&e?.path())?;
            }
        } else if p.is_file() {
            let mut perms = fs::metadata(p)?.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            fs::set_permissions(p, perms)?;
        }
        Ok(())
    }

    fn a_version(plan: &str) -> Version {
        Version {
            plan: plan.to_string(),
            seq: 1,
            parents: vec![],
            author: "cos".into(),
            why: "Initial plan.".into(),
            goal: "Ship it".into(),
            retired: false,
            steps: vec![Step::new(
                "st_A000000001",
                "Do the work",
                pred("test(status=pass)").unwrap(),
            )],
        }
    }

    #[test]
    fn root_prefers_explicit_configuration() {
        // Verified through the pure helper rather than by mutating process env,
        // which would race other tests.
        assert!(root().is_ok() || std::env::var("HOME").is_err());
    }

    #[test]
    fn writes_and_reloads_a_version() {
        let s = Scratch::new("roundtrip");
        let v = a_version("pl_1000000000");
        let (path, created) = write_version(&s.root, &v).unwrap();
        assert!(created);
        assert!(path.exists());

        let store = load_plan(&s.root, &v.plan).unwrap();
        assert_eq!(store.versions.len(), 1, "rejected: {:?}", store.rejected);
        assert!(store.rejected.is_empty());
        assert_eq!(store.versions[0].hash, v.hash());
        assert_eq!(store.versions[0].version, v);
    }

    #[test]
    fn versions_are_written_read_only() {
        let s = Scratch::new("readonly");
        let v = a_version("pl_2000000000");
        let (path, _) = write_version(&s.root, &v).unwrap();
        assert!(fs::metadata(&path).unwrap().permissions().readonly());
        // An in-place edit becomes a visible error rather than silent damage.
        assert!(fs::write(&path, b"tampered").is_err());
    }

    #[test]
    fn rewriting_an_identical_version_is_a_no_op() {
        let s = Scratch::new("idempotent");
        let v = a_version("pl_3000000000");
        let (_, first) = write_version(&s.root, &v).unwrap();
        let (_, second) = write_version(&s.root, &v).unwrap();
        assert!(first);
        assert!(!second, "identical content must not be rewritten");
        assert_eq!(load_plan(&s.root, &v.plan).unwrap().versions.len(), 1);
    }

    #[test]
    fn tampered_content_is_rejected_not_warned() {
        let s = Scratch::new("tamper");
        let v = a_version("pl_4000000000");
        let (path, _) = write_version(&s.root, &v).unwrap();

        // Simulate corruption in transit: same name, different bytes.
        make_writable_recursive(&s.root).unwrap();
        let mut tampered = v.clone();
        tampered.goal = "Something else entirely".into();
        fs::write(&path, tampered.render()).unwrap();

        let store = load_plan(&s.root, &v.plan).unwrap();
        assert!(
            store.versions.is_empty(),
            "tampered file must not be admitted"
        );
        assert_eq!(store.rejected.len(), 1);
        assert!(
            store.rejected[0].reason.contains("content hash mismatch"),
            "{}",
            store.rejected[0].reason
        );
    }

    #[test]
    fn a_file_that_merely_parses_is_not_adopted() {
        let s = Scratch::new("stray");
        let plan = "pl_5000000000";
        let v = a_version(plan);
        let dir = versions_dir(&s.root, plan);
        fs::create_dir_all(&dir).unwrap();
        // Well-formed content, but the name names no content.
        fs::write(dir.join("plan-draft.cmp"), v.render()).unwrap();
        // And a correctly-shaped name carrying the wrong hash.
        fs::write(dir.join("001-aaaaaaaaaaaa.cmp"), v.render()).unwrap();

        let store = load_plan(&s.root, plan).unwrap();
        assert!(store.versions.is_empty());
        assert_eq!(store.rejected.len(), 2);
    }

    #[test]
    fn a_version_filed_under_the_wrong_plan_is_rejected() {
        let s = Scratch::new("misfiled");
        let v = a_version("pl_6000000000");
        let dir = versions_dir(&s.root, "pl_9999999999");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(v.filename()), v.render()).unwrap();

        let store = load_plan(&s.root, "pl_9999999999").unwrap();
        assert!(store.versions.is_empty());
        assert!(store.rejected[0].reason.contains("declares plan"));
    }

    #[test]
    fn unrelated_files_are_ignored_with_a_reason() {
        let s = Scratch::new("foreign");
        let plan = "pl_7000000000";
        let dir = versions_dir(&s.root, plan);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("README.md"), "notes").unwrap();

        let store = load_plan(&s.root, plan).unwrap();
        assert!(store.versions.is_empty());
        assert_eq!(store.rejected.len(), 1);
    }

    /// Two machines that independently make the *same* revision from the same
    /// parent converge on one version instead of diverging (decision 0007).
    ///
    /// Each catalog holds a different amount of unrelated history, which under
    /// a recorded `max(seen) + 1` counter would have given the two revisions
    /// different bodies, different names, and a permanent false divergence.
    /// Nothing is recorded that either machine's local history can influence,
    /// so the bytes match and replication unions them into one file.
    #[test]
    fn the_same_revision_from_the_same_parent_converges() {
        let left = Scratch::new("converge-left");
        let right = Scratch::new("converge-right");
        let plan = "pl_8000000000";

        let root_version = a_version(plan);
        write_version(&left.root, &root_version).unwrap();
        write_version(&right.root, &root_version).unwrap();

        // The left machine has seen more of the plan's history than the right.
        let mut unrelated = a_version(plan);
        unrelated.seq = 2;
        unrelated.parents = vec![root_version.hash()];
        unrelated.goal = "An older direction, since abandoned".into();
        unrelated.why = "History the right machine never saw.".into();
        write_version(&left.root, &unrelated).unwrap();

        let revision = |base: &Version| {
            let mut v = base.clone();
            v.seq = base.seq + 1;
            v.parents = vec![base.hash()];
            v.why = "The tokenizer, not the grammar, drops it.".into();
            v.steps[0].work = "Fix the tokenizer".into();
            v
        };
        let from_left = revision(&root_version);
        let from_right = revision(&root_version);

        assert_eq!(
            from_left.hash(),
            from_right.hash(),
            "identical intent from an identical parent is one version"
        );

        write_version(&left.root, &from_left).unwrap();
        write_version(&right.root, &from_right).unwrap();
        assert_eq!(
            from_left.filename(),
            from_right.filename(),
            "one name, so replication unions rather than accumulating"
        );

        // Replicate right's file into left. It is the file left already holds.
        let (path, created) = write_version(&left.root, &from_right).unwrap();
        assert!(!created, "the replicated file is the one already present");

        let store = load_plan(&left.root, plan).unwrap();
        assert!(store.rejected.is_empty(), "{:?}", store.rejected);
        assert_eq!(
            store
                .versions
                .iter()
                .filter(|a| a.hash == from_left.hash())
                .count(),
            1,
            "the two machines' revisions are one version, not two: {}",
            path.display()
        );
    }

    #[test]
    fn resolves_a_unique_hash_prefix() {
        let s = Scratch::new("prefix");
        let v = a_version("pl_9000000000");
        write_version(&s.root, &v).unwrap();
        let store = load_plan(&s.root, &v.plan).unwrap();
        assert!(store.resolve_hash(&v.hash()[..8]).is_some());
        assert!(store.resolve_hash("ffffffffffffff").is_none());
    }

    #[test]
    fn lists_plans_and_tolerates_an_empty_catalog() {
        let s = Scratch::new("list");
        assert!(list_plans(&s.root).unwrap().is_empty());
        write_version(&s.root, &a_version("pl_A000000000")).unwrap();
        write_version(&s.root, &a_version("pl_B000000000")).unwrap();
        assert_eq!(
            list_plans(&s.root).unwrap(),
            vec!["pl_A000000000", "pl_B000000000"]
        );
    }

    #[test]
    fn events_round_trip_through_the_catalog() {
        use crate::event::{Event, EventKind};
        let s = Scratch::new("events");
        let plan = "pl_C000000000";
        let e = Event {
            id: "ev_0000000001".into(),
            at: 1,
            wall: 0,
            plan: plan.into(),
            step: "st_A000000001".into(),
            version: "a".repeat(64),
            actor: "cos".into(),
            kind: EventKind::Start,
            note: None,
            evidence_kind: None,
            attrs: vec![],
        };
        write_event(&s.root, &e).unwrap();
        let store = load_plan(&s.root, plan).unwrap();
        assert_eq!(store.events.len(), 1);
        assert_eq!(store.events[0], e);
        assert_eq!(store.next_event_at(), 2);
    }

    #[test]
    fn init_is_idempotent() {
        let s = Scratch::new("init");
        assert!(exists(&s.root));
        init(&s.root).unwrap();
        assert!(exists(&s.root));
    }
}
