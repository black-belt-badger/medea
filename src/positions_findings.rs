//! Conservative facts for the fetched positions snapshot.
//! A new side enum is a schema observation, not an investment signal.

use crate::diagnostics::{Finding, Severity};
use crate::positions::{Position, PositionSide};

pub fn positions_findings(positions: &[Position]) -> Vec<Finding> {
    let mut findings = Vec::new();
    if positions.is_empty() {
        findings.push(Finding {
            code: "POSITIONS_EMPTY",
            severity: Severity::Info,
            path: "positions",
            message: "Alpaca returned no open positions.",
            explanation: Some(
                "This describes the fetched paper-account snapshot, not historical holdings.",
            ),
        });
    }
    if positions
        .iter()
        .any(|position| matches!(&position.side, PositionSide::Other(_)))
    {
        findings.push(Finding {
            code: "POSITION_SIDE_UNKNOWN",
            severity: Severity::Warning,
            path: "positions[].side",
            message: "Alpaca returned a position side not recognized by this client.",
            explanation: Some(
                "The raw value is preserved in the JSON response; no meaning is inferred.",
            ),
        });
    }
    findings
}
