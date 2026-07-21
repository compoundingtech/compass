//! The block serialization syntax.
//!
//! **This syntax is provisional (DQ02).** The spec's working assumption is KDL
//! for versions and JSON for events, but DQ02 is explicitly unresolved, and
//! Compass takes no external crates — so a KDL parser would be a large
//! hand-rolled surface committed to before the question is settled. This
//! module implements the smallest format that carries the field set, is
//! readable and diffable by a human, and is cheap to discard when DQ02 lands.
//!
//! One format serves both versions and events, against the spec's assumption
//! of JSON for events: two hand-rolled parsers would be two things to throw
//! away rather than one. Noted as a deliberate deviation.
//!
//! ```text
//! @version
//! plan = pl_7Kq2
//! seq = 1
//! author = cos
//! why = Initial plan.
//!
//! @step st_a1
//! work = Reproduce with a failing test
//! accept = test(name=parser::nested_groups, status=fail)
//! ```
//!
//! A line beginning `@` opens a block, optionally with an argument. Subsequent
//! `key = value` lines belong to it. A repeated key is a multi-value field
//! (`parent`, `depends_on`); nothing here collapses duplicates. Blank lines are
//! insignificant and `#` at the start of a line is a comment.
//!
//! Values are single-line on the wire. A value containing a newline is escaped
//! (`\n`), so a multi-line Rationale round-trips exactly — required, since the
//! Rationale chain is the artifact the tool exists to preserve.

use std::fmt;

/// One `@kind arg` block and its ordered key/value entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub kind: String,
    pub arg: Option<String>,
    pub entries: Vec<(String, String)>,
}

impl Block {
    pub fn new(kind: impl Into<String>, arg: Option<String>) -> Block {
        Block {
            kind: kind.into(),
            arg,
            entries: Vec::new(),
        }
    }

    /// Append a key/value entry.
    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        self.entries.push((key.to_string(), value.into()));
    }

    /// Append a key/value entry only when `value` is `Some`.
    pub fn set_opt(&mut self, key: &str, value: Option<impl Into<String>>) {
        if let Some(v) = value {
            self.set(key, v);
        }
    }

    /// Append one entry per item, in the order given.
    pub fn set_many<I, S>(&mut self, key: &str, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for v in values {
            self.set(key, v);
        }
    }

    /// The single value for `key`, or `None` if absent.
    ///
    /// If the key repeats, the first value wins. Callers that expect a
    /// multi-value field must use [`Block::all`].
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Every value for `key`, in document order.
    pub fn all(&self, key: &str) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    /// The single value for `key`, erroring when absent or empty.
    pub fn require(&self, key: &str) -> Result<&str, ParseError> {
        match self.get(key) {
            Some(v) if !v.trim().is_empty() => Ok(v),
            Some(_) => Err(ParseError::new(format!(
                "`{}` block: `{key}` is present but empty",
                self.kind
            ))),
            None => Err(ParseError::new(format!(
                "`{}` block: missing required key `{key}`",
                self.kind
            ))),
        }
    }
}

/// A parsed document: an ordered list of blocks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Doc {
    pub blocks: Vec<Block>,
}

impl Doc {
    pub fn new() -> Doc {
        Doc::default()
    }

    pub fn push(&mut self, b: Block) {
        self.blocks.push(b);
    }

    /// The first block of the given kind.
    pub fn first(&self, kind: &str) -> Option<&Block> {
        self.blocks.iter().find(|b| b.kind == kind)
    }

    /// All blocks of the given kind, in document order.
    pub fn of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a Block> {
        self.blocks.iter().filter(move |b| b.kind == kind)
    }

    /// Render canonically.
    ///
    /// Byte-for-byte determinism matters: a version's identity is the hash of
    /// these bytes, so the same intent must render identically on every
    /// machine. Field order is the order the caller built, which the model
    /// layer fixes; set-valued keys are sorted by the model layer before they
    /// reach here.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for (i, b) in self.blocks.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push('@');
            out.push_str(&b.kind);
            if let Some(arg) = &b.arg {
                out.push(' ');
                out.push_str(arg);
            }
            out.push('\n');
            for (k, v) in &b.entries {
                out.push_str(k);
                out.push_str(" = ");
                out.push_str(&escape(v));
                out.push('\n');
            }
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: Option<usize>,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            line: None,
        }
    }

    fn at(line: usize, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            line: Some(line),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(l) => write!(f, "line {l}: {}", self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

/// Parse a document. Errors carry a line number, because the operator reading
/// the error is looking at the file.
pub fn parse(text: &str) -> Result<Doc, ParseError> {
    let mut doc = Doc::new();
    let mut current: Option<Block> = None;

    for (idx, raw) in text.lines().enumerate() {
        let lineno = idx + 1;
        let line = raw.trim_end();
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(header) = trimmed.strip_prefix('@') {
            if let Some(b) = current.take() {
                doc.push(b);
            }
            let mut parts = header.splitn(2, char::is_whitespace);
            let kind = parts.next().unwrap_or("").trim();
            if kind.is_empty() {
                return Err(ParseError::at(lineno, "block header has no kind"));
            }
            let arg = parts.next().map(|a| a.trim()).filter(|a| !a.is_empty());
            current = Some(Block::new(kind, arg.map(|a| a.to_string())));
            continue;
        }

        let Some(eq) = trimmed.find('=') else {
            return Err(ParseError::at(
                lineno,
                format!("expected `key = value` or a `@block` header, found `{trimmed}`"),
            ));
        };
        let key = trimmed[..eq].trim();
        let value = trimmed[eq + 1..].trim();
        if key.is_empty() {
            return Err(ParseError::at(lineno, "entry has an empty key"));
        }
        let Some(block) = current.as_mut() else {
            return Err(ParseError::at(
                lineno,
                format!("entry `{key}` appears before any `@block` header"),
            ));
        };
        block.set(key, unescape(value));
    }

    if let Some(b) = current.take() {
        doc.push(b);
    }
    Ok(doc)
}

/// Escape a value for single-line storage.
fn escape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

/// Inverse of [`escape`]. An unknown escape is preserved literally rather than
/// dropped, so no authored byte is silently lost.
fn unescape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut chars = v.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_worked_example_shape() {
        let text = "\
@version
plan = pl_7Kq2
seq = 1
author = cos
at = 1
why = Initial plan. Parser rejects nested groups.
goal = Nested groups parse correctly

@step st_a1
work = Reproduce with a failing test
accept = test(name=parser::nested_groups, status=fail)

@step st_b2
work = Fix the tokenizer
depends_on = st_a1
accept = test(name=parser::nested_groups, status=pass)
";
        let doc = parse(text).unwrap();
        assert_eq!(doc.blocks.len(), 3);

        let v = doc.first("version").unwrap();
        assert_eq!(v.get("plan"), Some("pl_7Kq2"));
        assert_eq!(v.get("seq"), Some("1"));
        assert_eq!(v.arg, None);

        let steps: Vec<_> = doc.of_kind("step").collect();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].arg.as_deref(), Some("st_a1"));
        assert_eq!(steps[1].get("depends_on"), Some("st_a1"));
        // A value containing `=` keeps everything after the first separator.
        assert_eq!(
            steps[1].get("accept"),
            Some("test(name=parser::nested_groups, status=pass)")
        );
    }

    #[test]
    fn repeated_keys_are_multi_value() {
        let doc = parse("@version\nparent = aaa\nparent = bbb\n").unwrap();
        let v = doc.first("version").unwrap();
        assert_eq!(v.all("parent"), vec!["aaa", "bbb"]);
        // `get` takes the first; multi-value fields must use `all`.
        assert_eq!(v.get("parent"), Some("aaa"));
    }

    #[test]
    fn round_trips_through_render_and_parse() {
        let mut doc = Doc::new();
        let mut v = Block::new("version", None);
        v.set("plan", "pl_7Kq2");
        v.set_many("parent", ["aaa", "bbb"]);
        v.set("why", "Line one.\nLine two.\\ with a backslash");
        doc.push(v);
        let mut s = Block::new("step", Some("st_a1".into()));
        s.set("work", "Do the thing");
        doc.push(s);

        let rendered = doc.render();
        let reparsed = parse(&rendered).unwrap();
        assert_eq!(reparsed, doc);
        // And rendering is a fixed point.
        assert_eq!(reparsed.render(), rendered);
    }

    #[test]
    fn multiline_values_survive_the_round_trip() {
        let mut doc = Doc::new();
        let mut v = Block::new("version", None);
        v.set(
            "why",
            "Reproduction showed the tokenizer,\nnot the grammar.\r\nRetargeting.",
        );
        doc.push(v);

        let rendered = doc.render();
        assert_eq!(rendered.lines().count(), 2, "value must stay on one line");
        assert_eq!(parse(&rendered).unwrap(), doc);
    }

    #[test]
    fn empty_and_comment_lines_are_insignificant() {
        let a = parse("@version\nplan = pl_1\n").unwrap();
        let b = parse("\n# a comment\n@version\n\nplan = pl_1\n\n# trailing\n").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn rejects_entries_before_a_block_header() {
        let err = parse("plan = pl_1\n").unwrap_err();
        assert_eq!(err.line, Some(1));
        assert!(err.message.contains("before any"), "{}", err.message);
    }

    #[test]
    fn rejects_a_line_that_is_neither_header_nor_entry() {
        let err = parse("@version\njust some prose\n").unwrap_err();
        assert_eq!(err.line, Some(2));
    }

    #[test]
    fn rejects_an_empty_block_kind() {
        assert!(parse("@\nplan = x\n").is_err());
    }

    #[test]
    fn require_distinguishes_missing_from_empty() {
        let doc = parse("@version\nwhy = \n").unwrap();
        let v = doc.first("version").unwrap();
        assert!(v.require("why").unwrap_err().message.contains("empty"));
        assert!(v.require("goal").unwrap_err().message.contains("missing"));
    }

    #[test]
    fn render_is_byte_stable_for_equal_documents() {
        let build = || {
            let mut doc = Doc::new();
            let mut v = Block::new("version", None);
            v.set("plan", "pl_X");
            v.set_many("parent", ["a", "b"]);
            doc.push(v);
            doc
        };
        assert_eq!(build().render(), build().render());
    }

    #[test]
    fn unknown_escapes_are_preserved_not_dropped() {
        assert_eq!(unescape(r"a\qb"), r"a\qb");
        assert_eq!(unescape(r"trailing\"), r"trailing\");
    }
}
