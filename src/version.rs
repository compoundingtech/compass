//! Build identity, following the shared cross-language versioning contract.
//!
//! The contract's canonical fields are `baseVersion`, `rev`, `dirty`,
//! `sourceKind`, `commitTs` and `buildTs`, from which two outputs are derived:
//!
//! - `machineVersion` — stable and parseable, for telemetry, APIs and logs.
//! - `displayVersion` — human-facing, for `compass version` and errors.
//!
//! The stamp is the same single-line JSON used by the TypeScript/Nix helpers
//! (`@overeng/utils/node/cli-version`, `effect-utils.lib.cliBuildStamp`),
//! carried in `CLI_BUILD_STAMP` and embedded at build time by `build.rs`:
//!
//! ```json
//! {"type":"local","rev":"abc123","ts":1739999700,"dirty":true}
//! {"type":"nix","version":"0.1.0","rev":"def456","commitTs":1739740800,"dirty":false}
//! ```
//!
//! ## Local constraint
//!
//! There is no shared Rust implementation of this contract yet — the existing
//! helpers are TypeScript and Nix. This module therefore reimplements the
//! *semantics* rather than forking them: identical field names, identical
//! `sourceKind` values, and identical `machineVersion` / `displayVersion`
//! formatting, so a Rust CLI and a TypeScript CLI report the same shape. If a
//! second Rust binary needs this, it should move to a shared crate rather than
//! being copied.

use crate::json::Json;

/// Base version from the package manifest.
const BASE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Stamp embedded at build time. Empty when neither Nix nor git supplied one.
const EMBEDDED_STAMP: &str = env!("COMPASS_BUILD_STAMP");

/// Runtime override, matching the shared contract's env var name.
const RUNTIME_ENV: &str = "CLI_BUILD_STAMP";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildIdentity {
    pub base_version: String,
    pub display_version: String,
    pub machine_version: String,
    /// `package`, `local`, or `nix`.
    pub source_kind: &'static str,
    pub rev: Option<String>,
    pub dirty: bool,
    pub commit_ts: Option<i64>,
    pub build_ts: Option<i64>,
}

impl BuildIdentity {
    pub fn to_json(&self) -> Json {
        Json::obj(vec![
            ("command", Json::str("version")),
            ("baseVersion", Json::str(&self.base_version)),
            ("displayVersion", Json::str(&self.display_version)),
            ("machineVersion", Json::str(&self.machine_version)),
            ("sourceKind", Json::str(self.source_kind)),
            (
                "rev",
                match &self.rev {
                    Some(r) => Json::str(r),
                    None => Json::Null,
                },
            ),
            ("dirty", Json::Bool(self.dirty)),
            (
                "commitTs",
                match self.commit_ts {
                    Some(t) => Json::num(t),
                    None => Json::Null,
                },
            ),
            (
                "buildTs",
                match self.build_ts {
                    Some(t) => Json::num(t),
                    None => Json::Null,
                },
            ),
        ])
    }
}

/// Resolve this binary's build identity.
pub fn identity() -> BuildIdentity {
    resolve(
        BASE_VERSION,
        EMBEDDED_STAMP,
        std::env::var(RUNTIME_ENV).ok().as_deref(),
        crate::event::now_wall() as i64,
    )
}

/// Pure resolution, so the formatting rules are testable without a build.
///
/// A build-embedded `nix` stamp wins over the runtime environment: it
/// describes the artifact that is actually running. Otherwise the runtime
/// stamp applies, then any embedded `local` stamp, then the package fallback.
pub fn resolve(
    base_version: &str,
    embedded: &str,
    runtime: Option<&str>,
    now: i64,
) -> BuildIdentity {
    let embedded_stamp = parse_stamp(embedded);
    let runtime_stamp = runtime.and_then(parse_stamp);

    let chosen = match (&embedded_stamp, &runtime_stamp) {
        (Some(e @ Stamp::Nix { .. }), _) => Some(e.clone()),
        (_, Some(r)) => Some(r.clone()),
        (Some(e), None) => Some(e.clone()),
        (None, None) => None,
    };

    match chosen {
        None => BuildIdentity {
            base_version: base_version.to_string(),
            display_version: base_version.to_string(),
            machine_version: base_version.to_string(),
            source_kind: "package",
            rev: None,
            dirty: false,
            commit_ts: None,
            build_ts: None,
        },
        Some(Stamp::Local { rev, ts, dirty }) => {
            let machine = format!(
                "{base_version}+local.{rev}{}",
                if dirty { ".dirty" } else { "" }
            );
            let display = format!(
                "{base_version} — running from local source ({rev}, {}{})",
                relative_time(ts, now),
                if dirty {
                    ", with uncommitted changes"
                } else {
                    ""
                }
            );
            BuildIdentity {
                base_version: base_version.to_string(),
                display_version: display,
                machine_version: machine,
                source_kind: "local",
                rev: Some(rev),
                dirty,
                commit_ts: Some(ts),
                build_ts: None,
            }
        }
        Some(Stamp::Nix {
            version,
            rev,
            commit_ts,
            build_ts,
            dirty,
        }) => {
            // Do not double-suffix a rev that already says it is dirty.
            let suffix = if dirty && !rev.ends_with("-dirty") {
                "-dirty"
            } else {
                ""
            };
            let machine = format!("{version}+{rev}{suffix}");
            let dirty_note = if dirty {
                ", with uncommitted changes"
            } else {
                ""
            };
            let display = match build_ts {
                Some(b) => format!("{machine} — built {}{dirty_note}", relative_time(b, now)),
                None => format!(
                    "{machine} — committed {}{dirty_note}",
                    relative_time(commit_ts, now)
                ),
            };
            BuildIdentity {
                base_version: version,
                display_version: display,
                machine_version: machine,
                source_kind: "nix",
                rev: Some(rev),
                dirty,
                commit_ts: Some(commit_ts),
                build_ts,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Stamp {
    Local {
        rev: String,
        ts: i64,
        dirty: bool,
    },
    Nix {
        version: String,
        rev: String,
        commit_ts: i64,
        build_ts: Option<i64>,
        dirty: bool,
    },
}

/// Parse a stamp. A malformed stamp yields `None` and degrades to the next
/// source rather than failing: a version string is diagnostics, never a reason
/// to refuse to run.
fn parse_stamp(src: &str) -> Option<Stamp> {
    let src = src.trim();
    if src.is_empty() {
        return None;
    }
    match field_str(src, "type")?.as_str() {
        "local" => Some(Stamp::Local {
            rev: field_str(src, "rev")?,
            ts: field_num(src, "ts").unwrap_or(0),
            dirty: field_bool(src, "dirty").unwrap_or(false),
        }),
        "nix" => Some(Stamp::Nix {
            version: field_str(src, "version")?,
            rev: field_str(src, "rev")?,
            commit_ts: field_num(src, "commitTs").unwrap_or(0),
            build_ts: field_num(src, "buildTs"),
            dirty: field_bool(src, "dirty").unwrap_or(false),
        }),
        _ => None,
    }
}

/// Read `"key":"value"` from a flat JSON object.
///
/// Deliberately not a general JSON parser: the stamp is a flat object this
/// project also writes, and Compass takes no external crates. Nested objects
/// and arrays are not supported and are not part of the contract.
fn field_str(src: &str, key: &str) -> Option<String> {
    let start = value_start(src, key)?;
    let rest = &src[start..];
    let rest = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => out.push(chars.next()?),
            c => out.push(c),
        }
    }
    None
}

fn field_num(src: &str, key: &str) -> Option<i64> {
    let start = value_start(src, key)?;
    let rest = &src[start..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '-'))
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn field_bool(src: &str, key: &str) -> Option<bool> {
    let start = value_start(src, key)?;
    let rest = &src[start..];
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

/// Byte offset of the value following `"key":`, skipping whitespace.
fn value_start(src: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{key}\"");
    let at = src.find(&needle)?;
    let after = &src[at + needle.len()..];
    let colon = after.find(':')?;
    let mut idx = at + needle.len() + colon + 1;
    while src[idx..].starts_with(char::is_whitespace) {
        idx += 1;
    }
    Some(idx)
}

/// Coarse relative time, matching the shared helper's human phrasing.
///
/// Relative text appears only in `displayVersion`; the contract forbids it in
/// any stored or telemetry version value.
fn relative_time(then: i64, now: i64) -> String {
    let d = now.saturating_sub(then);
    if d < 0 {
        return "in the future".to_string();
    }
    const MIN: i64 = 60;
    const HOUR: i64 = 60 * MIN;
    const DAY: i64 = 24 * HOUR;

    let plural = |n: i64, unit: &str| {
        if n == 1 {
            format!("1 {unit} ago")
        } else {
            format!("{n} {unit}s ago")
        }
    };

    if d < MIN {
        "just now".to_string()
    } else if d < HOUR {
        plural(d / MIN, "min")
    } else if d < DAY {
        plural(d / HOUR, "hour")
    } else {
        plural(d / DAY, "day")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_700_000_000;

    #[test]
    fn no_stamp_falls_back_to_the_package_version() {
        let id = resolve("0.1.0", "", None, NOW);
        assert_eq!(id.source_kind, "package");
        assert_eq!(id.machine_version, "0.1.0");
        assert_eq!(id.display_version, "0.1.0");
        assert_eq!(id.rev, None);
        assert!(!id.dirty);
    }

    #[test]
    fn parses_a_local_stamp_and_formats_both_versions() {
        let stamp = r#"{"type":"local","rev":"abc123","ts":1699999700,"dirty":true}"#;
        let id = resolve("0.1.0", stamp, None, NOW);
        assert_eq!(id.source_kind, "local");
        assert_eq!(id.machine_version, "0.1.0+local.abc123.dirty");
        assert!(
            id.display_version
                .starts_with("0.1.0 — running from local source (abc123, "),
            "{}",
            id.display_version
        );
        assert!(id.display_version.ends_with(", with uncommitted changes)"));
        assert!(id.dirty);
        assert_eq!(id.rev.as_deref(), Some("abc123"));
    }

    #[test]
    fn a_clean_local_stamp_has_no_dirty_markers() {
        let stamp = r#"{"type":"local","rev":"abc123","ts":1699999700,"dirty":false}"#;
        let id = resolve("0.1.0", stamp, None, NOW);
        assert_eq!(id.machine_version, "0.1.0+local.abc123");
        assert!(!id.display_version.contains("uncommitted"));
    }

    #[test]
    fn parses_a_nix_stamp_committed() {
        let stamp = r#"{"type":"nix","version":"0.2.0","rev":"def456","commitTs":1699740800,"dirty":false}"#;
        let id = resolve("0.1.0", stamp, None, NOW);
        assert_eq!(id.source_kind, "nix");
        assert_eq!(id.machine_version, "0.2.0+def456");
        assert!(id.display_version.starts_with("0.2.0+def456 — committed "));
        // The stamp's own version wins over the manifest for a nix build.
        assert_eq!(id.base_version, "0.2.0");
    }

    #[test]
    fn a_nix_stamp_with_a_build_timestamp_says_built_not_committed() {
        let stamp = r#"{"type":"nix","version":"0.2.0","rev":"def456","commitTs":1699740800,"buildTs":1699999700,"dirty":false}"#;
        let id = resolve("0.1.0", stamp, None, NOW);
        assert!(
            id.display_version.contains("— built "),
            "{}",
            id.display_version
        );
        assert!(!id.display_version.contains("committed"));
        assert_eq!(id.build_ts, Some(1_699_999_700));
    }

    #[test]
    fn a_dirty_nix_rev_is_not_double_suffixed() {
        let already =
            r#"{"type":"nix","version":"0.2.0","rev":"def456-dirty","commitTs":1,"dirty":true}"#;
        assert_eq!(
            resolve("0.1.0", already, None, NOW).machine_version,
            "0.2.0+def456-dirty"
        );
        let plain = r#"{"type":"nix","version":"0.2.0","rev":"def456","commitTs":1,"dirty":true}"#;
        assert_eq!(
            resolve("0.1.0", plain, None, NOW).machine_version,
            "0.2.0+def456-dirty"
        );
    }

    #[test]
    fn an_embedded_nix_stamp_wins_over_the_runtime_environment() {
        let embedded =
            r#"{"type":"nix","version":"0.2.0","rev":"embedded","commitTs":1,"dirty":false}"#;
        let runtime = r#"{"type":"local","rev":"runtime","ts":2,"dirty":false}"#;
        let id = resolve("0.1.0", embedded, Some(runtime), NOW);
        assert_eq!(id.rev.as_deref(), Some("embedded"));
    }

    #[test]
    fn the_runtime_environment_wins_over_an_embedded_local_stamp() {
        let embedded = r#"{"type":"local","rev":"embedded","ts":1,"dirty":false}"#;
        let runtime =
            r#"{"type":"nix","version":"0.3.0","rev":"runtime","commitTs":2,"dirty":false}"#;
        let id = resolve("0.1.0", embedded, Some(runtime), NOW);
        assert_eq!(id.rev.as_deref(), Some("runtime"));
        assert_eq!(id.source_kind, "nix");
    }

    #[test]
    fn a_malformed_stamp_degrades_instead_of_failing() {
        for bad in [
            "not json",
            "{}",
            r#"{"type":"local"}"#,
            r#"{"type":"unknown","rev":"x"}"#,
            "{\"type\":\"local\",\"rev\":",
        ] {
            let id = resolve("0.1.0", bad, None, NOW);
            assert_eq!(id.source_kind, "package", "`{bad}` should degrade");
        }
    }

    #[test]
    fn stamp_fields_tolerate_whitespace_and_reordering() {
        let stamp = r#"{ "dirty" : true , "rev" : "abc" , "type" : "local" , "ts" : 5 }"#;
        let id = resolve("0.1.0", stamp, None, NOW);
        assert_eq!(id.source_kind, "local");
        assert_eq!(id.rev.as_deref(), Some("abc"));
        assert!(id.dirty);
    }

    #[test]
    fn relative_time_phrasing() {
        assert_eq!(relative_time(NOW, NOW), "just now");
        assert_eq!(relative_time(NOW - 60, NOW), "1 min ago");
        assert_eq!(relative_time(NOW - 300, NOW), "5 mins ago");
        assert_eq!(relative_time(NOW - 3600, NOW), "1 hour ago");
        assert_eq!(relative_time(NOW - 7200, NOW), "2 hours ago");
        assert_eq!(relative_time(NOW - 86400, NOW), "1 day ago");
        assert_eq!(relative_time(NOW - 3 * 86400, NOW), "3 days ago");
    }

    #[test]
    fn machine_version_never_contains_relative_prose() {
        for stamp in [
            r#"{"type":"local","rev":"abc","ts":1,"dirty":true}"#,
            r#"{"type":"nix","version":"1.0.0","rev":"def","commitTs":1,"dirty":false}"#,
            "",
        ] {
            let id = resolve("0.1.0", stamp, None, NOW);
            for word in ["ago", "committed", "built", "running"] {
                assert!(
                    !id.machine_version.contains(word),
                    "machineVersion `{}` contains `{word}`",
                    id.machine_version
                );
            }
        }
    }

    #[test]
    fn json_carries_every_contract_field() {
        let id = resolve("0.1.0", "", None, NOW);
        let rendered = id.to_json().render();
        for key in [
            "baseVersion",
            "displayVersion",
            "machineVersion",
            "sourceKind",
            "rev",
            "dirty",
            "commitTs",
            "buildTs",
        ] {
            assert!(rendered.contains(key), "missing `{key}` in {rendered}");
        }
    }

    #[test]
    fn the_real_binary_identity_resolves() {
        // Whatever build.rs embedded, this must not panic and must produce a
        // non-empty machine version.
        let id = identity();
        assert!(!id.machine_version.is_empty());
        assert!(["package", "local", "nix"].contains(&id.source_kind));
    }
}
