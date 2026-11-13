//! Pure diagnostics shared conceptually with Kirke, not terminal rendering.
//! These types contain stable, non-sensitive facts, never provider response bodies.

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[allow(dead_code)]
impl Severity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub code: &'static str,
    pub severity: Severity,
    pub path: &'static str,
    pub message: &'static str,
    pub explanation: Option<&'static str>,
}
