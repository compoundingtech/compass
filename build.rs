//! Embeds build identity, following the shared build-versioning contract.
//!
//! The canonical stamp is a single-line JSON document under the name
//! `CLI_BUILD_STAMP`, in one of two shapes:
//!
//! ```json
//! {"type":"local","rev":"abc123","ts":1739999700,"dirty":true}
//! {"type":"nix","version":"0.1.0","rev":"def456","commitTs":1739740800,"dirty":false}
//! ```
//!
//! Resolution order at build time:
//!
//! 1. `CLI_BUILD_STAMP` in the build environment — how the Nix packaging layer
//!    injects a `nix` stamp. Passed through verbatim.
//! 2. Otherwise a `local` stamp derived from git.
//! 3. Otherwise empty, which resolves to `sourceKind: package` at runtime.
//!
//! **This must never fail the build.** Compass is packaged with Nix, where the
//! build runs in a sandbox with no git and no `.git` directory. Every step
//! here degrades to the next rather than panicking.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=CLI_BUILD_STAMP");
    // A commit or a dirty working tree changes the stamp.
    for p in [".git/HEAD", ".git/index"] {
        if std::path::Path::new(p).exists() {
            println!("cargo:rerun-if-changed={p}");
        }
    }

    let stamp = std::env::var("CLI_BUILD_STAMP")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(local_stamp)
        .unwrap_or_default();

    // Values reach the compiler as an env var, so newlines are not allowed.
    let stamp = stamp.replace(['\n', '\r'], " ");
    println!("cargo:rustc-env=COMPASS_BUILD_STAMP={stamp}");
}

/// Derive a `local` stamp from git, or `None` when git is unavailable.
fn local_stamp() -> Option<String> {
    let rev = git(&["rev-parse", "--short=12", "HEAD"])?;
    let ts = git(&["log", "-1", "--format=%ct"])
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    // An empty porcelain listing means a clean tree. A git failure here is
    // reported as clean rather than guessed as dirty.
    let dirty = git(&["status", "--porcelain"])
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    Some(format!(
        r#"{{"type":"local","rev":"{}","ts":{},"dirty":{}}}"#,
        escape(&rev),
        ts,
        dirty
    ))
}

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
