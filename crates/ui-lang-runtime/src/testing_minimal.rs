//! Source metadata retained by renderer-only consumers.
//!
//! The native headless Ice test driver is available through the default
//! `test-runtime` feature. Component crates that only render widgets can turn
//! that feature off, including on `wasm32`.

use std::fmt;

/// Source location attached to a generated test operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub path: &'static str,
    pub line: usize,
    pub column: usize,
    pub statement: &'static str,
}

impl Location {
    pub const fn new(
        path: &'static str,
        line: usize,
        column: usize,
        statement: &'static str,
    ) -> Self {
        Self {
            path,
            line,
            column,
            statement,
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}:{}", self.path, self.line, self.column)
    }
}

pub(crate) const fn current_render_source() -> Option<Location> {
    None
}
