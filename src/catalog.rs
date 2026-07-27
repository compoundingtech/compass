//! The Catalog: on-disk storage and deliberate admission.
//!
//! ```text
//! <root>/plans/<planref>/versions/<seq>-<hash12>.ts    immutable, mode 0444
//! <root>/plans/<planref>/events/<at>-<id>.cmp          append-only
//! ```
//!
//! ## Admission (02-artifacts)
//!
//! A file becomes a Plan Version when it sits in its expected location and the
//! SHA-256 of its bytes matches the hash embedded in its filename. Admission
//! looks at bytes and nothing else — it does not evaluate the module, and in
//! particular does not require that what the module imports be present, because
//! replication delivers files in no useful order. A mismatch is a rejection with
//! an error, never a warning.
//!
//! The identity is the hash of the raw bytes as they sit on disk. Because each
//! version imports its predecessors by filename, and each filename carries a
//! hash of content, the lineage is walkable from the bytes alone: the parents of
//! a version are the versions whose hash-prefix appears in its import
//! specifiers. A prefix that resolves to no local file is a missing predecessor
//! (an orphan), repaired by waiting.

use crate::event::Event;
use crate::model::{filename_for, parse_filename, Version, VERSION_EXT};
use std::fs;
use std::path::{Path, PathBuf};

/// A file admitted as a Plan Version. Its intent is recovered by evaluation on
/// demand (see [`crate::eval`]); admission keeps only what the bytes determine.
#[derive(Debug, Clone)]
pub struct Admitted {
    /// Full content hash — the identity.
    pub hash: String,
    pub path: PathBuf,
    pub plan: String,
    /// A reading aid: one past the longest predecessor. Never a key.
    pub seq: u64,
    /// Predecessor references: a full hash when the predecessor is present, or
    /// the raw 12-hex prefix when it has not arrived (an orphan edge).
    pub parents: Vec<String>,
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

/// One admitted version before its parents are resolved against its siblings.
struct Raw {
    hash: String,
    path: PathBuf,
    plan: String,
    seq: u64,
    /// 12-hex prefixes of the version files this module imports.
    import_prefixes: Vec<String>,
}

/// Resolve the catalog root (configuration, never compiled in).
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
/// Where `start` scaffolds a draft before its identity is known. It is a sibling
/// of `plans/`, never under it, so `list_plans` never mistakes a draft for a Plan
/// — a draft has no PlanRef until it is committed (decision 0017).
pub fn drafts_dir(root: &Path) -> PathBuf {
    root.join("drafts")
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
    let mut raws: Vec<Raw> = Vec::new();
    if vdir.is_dir() {
        for path in sorted_files(&vdir)? {
            match admit_version(&path, plan) {
                Ok(raw) => raws.push(raw),
                Err(reason) => store.rejected.push(Rejected { path, reason }),
            }
        }
    }

    // Resolve import-prefixes to full predecessor hashes among the siblings.
    let by_prefix: std::collections::HashMap<&str, &str> = raws
        .iter()
        .map(|r| (&r.hash[..crate::model::HASH_PREFIX_LEN], r.hash.as_str()))
        .collect();
    for r in &raws {
        let mut parents: Vec<String> = r
            .import_prefixes
            .iter()
            .map(|p| {
                by_prefix
                    .get(p.as_str())
                    .map(|h| h.to_string())
                    .unwrap_or_else(|| p.clone())
            })
            .collect();
        parents.sort();
        parents.dedup();
        store.versions.push(Admitted {
            hash: r.hash.clone(),
            path: r.path.clone(),
            plan: r.plan.clone(),
            seq: r.seq,
            parents,
        });
    }

    reject_misfiled(&mut store, plan);

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

    store
        .versions
        .sort_by(|a, b| (a.seq, &a.hash).cmp(&(b.seq, &b.hash)));
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

/// Decide whether one file may become a Plan Version. Bytes only — no evaluation.
fn admit_version(path: &Path, expected_plan: &str) -> Result<Raw, String> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "file has no readable name".to_string())?;

    if !name.ends_with(&format!(".{VERSION_EXT}")) {
        return Err(format!("not a .{VERSION_EXT} file"));
    }
    let (seq, want_prefix) = parse_filename(name).ok_or_else(|| {
        format!("filename `{name}` is not `<seq>-<hash>.{VERSION_EXT}`, so it names no content")
    })?;

    let bytes = fs::read(path).map_err(|e| format!("cannot read: {e}"))?;
    let actual = crate::sha256::sha256_hex(&bytes);
    if !actual.starts_with(&want_prefix) {
        return Err(format!(
            "content hash mismatch: filename claims {want_prefix}, content hashes to {}",
            &actual[..want_prefix.len().min(actual.len())]
        ));
    }

    let source = std::str::from_utf8(&bytes).map_err(|e| format!("not valid UTF-8: {e}"))?;
    let import_prefixes = predecessor_prefixes(source, path, expected_plan)?;

    Ok(Raw {
        hash: actual,
        path: path.to_path_buf(),
        plan: expected_plan.to_string(),
        seq,
        import_prefixes,
    })
}

/// The hash-prefixes of the *predecessor* version files a module imports, read
/// statically. A predecessor is a version of the SAME plan; a version of another
/// plan is a cross-plan reference (CMP.API-R05), not a parent, and is excluded
/// from the lineage so it never shows as an orphan predecessor edge.
fn predecessor_prefixes(
    source: &str,
    path: &Path,
    expected_plan: &str,
) -> Result<Vec<String>, String> {
    let specs = crate::eval::import_specifiers(source, path)
        .map_err(|e| format!("cannot read imports: {}", e.message()))?;
    let mut out = Vec::new();
    for spec in specs {
        // A predecessor import is a relative path to a version file.
        let file = spec.rsplit('/').next().unwrap_or(&spec);
        if let Some((_seq, prefix)) = parse_filename(file) {
            // Only a same-plan version is a predecessor. A cross-plan reference
            // (target plan differs) is not part of this plan's lineage.
            match crate::eval::import_target_plan(path, &spec) {
                Some(other) if other != expected_plan => continue,
                _ => out.push(prefix),
            }
        }
    }
    Ok(out)
}

/// Reject versions misfiled under the wrong Plan (decision 0017, CMP.DM-R17).
///
/// A Plan's identity is the content hash of its origin. A version whose derived
/// origin resolves to a PlanRef different from the directory it sits in is
/// rejected — never reinterpreted into the Plan it was filed under — on the same
/// terms as a version whose content hash disagrees with its own filename.
///
/// The origin is derived by walking resolved predecessor pointers within the
/// store (no evaluation, no extra IO) to the predecessor-less ancestor. When the
/// walk reaches an ancestor that is absent locally the version is an orphan, not
/// a misfiling: its Plan cannot yet be confirmed, so it is left alone.
fn reject_misfiled(store: &mut PlanStore, plan: &str) {
    use std::collections::{HashMap, HashSet};
    let parents_of: HashMap<String, Vec<String>> = store
        .versions
        .iter()
        .map(|a| (a.hash.clone(), a.parents.clone()))
        .collect();
    let present: HashSet<String> = store.versions.iter().map(|a| a.hash.clone()).collect();

    // The origin PlanRef of a version, or None when an ancestor is absent (an
    // orphan, whose Plan cannot be confirmed) or the lineage forms a cycle.
    let origin_prefix = |start: &str| -> Option<String> {
        let mut cur = start.to_string();
        let mut seen: HashSet<String> = HashSet::new();
        loop {
            if !seen.insert(cur.clone()) {
                return None;
            }
            let parents = parents_of.get(&cur)?;
            if parents.is_empty() {
                return Some(cur[..crate::model::HASH_PREFIX_LEN.min(cur.len())].to_string());
            }
            match parents.iter().find(|p| present.contains(*p)) {
                Some(p) => cur = p.clone(),
                None => return None,
            }
        }
    };

    let mut kept = Vec::new();
    for a in std::mem::take(&mut store.versions) {
        match origin_prefix(&a.hash) {
            Some(pref) if pref != plan => store.rejected.push(Rejected {
                reason: format!(
                    "misfiled: its origin resolves to plan {pref}, but it is filed under {plan} \
                     (decision 0017) — a misfiled version is never reinterpreted into the plan it \
                     was filed under"
                ),
                path: a.path,
            }),
            _ => kept.push(a),
        }
    }
    store.versions = kept;
}

/// Derive a version's PlanRef from its authored bytes (decision 0017).
///
/// A Plan's identity is the content hash of its origin — the single
/// predecessor-less version. An origin (a module that imports no predecessor) is
/// its own PlanRef: the hash of its bytes, the same hash its version filename
/// carries. A revision inherits its Plan from its predecessor: its origin is
/// found by walking the predecessor imports back to the predecessor-less version,
/// and hashing that. The operator names nothing; identity is derived, and the
/// prefix width matches the version filenames' for consistency.
pub fn derive_planref(path: &Path, source: &[u8]) -> Result<String, String> {
    let src = std::str::from_utf8(source)
        .map_err(|e| format!("{}: not valid UTF-8: {e}", path.display()))?;
    let origin_bytes = match sibling_predecessor_paths(path, src)?.into_iter().next() {
        None => source.to_vec(),
        Some(pred) => walk_to_origin(&pred)?,
    };
    Ok(crate::sha256::sha256_hex(&origin_bytes)[..crate::model::HASH_PREFIX_LEN].to_string())
}

/// The resolved paths of the *predecessor* version files a module imports — the
/// same-plan siblings, sitting in the importer's own directory. A cross-plan
/// reference resolves elsewhere and is not a predecessor, so it is excluded, as
/// is the `compass` prelude (which is not a version reference).
fn sibling_predecessor_paths(path: &Path, source: &str) -> Result<Vec<PathBuf>, String> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let specs = crate::eval::import_specifiers(source, path)
        .map_err(|e| format!("cannot read imports: {}", e.message()))?;
    let mut out = Vec::new();
    for spec in specs {
        let file = spec.rsplit('/').next().unwrap_or(&spec);
        if crate::model::parse_filename(file).is_none() {
            continue;
        }
        if let Some(target) = crate::eval::normalize_lexical(dir, &spec) {
            if target.parent() == Some(dir) {
                out.push(target);
            }
        }
    }
    Ok(out)
}

/// Walk a predecessor chain to its origin and return the origin's raw bytes.
/// Any predecessor of a version shares that version's origin, so following one
/// predecessor at each step suffices.
fn walk_to_origin(pred: &Path) -> Result<Vec<u8>, String> {
    let mut current = pred.to_path_buf();
    let mut seen = std::collections::HashSet::new();
    loop {
        if !seen.insert(current.clone()) {
            return Err(
                "predecessor lineage forms a cycle; a plan's identity cannot be derived".into(),
            );
        }
        let bytes = fs::read(&current).map_err(|_| {
            format!(
                "predecessor {} has not arrived: a plan's identity is its origin, which cannot be \
                 derived until the origin is present (decision 0017)",
                current.display()
            )
        })?;
        let src = std::str::from_utf8(&bytes)
            .map_err(|e| format!("{}: not valid UTF-8: {e}", current.display()))?;
        match sibling_predecessor_paths(&current, src)?.into_iter().next() {
            None => return Ok(bytes),
            Some(next) => current = next,
        }
    }
}

/// Write a Plan Version from its authored source bytes.
///
/// Returns its path and whether it was newly created. The name is the content
/// hash, so an identical file is the same version and is left untouched — the
/// whole of Compass's idempotency, with no caller-supplied key (CMP-R10).
pub fn write_version(
    root: &Path,
    plan: &str,
    seq: u64,
    source: &[u8],
) -> Result<(PathBuf, String, bool), String> {
    let hash = crate::sha256::sha256_hex(source);
    let dir = versions_dir(root, plan);
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let path = dir.join(filename_for(seq, &hash));
    if path.exists() {
        return Ok((path, hash, false));
    }
    write_readonly(&path, source)?;
    Ok((path, hash, true))
}

/// Append a Progress Event.
pub fn write_event(root: &Path, e: &Event) -> Result<PathBuf, String> {
    let dir = events_dir(root, &e.plan);
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let path = dir.join(e.filename());
    write_readonly(&path, e.render().as_bytes())?;
    Ok(path)
}

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

/// Build a domain [`Version`] for an admitted file by evaluating it.
pub fn evaluate(a: &Admitted) -> Result<Version, crate::eval::EvalError> {
    let map = crate::eval::eval_plan_file(&a.path)?;
    let canon = std::fs::canonicalize(&a.path).unwrap_or_else(|_| a.path.clone());
    let sem = map
        .get(&canon)
        .or_else(|| map.get(&a.path))
        .ok_or_else(|| crate::eval::EvalError::Failed("evaluation produced no plan".into()))?;
    Ok(Version::from_sem(&a.plan, a.seq, a.parents.clone(), sem))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch {
        root: PathBuf,
    }
    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let base = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
            let uniq = format!("{}-{}", std::process::id(), tag);
            let root = PathBuf::from(base).join(format!("compass-test-{uniq}"));
            let _ = make_writable_recursive(&root);
            let _ = fs::remove_dir_all(&root);
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

    const ROOT_MODULE: &str = r#"import { plan, step, evidence } from "compass"
export const a = step({ work: "do a", accept: evidence.test({ status: "pass" }) })
export default plan({ author: "cos", goal: "Ship", why: "start", steps: [a] })
"#;

    /// The PlanRef a module files under: the hash-prefix of its own bytes, for an
    /// origin (decision 0017).
    fn planref_of(source: &str) -> String {
        crate::sha256::sha256_hex(source.as_bytes())[..crate::model::HASH_PREFIX_LEN].to_string()
    }

    #[test]
    fn admits_a_version_by_source_byte_hash() {
        let s = Scratch::new("admit");
        let plan = planref_of(ROOT_MODULE);
        let (path, hash, created) =
            write_version(&s.root, &plan, 1, ROOT_MODULE.as_bytes()).unwrap();
        assert!(created);
        assert!(path.exists());
        assert_eq!(hash, crate::sha256::sha256_hex(ROOT_MODULE.as_bytes()));

        let store = load_plan(&s.root, &plan).unwrap();
        assert_eq!(store.versions.len(), 1, "rejected: {:?}", store.rejected);
        assert_eq!(store.versions[0].hash, hash);
        assert!(store.versions[0].parents.is_empty());
    }

    #[test]
    fn identical_content_is_a_no_op() {
        let s = Scratch::new("noop");
        let plan = planref_of(ROOT_MODULE);
        let (_, _, first) = write_version(&s.root, &plan, 1, ROOT_MODULE.as_bytes()).unwrap();
        let (_, _, second) = write_version(&s.root, &plan, 1, ROOT_MODULE.as_bytes()).unwrap();
        assert!(first);
        assert!(!second);
    }

    #[test]
    fn tampered_bytes_are_rejected() {
        let s = Scratch::new("tamper");
        let plan = planref_of(ROOT_MODULE);
        let (path, _, _) = write_version(&s.root, &plan, 1, ROOT_MODULE.as_bytes()).unwrap();
        make_writable_recursive(&s.root).unwrap();
        fs::write(&path, b"import x from 'y'\n").unwrap();
        let store = load_plan(&s.root, &plan).unwrap();
        assert!(store.versions.is_empty());
        assert!(store.rejected[0].reason.contains("content hash mismatch"));
    }

    /// An origin filed under a directory that is not its own hash is rejected as
    /// misfiled, distinctly from tampering: the filename still matches the
    /// content (decision 0017).
    #[test]
    fn a_misfiled_version_is_rejected() {
        let s = Scratch::new("misfiled");
        let wrong = "999999999999";
        assert_ne!(wrong, planref_of(ROOT_MODULE));
        let (path, _, _) = write_version(&s.root, wrong, 1, ROOT_MODULE.as_bytes()).unwrap();
        assert!(path.exists());
        let store = load_plan(&s.root, wrong).unwrap();
        assert!(
            store.versions.is_empty(),
            "a misfiled origin is not admitted"
        );
        assert_eq!(store.rejected.len(), 1);
        assert!(store.rejected[0].reason.contains("misfiled"));
    }

    #[test]
    fn a_version_evaluates_to_its_intent() {
        let s = Scratch::new("eval");
        let plan = planref_of(ROOT_MODULE);
        write_version(&s.root, &plan, 1, ROOT_MODULE.as_bytes()).unwrap();
        let store = load_plan(&s.root, &plan).unwrap();
        let v = evaluate(&store.versions[0]).unwrap();
        assert_eq!(v.goal, "Ship");
        assert_eq!(v.steps.len(), 1);
        assert_eq!(v.steps[0].id, "a");
    }
}
