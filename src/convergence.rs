//! Convergence — is this catalog complete? (CMP-R17, CMP.INT-R05, CMP.INT-R06)
//!
//! Convergence is a property of the **replication substrate**, not of the
//! catalog. No file states how many versions a plan should have, so a chain
//! missing its middle is indistinguishable from a chain that is simply
//! shorter. It therefore cannot be inferred from the data, and this module
//! never tries.
//!
//! Compass composes with fabric for replication (CMP.INT-R03). That
//! integration is **not built**, so there is nothing to ask. The honest report
//! is `unknown`, and the spec is explicit that an unknown convergence state is
//! reported as unknown and **never assumed converged** — serving a possibly
//! stale head as authoritative with no indication is a defect, not an
//! acceptable consequence of asynchrony.
//!
//! Every command reports this alongside its answer, so a reader can never
//! mistake a pre-convergence answer for an authoritative one.

/// What is known about whether the local catalog has everything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Convergence {
    /// The substrate cannot be asked. Never treat as converged.
    Unknown { reason: String },
}

impl Convergence {
    /// Establish convergence state before reporting authoritative state.
    ///
    /// No sync mechanism is wired up yet, so this always reports unknown with
    /// the reason. When fabric integration lands this grows the converged and
    /// receiving cases; the honest-unknown default stays as the fallback for a
    /// substrate that cannot answer.
    pub fn probe() -> Convergence {
        Convergence::Unknown {
            reason: "no sync configured".to_string(),
        }
    }

    /// Stable machine token.
    pub fn state(&self) -> &'static str {
        match self {
            Convergence::Unknown { .. } => "unknown",
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            Convergence::Unknown { reason } => reason,
        }
    }

    /// Whether the catalog is known to be complete. Never true while unknown.
    pub fn is_converged(&self) -> bool {
        false
    }

    /// One-line human rendering, e.g. `unknown (no sync configured)`.
    pub fn describe(&self) -> String {
        format!("{} ({})", self.state(), self.reason())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_never_assumed_converged() {
        assert!(!Convergence::probe().is_converged());
    }

    #[test]
    fn reports_unknown_with_a_reason() {
        let c = Convergence::probe();
        assert_eq!(c.state(), "unknown");
        assert_eq!(c.reason(), "no sync configured");
        assert_eq!(c.describe(), "unknown (no sync configured)");
    }
}
