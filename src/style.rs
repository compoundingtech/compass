//! Terminal styling.
//!
//! Colour is applied only when stdout is a terminal and `NO_COLOR` is unset,
//! so piped and captured output stays plain. Status is never carried by colour
//! alone — every marker is also a symbol — because colour is unavailable to
//! some readers and absent in logs.
//!
//! No emoji: they render inconsistently across terminals and add width the
//! layout cannot predict.

use std::io::IsTerminal;
use std::sync::OnceLock;

fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal())
}

/// The styling rule itself, independent of how stdout happens to be wired.
/// Kept separate from [`enabled`] so it can be tested for both states — a test
/// that infers the flag from its own environment tests the environment.
fn wrap_when(on: bool, code: &str, s: &str) -> String {
    if on && !s.is_empty() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn wrap(code: &str, s: &str) -> String {
    wrap_when(enabled(), code, s)
}

pub fn bold(s: &str) -> String {
    wrap("1", s)
}

pub fn dim(s: &str) -> String {
    wrap("2", s)
}

pub fn red(s: &str) -> String {
    wrap("31", s)
}

pub fn green(s: &str) -> String {
    wrap("32", s)
}

pub fn yellow(s: &str) -> String {
    wrap("33", s)
}

pub fn cyan(s: &str) -> String {
    wrap("36", s)
}

/// `CRITICAL` badge: white bold on red.
pub fn critical() -> String {
    wrap("1;37;41", " CRITICAL ")
}

/// `WARNING` badge: black bold on yellow.
pub fn warning() -> String {
    wrap("1;30;43", " WARNING ")
}

/// A `fix:` line — the actionable command that resolves a problem.
pub fn fix(cmd: &str) -> String {
    format!("    {} {}", cyan("fix:"), cmd)
}

/// A `note:` line — context that is not itself actionable.
pub fn note(text: &str) -> String {
    format!("    {} {}", dim("note:"), dim(text))
}

/// Truncate to `max` display columns, marking elision.
pub fn truncate(s: &str, max: usize) -> String {
    let flat = s.replace('\n', " ");
    if flat.chars().count() <= max {
        return flat;
    }
    let keep: String = flat.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", keep.trim_end())
}

/// Wrap text to `width` columns, preserving the author's own line breaks.
///
/// Truncation is right for a title in a dense listing, where the reader can
/// ask for the rest. It is wrong for a Rationale under divergence: that text
/// is the whole basis on which a reconciliation is decided, and eliding it
/// hides the reasoning exactly where it is most needed. Wrapping costs lines
/// and keeps the words.
pub fn wrap_text(s: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for para in s.lines() {
        let mut line = String::new();
        let mut cols = 0usize;
        for word in para.split_whitespace() {
            let w = word.chars().count();
            if cols > 0 && cols + 1 + w > width {
                out.push(std::mem::take(&mut line));
                cols = 0;
            }
            if cols > 0 {
                line.push(' ');
                cols += 1;
            }
            line.push_str(word);
            cols += w;
        }
        // A blank input line is a paragraph break the author wrote; keep it.
        out.push(line);
    }
    out
}

/// Wrap and indent, ready to append to a report.
pub fn wrapped_block(s: &str, indent: &str, width: usize) -> String {
    let mut out = String::new();
    for l in wrap_text(s, width.saturating_sub(indent.chars().count())) {
        if l.is_empty() {
            out.push('\n');
        } else {
            out.push_str(&format!("{indent}{}\n", dim(&l)));
        }
    }
    out
}

/// `n` followed by `noun`, pluralised with a trailing `s` when `n != 1`.
///
/// "1 events" reads as a bug in the counter and costs the reader a moment
/// deciding whether it is one.
pub fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("{n} {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// `n` followed by a noun whose plural is not formed by adding `s`.
pub fn count_of(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("{n} {one}")
    } else {
        format!("{n} {many}")
    }
}

/// Short form of a content hash for display. The full hash remains identity.
pub fn short(hash: &str) -> String {
    hash.chars().take(12).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styling_is_inert_when_disabled() {
        assert_eq!(wrap_when(false, "1", "x"), "x");
        assert_eq!(wrap_when(false, "1;37;41", " CRITICAL "), " CRITICAL ");
        assert!(!wrap_when(false, "31", "x").contains('\x1b'));
    }

    #[test]
    fn styling_wraps_when_enabled() {
        assert_eq!(wrap_when(true, "1", "x"), "\x1b[1mx\x1b[0m");
        assert!(wrap_when(true, "31", "x").contains('\x1b'));
    }

    #[test]
    fn empty_input_is_never_wrapped() {
        // An escape pair around nothing is invisible output that still costs
        // width in a layout and noise in a log.
        assert_eq!(wrap_when(true, "1", ""), "");
    }

    #[test]
    fn truncate_marks_elision_and_flattens_newlines() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("a\nb", 10), "a b");
        let t = truncate("abcdefghij", 5);
        assert!(t.ends_with('…'));
        assert_eq!(t.chars().count(), 5);
    }

    #[test]
    fn truncate_handles_multibyte_without_panicking() {
        let t = truncate("héllo wörld ✓✓✓", 6);
        assert_eq!(t.chars().count(), 6);
    }

    #[test]
    fn short_hash_is_twelve_characters() {
        assert_eq!(short(&"a".repeat(64)), "a".repeat(12));
        assert_eq!(short("abc"), "abc");
    }
}
