//! Progress Events — the execution layer (CMP-R10, decision 0005).
//!
//! Events are append-only records against a Step. They never alter structural
//! intent and never create a Plan Version. Correction is a further event,
//! never an edit.
//!
//! An event names the version it was observed against, so an event recorded
//! against a Step that a later version supersedes can still be attributed
//! through the `supersedes` edge.
//!
//! **Only `evidence` events feed acceptance.** `start`, `update`, `handoff`
//! and `done` are operational markers: they say what an actor is doing, not
//! what is true. A `done` event does not complete a Step — acceptance is
//! Compass-judged from the Step's own `accept` predicate (CMP-R14). This is
//! the whole point of decision 0006 and is easy to get backwards.
//!
//! Ordering uses a logical counter, not wall clock, for the same reason
//! versions do: machines skew. Wall time is recorded as `wall` for a human
//! reading the log, and nothing orders on it.

use crate::block::{parse as parse_block, Block, Doc, ParseError};
use crate::model::EXT;
use crate::predicate::Evidence;
use std::time::{SystemTime, UNIX_EPOCH};

/// The names Compass records on an event itself, as opposed to the `@attrs`
/// a writer supplies (decision 0008, CMP.DM-R13b).
///
/// These names are reserved. A predicate term naming one binds the recorded
/// value; an evidence attribute using one is refused. Keep this in step with
/// [`Event::render`] — every key written into the `@event` block belongs here,
/// or a writer regains a name that a predicate would read as recorded.
pub const RECORDED_FIELDS: [&str; 10] = [
    "id",
    "at",
    "wall",
    "plan",
    "step",
    "version",
    "actor",
    "kind",
    "evidence_kind",
    "note",
];

/// Whether `name` is recorded by Compass rather than claimed by a writer.
///
/// The single authority for the reserved set: write-time refusal, read-time
/// rejection, and predicate binding all ask this, so they cannot drift apart.
pub fn is_recorded_field(name: &str) -> bool {
    RECORDED_FIELDS.contains(&name)
}

/// What kind of progress an event records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Start,
    Update,
    Handoff,
    Done,
    Evidence,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::Start => "start",
            EventKind::Update => "update",
            EventKind::Handoff => "handoff",
            EventKind::Done => "done",
            EventKind::Evidence => "evidence",
        }
    }

    pub fn parse(s: &str) -> Option<EventKind> {
        Some(match s {
            "start" => EventKind::Start,
            "update" => EventKind::Update,
            "handoff" => EventKind::Handoff,
            "done" => EventKind::Done,
            "evidence" => EventKind::Evidence,
            _ => return None,
        })
    }

    /// The kinds a `compass progress` invocation may record. `evidence` is
    /// excluded: it has its own command because it carries attributes.
    pub const PROGRESS_KINDS: [&'static str; 4] = ["start", "update", "handoff", "done"];
}

/// One append-only progress record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: String,
    /// Logical time, used for ordering and for the filename.
    pub at: u64,
    /// Unix seconds when written. Informational only; nothing orders on it.
    pub wall: u64,
    pub plan: String,
    pub step: String,
    /// Content hash of the version this was observed against.
    pub version: String,
    pub actor: String,
    pub kind: EventKind,
    pub note: Option<String>,
    /// For `evidence` events: the evidence kind an `accept` atom matches on.
    pub evidence_kind: Option<String>,
    /// For `evidence` events: arbitrary attributes, sorted by key.
    pub attrs: Vec<(String, String)>,
}

impl Event {
    /// Reduce to the form acceptance evaluates over, if this is evidence.
    ///
    /// The recorded fields travel with the claim, so a criterion naming an
    /// approver is judged against the identity Compass observed rather than
    /// one the writer supplied (decision 0008).
    pub fn as_evidence(&self) -> Option<Evidence> {
        match (self.kind, &self.evidence_kind) {
            (EventKind::Evidence, Some(k)) => {
                Some(Evidence::new(k, self.attrs.clone()).with_recorded(self.recorded_fields()))
            }
            _ => None,
        }
    }

    /// The reserved fields of this event, as predicate-visible values.
    pub fn recorded_fields(&self) -> Vec<(String, String)> {
        let mut out = vec![
            ("id".to_string(), self.id.clone()),
            ("at".to_string(), self.at.to_string()),
            ("wall".to_string(), self.wall.to_string()),
            ("plan".to_string(), self.plan.clone()),
            ("step".to_string(), self.step.clone()),
            ("version".to_string(), self.version.clone()),
            ("actor".to_string(), self.actor.clone()),
            ("kind".to_string(), self.kind.as_str().to_string()),
        ];
        if let Some(k) = &self.evidence_kind {
            out.push(("evidence_kind".to_string(), k.clone()));
        }
        if let Some(n) = &self.note {
            out.push(("note".to_string(), n.clone()));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// The first attribute key that shadows a recorded field, if any.
    ///
    /// Shadowing is refused rather than sanitised: renaming or dropping the
    /// attribute would leave a plan authored against a criterion that could
    /// never have meant what it appeared to (decision 0008).
    pub fn shadowing_attr(attrs: &[(String, String)]) -> Option<&str> {
        attrs
            .iter()
            .map(|(k, _)| k.as_str())
            .find(|k| is_recorded_field(k))
    }

    pub fn render(&self) -> String {
        let mut doc = Doc::new();
        let mut e = Block::new("event", Some(self.id.clone()));
        e.set("at", self.at.to_string());
        e.set("wall", self.wall.to_string());
        e.set("plan", &self.plan);
        e.set("step", &self.step);
        e.set("version", &self.version);
        e.set("actor", &self.actor);
        e.set("kind", self.kind.as_str());
        e.set_opt("evidence_kind", self.evidence_kind.clone());
        e.set_opt("note", self.note.clone());
        doc.push(e);

        if !self.attrs.is_empty() {
            let mut a = Block::new("attrs", None);
            let mut sorted = self.attrs.clone();
            sorted.sort_by(|x, y| x.0.cmp(&y.0));
            for (k, v) in sorted {
                a.set(&k, v);
            }
            doc.push(a);
        }
        doc.render()
    }

    pub fn parse(text: &str) -> Result<Event, ParseError> {
        let doc = parse_block(text)?;
        let eb = doc
            .first("event")
            .ok_or_else(|| ParseError::new("no `@event` block"))?;

        let id = eb
            .arg
            .clone()
            .ok_or_else(|| ParseError::new("`@event` block has no id argument"))?;
        let kind_raw = eb.require("kind")?;
        let kind = EventKind::parse(kind_raw)
            .ok_or_else(|| ParseError::new(format!("unknown event kind `{kind_raw}`")))?;

        let at = eb
            .require("at")?
            .parse()
            .map_err(|_| ParseError::new("`at` must be a non-negative integer"))?;
        let wall = eb.get("wall").and_then(|w| w.parse().ok()).unwrap_or(0);

        let attrs = doc
            .first("attrs")
            .map(|a| {
                let mut v: Vec<(String, String)> = a.entries.clone();
                v.sort_by(|x, y| x.0.cmp(&y.0));
                v
            })
            .unwrap_or_default();

        // A file Compass wrote can never carry a shadowing attribute, so this
        // only fires on one authored elsewhere. Rejecting is what makes such a
        // file visible as a rejected file rather than quietly inert evidence.
        if let Some(k) = Event::shadowing_attr(&attrs) {
            return Err(ParseError::new(format!(
                "attribute `{k}` shadows the recorded field `{k}`; recorded fields are reserved \
                 and a predicate term naming one binds what Compass recorded (decision 0008)"
            )));
        }

        let evidence_kind = eb.get("evidence_kind").map(|s| s.to_string());
        if kind == EventKind::Evidence && evidence_kind.is_none() {
            return Err(ParseError::new(
                "an `evidence` event must name an `evidence_kind`",
            ));
        }

        Ok(Event {
            id,
            at,
            wall,
            plan: eb.require("plan")?.to_string(),
            step: eb.require("step")?.to_string(),
            version: eb.require("version")?.to_string(),
            actor: eb.require("actor")?.to_string(),
            kind,
            note: eb.get("note").map(|s| s.to_string()),
            evidence_kind,
            attrs,
        })
    }

    /// Storage filename: `<at>-<id>.<ext>`, zero-padded so a directory listing
    /// sorts in logical order.
    pub fn filename(&self) -> String {
        format!("{:012}-{}.{}", self.at, self.id, EXT)
    }
}

/// Current wall-clock seconds, or 0 if the clock is before the epoch.
pub fn now_wall() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(kind: EventKind) -> Event {
        Event {
            id: "ev_0123456789".into(),
            at: 3,
            wall: 1_700_000_000,
            plan: "pl_7000000000".into(),
            step: "st_A000000001".into(),
            version: "a".repeat(64),
            actor: "cos".into(),
            kind,
            note: None,
            evidence_kind: None,
            attrs: vec![],
        }
    }

    fn evidence_sample() -> Event {
        let mut e = sample(EventKind::Evidence);
        e.evidence_kind = Some("test".into());
        e.attrs = vec![
            ("status".into(), "pass".into()),
            ("name".into(), "parser::nested_groups".into()),
        ];
        e
    }

    #[test]
    fn round_trips_a_progress_event() {
        let mut e = sample(EventKind::Start);
        e.note = Some("picked this up".into());
        assert_eq!(Event::parse(&e.render()).unwrap(), e);
    }

    #[test]
    fn round_trips_an_evidence_event_with_attributes() {
        let e = evidence_sample();
        let parsed = Event::parse(&e.render()).unwrap();
        // Attributes come back sorted by key.
        assert_eq!(
            parsed.attrs,
            vec![
                ("name".to_string(), "parser::nested_groups".to_string()),
                ("status".to_string(), "pass".to_string()),
            ]
        );
        assert_eq!(parsed.kind, EventKind::Evidence);
        assert_eq!(parsed.evidence_kind.as_deref(), Some("test"));
    }

    #[test]
    fn round_trips_a_multiline_note() {
        let mut e = sample(EventKind::Handoff);
        e.note = Some("blocked on review\nsee thread".into());
        assert_eq!(Event::parse(&e.render()).unwrap().note, e.note);
    }

    #[test]
    fn only_evidence_events_become_evidence() {
        assert!(evidence_sample().as_evidence().is_some());
        for k in [
            EventKind::Start,
            EventKind::Update,
            EventKind::Handoff,
            EventKind::Done,
        ] {
            assert!(
                sample(k).as_evidence().is_none(),
                "{} must not feed acceptance",
                k.as_str()
            );
        }
    }

    #[test]
    fn evidence_carries_its_kind_and_attributes() {
        let ev = evidence_sample().as_evidence().unwrap();
        assert_eq!(ev.kind, "test");
        assert_eq!(ev.attrs.len(), 2);
    }

    #[test]
    fn rejects_evidence_without_an_evidence_kind() {
        let mut e = sample(EventKind::Evidence);
        e.evidence_kind = None;
        let err = Event::parse(&e.render()).unwrap_err();
        assert!(err.message.contains("evidence_kind"), "{err}");
    }

    #[test]
    fn rejects_an_unknown_kind() {
        let text = sample(EventKind::Start)
            .render()
            .replace("kind = start", "kind = invented");
        assert!(Event::parse(&text).is_err());
    }

    #[test]
    fn requires_the_mandatory_fields() {
        for field in ["plan", "step", "version", "actor", "kind", "at"] {
            let text: String = sample(EventKind::Start)
                .render()
                .lines()
                .filter(|l| !l.starts_with(&format!("{field} = ")))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(Event::parse(&text).is_err(), "missing `{field}` must fail");
        }
    }

    #[test]
    fn evidence_carries_the_recorded_actor() {
        let ev = evidence_sample().as_evidence().unwrap();
        assert_eq!(
            ev.recorded
                .iter()
                .find(|(k, _)| k == "actor")
                .map(|(_, v)| v.as_str()),
            Some("cos")
        );
    }

    #[test]
    fn a_file_carrying_a_shadowing_attribute_is_rejected() {
        // Compass will not write one, so this is a file authored elsewhere.
        // It is refused rather than read with the attribute quietly inert:
        // a rejected file is visible, an ignored attribute is not.
        let mut e = evidence_sample();
        e.attrs.push(("actor".into(), "editor".into()));
        let err = Event::parse(&e.render()).unwrap_err();
        assert!(err.message.contains("actor"), "{err}");
        assert!(err.message.contains("shadows"), "{err}");
    }

    #[test]
    fn every_rendered_event_key_is_a_recorded_field() {
        // The reserved set is the contract between writing, reading and
        // predicate binding. A key rendered into `@event` but missing here
        // would be a name a writer could reclaim.
        let mut e = evidence_sample();
        e.note = Some("a note".into());
        for line in e.render().lines() {
            let Some((key, _)) = line.split_once(" = ") else {
                continue;
            };
            let key = key.trim();
            // The `@attrs` block is claimed, not recorded.
            if e.attrs.iter().any(|(k, _)| k == key) {
                continue;
            }
            assert!(
                is_recorded_field(key),
                "`{key}` is written by Compass but not reserved"
            );
        }
    }

    #[test]
    fn filenames_sort_in_logical_order() {
        let mut a = sample(EventKind::Start);
        a.at = 2;
        let mut b = sample(EventKind::Start);
        b.at = 10;
        let mut names = vec![b.filename(), a.filename()];
        names.sort();
        assert!(names[0].contains("000000000002"), "{names:?}");
        assert!(names[1].contains("000000000010"), "{names:?}");
    }

    #[test]
    fn kind_strings_round_trip() {
        for k in [
            EventKind::Start,
            EventKind::Update,
            EventKind::Handoff,
            EventKind::Done,
            EventKind::Evidence,
        ] {
            assert_eq!(EventKind::parse(k.as_str()), Some(k));
        }
        assert_eq!(EventKind::parse("nope"), None);
    }
}
