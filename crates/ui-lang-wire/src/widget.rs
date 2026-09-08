//! Commands applied only to the requesting guest's mounted widget tree.
use serde::{Deserialize, Serialize};

/// Payload of `host.widget`. Mutation requests return an encoded unit;
/// `Focused` returns an encoded bool. Targets are the tree's qualified keys.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WidgetCommand {
    FocusPrevious,
    FocusNext,
    Focus {
        target: String,
    },
    Focused {
        target: String,
    },
    CursorFront {
        target: String,
    },
    CursorEnd {
        target: String,
    },
    Cursor {
        target: String,
        position: u32,
    },
    SelectAll {
        target: String,
    },
    Select {
        target: String,
        start: u32,
        end: u32,
    },
    Snap {
        target: String,
        x: f32,
        y: f32,
    },
    SnapEnd {
        target: String,
    },
    ScrollTo {
        target: String,
        x: f32,
        y: f32,
    },
    ScrollToKey {
        target: String,
        key: u64,
    },
    ScrollBy {
        target: String,
        x: f32,
        y: f32,
    },
}

impl WidgetCommand {
    /// Bound a decoded request without changing its target identity.
    pub fn validate(&mut self) -> Result<(), String> {
        let target = match self {
            Self::FocusPrevious | Self::FocusNext => return Ok(()),
            Self::Focus { target }
            | Self::Focused { target }
            | Self::CursorFront { target }
            | Self::CursorEnd { target }
            | Self::Cursor { target, .. }
            | Self::SelectAll { target }
            | Self::Select { target, .. }
            | Self::Snap { target, .. }
            | Self::SnapEnd { target }
            | Self::ScrollTo { target, .. }
            | Self::ScrollBy { target, .. }
            | Self::ScrollToKey { target, .. } => target,
        };
        if target.len() > crate::MAX_STRING_BYTES {
            return Err("widget target exceeds the key byte limit".into());
        }
        match self {
            Self::Snap { x, y, .. } | Self::ScrollTo { x, y, .. } | Self::ScrollBy { x, y, .. }
                if !x.is_finite() || !y.is_finite() =>
            {
                return Err("widget offsets must be finite".into());
            }
            _ => {}
        }
        if let Self::Snap { x, y, .. } = self {
            *x = x.clamp(0.0, 1.0);
            *y = y.clamp(0.0, 1.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_targets_are_rejected_whole_instead_of_redirected() {
        let key = "가".repeat(crate::MAX_STRING_BYTES / 3 + 1);
        let mut command = WidgetCommand::Focus {
            target: key.clone(),
        };
        assert!(
            command.validate().is_err(),
            "oversized target must be refused"
        );
        assert_eq!(command, WidgetCommand::Focus { target: key });
        let expected = WidgetCommand::Focus {
            target: "App/chat(room)/draft".into(),
        };
        let mut decoded: WidgetCommand = crate::decode(&crate::encode(&expected)).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn widget_offsets_reject_nonfinite_values_and_preserve_scroll_direction() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for mut command in [
                WidgetCommand::Snap {
                    target: "list".into(),
                    x: value,
                    y: 0.0,
                },
                WidgetCommand::ScrollTo {
                    target: "list".into(),
                    x: 0.0,
                    y: value,
                },
                WidgetCommand::ScrollBy {
                    target: "list".into(),
                    x: value,
                    y: 0.0,
                },
            ] {
                assert!(
                    command.validate().is_err(),
                    "nonfinite offset must be refused"
                );
            }
        }
        let mut snap = WidgetCommand::Snap {
            target: "list".into(),
            x: -1.0,
            y: 2.0,
        };
        snap.validate().unwrap();
        assert_eq!(
            snap,
            WidgetCommand::Snap {
                target: "list".into(),
                x: 0.0,
                y: 1.0
            }
        );
        let expected = WidgetCommand::ScrollBy {
            target: "list".into(),
            x: -24.0,
            y: 10.0,
        };
        let mut command = expected.clone();
        command.validate().unwrap();
        assert_eq!(command, expected);
    }
}
