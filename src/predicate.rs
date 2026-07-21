//! The acceptance predicate language (CMP-R11, decision 0006, DQ03).
//!
//! Decision 0006 settles that acceptance cannot be prose, because readiness
//! must be computable without a judge. It does not settle the vocabulary —
//! that is DQ03, the largest remaining design task. This module implements the
//! smallest language that satisfies the stated constraints, and no more.
//!
//! ```text
//! accept = test(name=parser::nested_groups, status=pass)
//! accept = all(test(name=x, status=pass), review(by=cos))
//! accept = any(test(name=x, status=pass), waiver(by=cos))
//! accept = not(test(name=x, status=fail))
//! ```
//!
//! An **atom** `kind(k=v, ...)` holds when a recorded evidence event of that
//! kind exists carrying *all* of those attributes. Extra attributes on the
//! event are ignored, so an atom is a subset match: evidence may carry more
//! context than the predicate demands without breaking it.
//!
//! `all` / `any` / `not` compose. They are reserved words and cannot name an
//! evidence kind.
//!
//! Evaluation is monotone in the evidence set except through `not`: adding
//! evidence can only satisfy more atoms, but can falsify a `not`. This is
//! deliberate — `not(test(status=fail))` is the natural way to express "no
//! known failure" — and it means acceptance is not permanent. A step can
//! become unaccepted when new failing evidence arrives, which is the honest
//! behaviour under append-only progress.
//!
//! ## What DQ03 asks that this does not answer
//!
//! - **Cross-step reference.** Predicates cannot name another Step. The
//!   dependency graph (`depends_on`) already expresses ordering, and letting
//!   acceptance reach across steps would give two mechanisms for one relation.
//! - **Unsatisfiability detection.** `all(p, not(p))` can never hold, and
//!   nothing here detects that. Detecting it in general is a solver problem;
//!   detecting the syntactic case would be cheap but would imply a guarantee
//!   the language cannot keep.
//!
//! ## Canonical form
//!
//! Rendering sorts attributes by key and normalises spacing, so two authors
//! expressing the same criterion produce identical bytes and therefore an
//! identical version hash. Attribute order carries no meaning, so nothing is
//! lost. Argument order in `all` / `any` *is* preserved, because it determines
//! which unsatisfied branch gets reported first.

use std::fmt;

/// A parsed acceptance predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pred {
    /// `kind(k=v, ...)` — an evidence event of `kind` with all these attributes.
    Atom {
        kind: String,
        attrs: Vec<(String, String)>,
    },
    All(Vec<Pred>),
    Any(Vec<Pred>),
    Not(Box<Pred>),
}

/// One recorded evidence event, reduced to what acceptance evaluates over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub kind: String,
    pub attrs: Vec<(String, String)>,
}

impl Evidence {
    pub fn new(kind: impl Into<String>, attrs: Vec<(String, String)>) -> Evidence {
        Evidence {
            kind: kind.into(),
            attrs,
        }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

impl Pred {
    /// Whether this predicate holds against the recorded evidence.
    pub fn eval(&self, evidence: &[Evidence]) -> bool {
        match self {
            Pred::Atom { kind, attrs } => evidence.iter().any(|e| {
                e.kind == *kind && attrs.iter().all(|(k, v)| e.get(k) == Some(v.as_str()))
            }),
            // An empty `all` is vacuously satisfied; an empty `any` is not.
            Pred::All(ps) => ps.iter().all(|p| p.eval(evidence)),
            Pred::Any(ps) => ps.iter().any(|p| p.eval(evidence)),
            Pred::Not(p) => !p.eval(evidence),
        }
    }

    /// Why this predicate does not hold, or `None` when it does.
    ///
    /// Readiness must explain itself (CMP-R12), and an explanation naming the
    /// whole predicate is not an explanation — it must name the branch that
    /// actually failed.
    pub fn explain(&self, evidence: &[Evidence]) -> Option<String> {
        if self.eval(evidence) {
            return None;
        }
        Some(match self {
            Pred::Atom { .. } => format!("no evidence matching {self}"),
            Pred::All(ps) => {
                // Report the first unsatisfied conjunct: it is the nearest
                // thing to do next.
                match ps.iter().find_map(|p| p.explain(evidence)) {
                    Some(reason) => reason,
                    None => format!("{self} is unsatisfied"),
                }
            }
            Pred::Any(ps) => {
                if ps.is_empty() {
                    "any() with no alternatives can never be satisfied".to_string()
                } else {
                    let parts: Vec<String> = ps.iter().map(|p| p.to_string()).collect();
                    format!("none of these hold: {}", parts.join(", "))
                }
            }
            Pred::Not(inner) => format!("evidence matching {inner} is present but must not be"),
        })
    }
}

impl fmt::Display for Pred {
    /// Canonical rendering. This is what gets stored and hashed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pred::Atom { kind, attrs } => {
                write!(f, "{kind}(")?;
                for (i, (k, v)) in attrs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}={}", render_value(v))?;
                }
                write!(f, ")")
            }
            Pred::All(ps) | Pred::Any(ps) => {
                let name = if matches!(self, Pred::All(_)) {
                    "all"
                } else {
                    "any"
                };
                write!(f, "{name}(")?;
                for (i, p) in ps.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ")")
            }
            Pred::Not(p) => write!(f, "not({p})"),
        }
    }
}

/// Quote a value only when it would otherwise not survive a round trip.
fn render_value(v: &str) -> String {
    let needs_quotes = v.is_empty()
        || v.chars()
            .any(|c| c == ',' || c == ')' || c == '(' || c == '"' || c == '=' || c.is_whitespace());
    if !needs_quotes {
        return v.to_string();
    }
    let mut out = String::with_capacity(v.len() + 2);
    out.push('"');
    for c in v.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredError {
    pub message: String,
    pub position: usize,
}

impl fmt::Display for PredError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at offset {})", self.message, self.position)
    }
}

/// Parse an acceptance predicate.
pub fn parse(src: &str) -> Result<Pred, PredError> {
    let mut p = Parser {
        src: src.as_bytes(),
        pos: 0,
    };
    p.skip_ws();
    let pred = p.parse_pred()?;
    p.skip_ws();
    if p.pos < p.src.len() {
        return Err(p.err("unexpected trailing input"));
    }
    Ok(pred)
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, message: impl Into<String>) -> PredError {
        PredError {
            message: message.into(),
            position: self.pos,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), PredError> {
        self.skip_ws();
        if self.peek() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.err(format!("expected `{}`", c as char)))
        }
    }

    fn parse_pred(&mut self) -> Result<Pred, PredError> {
        self.skip_ws();
        let name = self.parse_ident()?;
        match name.as_str() {
            "all" => Ok(Pred::All(self.parse_pred_list()?)),
            "any" => Ok(Pred::Any(self.parse_pred_list()?)),
            "not" => {
                self.expect(b'(')?;
                let inner = self.parse_pred()?;
                self.expect(b')')?;
                Ok(Pred::Not(Box::new(inner)))
            }
            _ => {
                let attrs = self.parse_attrs()?;
                Ok(Pred::Atom { kind: name, attrs })
            }
        }
    }

    fn parse_pred_list(&mut self) -> Result<Vec<Pred>, PredError> {
        self.expect(b'(')?;
        let mut out = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b')') {
            self.pos += 1;
            return Ok(out);
        }
        loop {
            out.push(self.parse_pred()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b')') => {
                    self.pos += 1;
                    return Ok(out);
                }
                _ => return Err(self.err("expected `,` or `)` in predicate list")),
            }
        }
    }

    fn parse_attrs(&mut self) -> Result<Vec<(String, String)>, PredError> {
        self.expect(b'(')?;
        let mut attrs: Vec<(String, String)> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b')') {
            self.pos += 1;
            return Ok(attrs);
        }
        loop {
            self.skip_ws();
            let key = self.parse_ident()?;
            self.expect(b'=')?;
            let value = self.parse_value()?;
            if attrs.iter().any(|(k, _)| *k == key) {
                return Err(self.err(format!("duplicate attribute `{key}`")));
            }
            attrs.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b')') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.err("expected `,` or `)` in attribute list")),
            }
        }
        // Canonical order: attribute order carries no meaning.
        attrs.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(attrs)
    }

    fn parse_ident(&mut self) -> Result<String, PredError> {
        self.skip_ws();
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' || c == b'.' || c == b':' || c == b'-' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(self.err("expected an identifier"));
        }
        Ok(String::from_utf8_lossy(&self.src[start..self.pos]).into_owned())
    }

    fn parse_value(&mut self) -> Result<String, PredError> {
        self.skip_ws();
        if self.peek() == Some(b'"') {
            return self.parse_quoted();
        }
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == b',' || c == b')' || c == b'(' {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            return Err(self.err("expected a value"));
        }
        Ok(String::from_utf8_lossy(&self.src[start..self.pos])
            .trim()
            .to_string())
    }

    fn parse_quoted(&mut self) -> Result<String, PredError> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err(self.err("unterminated quoted value")),
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(c) => {
                            out.push(c as char);
                            self.pos += 1;
                        }
                        None => return Err(self.err("trailing escape in quoted value")),
                    }
                }
                Some(_) => {
                    // Step by whole characters so multi-byte UTF-8 survives.
                    let rest = &self.src[self.pos..];
                    let s = String::from_utf8_lossy(rest);
                    let c = s.chars().next().unwrap_or('\u{fffd}');
                    out.push(c);
                    self.pos += c.len_utf8();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(kind: &str, attrs: &[(&str, &str)]) -> Evidence {
        Evidence::new(
            kind,
            attrs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }

    fn p(src: &str) -> Pred {
        parse(src).unwrap_or_else(|e| panic!("parse `{src}` failed: {e}"))
    }

    // --- parsing ---------------------------------------------------------

    #[test]
    fn parses_an_atom_with_attributes() {
        assert_eq!(
            p("test(name=parser::nested_groups, status=pass)"),
            Pred::Atom {
                kind: "test".into(),
                attrs: vec![
                    ("name".into(), "parser::nested_groups".into()),
                    ("status".into(), "pass".into()),
                ],
            }
        );
    }

    #[test]
    fn parses_an_atom_with_no_attributes() {
        assert_eq!(
            p("deployed()"),
            Pred::Atom {
                kind: "deployed".into(),
                attrs: vec![],
            }
        );
    }

    #[test]
    fn parses_nested_combinators() {
        let pred =
            p("all(test(status=pass), any(review(by=cos), waiver()), not(test(status=fail)))");
        let Pred::All(parts) = &pred else {
            panic!("expected all, got {pred:?}")
        };
        assert_eq!(parts.len(), 3);
        assert!(matches!(parts[1], Pred::Any(_)));
        assert!(matches!(parts[2], Pred::Not(_)));
    }

    #[test]
    fn parses_quoted_values_with_separators_inside() {
        let pred = p(r#"note(text="a, b) c", who=cos)"#);
        let Pred::Atom { attrs, .. } = &pred else {
            panic!()
        };
        assert_eq!(attrs[0], ("text".to_string(), "a, b) c".to_string()));
    }

    #[test]
    fn tolerates_surrounding_whitespace() {
        assert_eq!(
            p("  all( test(status=pass) ,  review(by=cos) )  "),
            p("all(test(status=pass), review(by=cos))")
        );
    }

    #[test]
    fn rejects_malformed_input() {
        for bad in [
            "test(",
            "test(name)",
            "test(name=)",
            "all(test(status=pass)",
            "not()",
            "test(status=pass) trailing",
            "",
            "test(a=1, a=2)",
            r#"note(text="unterminated)"#,
        ] {
            assert!(parse(bad).is_err(), "expected `{bad}` to fail to parse");
        }
    }

    // --- evaluation ------------------------------------------------------

    #[test]
    fn atom_requires_all_named_attributes() {
        let e = vec![ev("test", &[("name", "x"), ("status", "pass")])];
        assert!(p("test(name=x, status=pass)").eval(&e));
        assert!(!p("test(name=x, status=fail)").eval(&e));
        assert!(!p("test(name=other, status=pass)").eval(&e));
        assert!(!p("build(name=x, status=pass)").eval(&e));
    }

    #[test]
    fn atom_is_a_subset_match_extra_evidence_attrs_are_ignored() {
        let e = vec![ev(
            "test",
            &[("name", "x"), ("status", "pass"), ("host", "dev3")],
        )];
        assert!(p("test(status=pass)").eval(&e));
        assert!(p("test()").eval(&e));
    }

    #[test]
    fn atom_matches_across_separate_events_not_within_one() {
        // Two attributes demanded by one atom must be carried by a *single*
        // event; satisfying them from two different events would let unrelated
        // facts combine into an acceptance nobody recorded.
        let split = vec![
            ev("test", &[("name", "x")]),
            ev("test", &[("status", "pass")]),
        ];
        assert!(!p("test(name=x, status=pass)").eval(&split));

        let together = vec![ev("test", &[("name", "x"), ("status", "pass")])];
        assert!(p("test(name=x, status=pass)").eval(&together));
    }

    #[test]
    fn combinators_evaluate() {
        let e = vec![ev("test", &[("status", "pass")])];
        assert!(p("all(test(status=pass))").eval(&e));
        assert!(!p("all(test(status=pass), review(by=cos))").eval(&e));
        assert!(p("any(test(status=pass), review(by=cos))").eval(&e));
        assert!(!p("any(review(by=cos), waiver(by=cos))").eval(&e));
        assert!(p("not(test(status=fail))").eval(&e));
        assert!(!p("not(test(status=pass))").eval(&e));
    }

    #[test]
    fn empty_combinators_follow_the_usual_identities() {
        let e: Vec<Evidence> = vec![];
        assert!(p("all()").eval(&e), "empty all is vacuously true");
        assert!(!p("any()").eval(&e), "empty any is false");
    }

    #[test]
    fn nothing_is_satisfied_by_an_empty_evidence_set() {
        let e: Vec<Evidence> = vec![];
        assert!(!p("test(status=pass)").eval(&e));
        assert!(!p("all(test(status=pass), review(by=cos))").eval(&e));
    }

    #[test]
    fn negation_makes_acceptance_revocable() {
        let pred = p("not(test(status=fail))");
        assert!(pred.eval(&[]));
        assert!(!pred.eval(&[ev("test", &[("status", "fail")])]));
    }

    // --- explanation -----------------------------------------------------

    #[test]
    fn satisfied_predicates_have_nothing_to_explain() {
        let e = vec![ev("test", &[("status", "pass")])];
        assert_eq!(p("test(status=pass)").explain(&e), None);
    }

    #[test]
    fn explanation_names_the_failing_branch_not_the_whole_predicate() {
        let e = vec![ev("test", &[("status", "pass")])];
        let reason = p("all(test(status=pass), review(by=cos))")
            .explain(&e)
            .unwrap();
        assert!(reason.contains("review(by=cos)"), "{reason}");
        assert!(!reason.contains("test(status=pass)"), "{reason}");
    }

    #[test]
    fn explanations_cover_each_combinator() {
        let e: Vec<Evidence> = vec![ev("test", &[("status", "fail")])];
        assert!(p("review(by=cos)")
            .explain(&e)
            .unwrap()
            .contains("no evidence matching"));
        assert!(p("any(review(by=cos), waiver())")
            .explain(&e)
            .unwrap()
            .contains("none of these hold"));
        assert!(p("not(test(status=fail))")
            .explain(&e)
            .unwrap()
            .contains("must not be"));
    }

    // --- canonical form --------------------------------------------------

    #[test]
    fn rendering_sorts_attributes_so_equal_intent_hashes_equally() {
        assert_eq!(
            p("test(status=pass, name=x)").to_string(),
            "test(name=x, status=pass)"
        );
        assert_eq!(
            p("test(status=pass, name=x)"),
            p("test(name=x, status=pass)")
        );
    }

    #[test]
    fn rendering_preserves_combinator_argument_order() {
        // Argument order decides which branch is reported first, so it is not
        // sorted away.
        assert_eq!(p("all(b(), a())").to_string(), "all(b(), a())");
    }

    #[test]
    fn canonical_form_round_trips() {
        for src in [
            "test(name=parser::nested_groups, status=pass)",
            "all(test(status=pass), not(build(status=fail)))",
            "any(review(by=cos), waiver(by=dev))",
            "deployed()",
            r#"note(text="a, b) c")"#,
            "not(all(a(), b()))",
        ] {
            let once = p(src);
            let text = once.to_string();
            let twice = p(&text);
            assert_eq!(once, twice, "`{src}` did not round trip via `{text}`");
            assert_eq!(text, twice.to_string(), "rendering is not a fixed point");
        }
    }

    #[test]
    fn values_needing_quotes_are_quoted() {
        assert_eq!(render_value("plain"), "plain");
        assert_eq!(render_value("with space"), "\"with space\"");
        assert_eq!(render_value("with,comma"), "\"with,comma\"");
        assert_eq!(render_value(""), "\"\"");
        assert_eq!(render_value(r#"quo"te"#), r#""quo\"te""#);
    }
}
