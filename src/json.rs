//! A minimal JSON writer.
//!
//! Compass takes no external crates, and `--json` must carry the same fields as
//! the human rendering, so a small ordered-object emitter is enough. Objects
//! preserve insertion order: the JSON reads in the same order as the terminal
//! output, which makes the two renderings diffable by eye.
//!
//! This module only writes JSON. Nothing in Compass parses it.

use std::fmt::Write as _;

#[derive(Debug, Clone)]
pub enum Json {
    Null,
    Bool(bool),
    /// Pre-rendered numeric literal. Constructed via `Json::num`.
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    /// Insertion-ordered object.
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn str(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }

    pub fn num(n: impl Into<i64>) -> Json {
        Json::Num(n.into().to_string())
    }

    pub fn obj(fields: Vec<(&str, Json)>) -> Json {
        Json::Obj(
            fields
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        )
    }

    pub fn arr(items: Vec<Json>) -> Json {
        Json::Arr(items)
    }

    pub fn strs<I, S>(items: I) -> Json
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Json::Arr(items.into_iter().map(|s| Json::Str(s.into())).collect())
    }

    /// Render as pretty-printed JSON with a trailing newline.
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }

    fn write(&self, out: &mut String, indent: usize) {
        let pad = "  ".repeat(indent);
        let inner_pad = "  ".repeat(indent + 1);
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => out.push_str(n),
            Json::Str(s) => escape_into(s, out),
            Json::Arr(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    out.push_str(&inner_pad);
                    item.write(out, indent + 1);
                    if i + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                let _ = write!(out, "{pad}]");
            }
            Json::Obj(fields) => {
                if fields.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push_str("{\n");
                for (i, (k, v)) in fields.iter().enumerate() {
                    out.push_str(&inner_pad);
                    escape_into(k, out);
                    out.push_str(": ");
                    v.write(out, indent + 1);
                    if i + 1 < fields.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                let _ = write!(out, "{pad}}}");
            }
        }
    }
}

/// Write `s` as a quoted, escaped JSON string.
fn escape_into(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(j: &Json) -> String {
        let mut out = String::new();
        j.write(&mut out, 0);
        out
    }

    #[test]
    fn escapes_the_dangerous_characters() {
        assert_eq!(s(&Json::str("a\"b")), r#""a\"b""#);
        assert_eq!(s(&Json::str("a\\b")), r#""a\\b""#);
        assert_eq!(s(&Json::str("a\nb")), r#""a\nb""#);
        assert_eq!(s(&Json::str("a\tb")), r#""a\tb""#);
        assert_eq!(s(&Json::str("a\rb")), r#""a\rb""#);
    }

    #[test]
    fn escapes_control_characters_as_unicode() {
        assert_eq!(s(&Json::str("\u{01}")), r#""\u0001""#);
        assert_eq!(s(&Json::str("\u{1f}")), r#""\u001f""#);
        assert_eq!(s(&Json::str("\u{08}")), r#""\b""#);
        assert_eq!(s(&Json::str("\u{0c}")), r#""\f""#);
    }

    #[test]
    fn passes_through_non_ascii_unescaped() {
        assert_eq!(s(&Json::str("héllo → ✓")), "\"héllo → ✓\"");
    }

    #[test]
    fn empty_containers_are_compact() {
        assert_eq!(s(&Json::Arr(vec![])), "[]");
        assert_eq!(s(&Json::Obj(vec![])), "{}");
    }

    #[test]
    fn nested_rendering_is_stable_and_ordered() {
        let j = Json::obj(vec![
            ("plan", Json::str("pl_ABC")),
            ("seq", Json::num(2)),
            ("converged", Json::Bool(false)),
            ("parent", Json::strs(["aa", "bb"])),
            ("nothing", Json::Null),
        ]);
        assert_eq!(
            s(&j),
            "{\n  \"plan\": \"pl_ABC\",\n  \"seq\": 2,\n  \"converged\": false,\n  \"parent\": [\n    \"aa\",\n    \"bb\"\n  ],\n  \"nothing\": null\n}"
        );
    }

    #[test]
    fn render_appends_a_single_newline() {
        assert_eq!(Json::str("x").render(), "\"x\"\n");
    }
}
