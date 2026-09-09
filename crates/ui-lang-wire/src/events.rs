//! Opt-in window lifecycle and IME observations, never native input commands.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interest {
    pub focus: bool,
    pub close: bool,
    pub files: bool,
    pub input_method: bool,
}

impl Interest {
    pub const ALL: Self = Self {
        focus: true,
        close: true,
        files: true,
        input_method: true,
    };

    pub fn include(&mut self, other: Self) {
        self.focus |= other.focus;
        self.close |= other.close;
        self.files |= other.files;
        self.input_method |= other.input_method;
    }

    pub fn accepts(self, event: &Event) -> bool {
        match event {
            Event::Window(Window::Focused | Window::Unfocused) => self.focus,
            Event::Window(Window::CloseRequested | Window::Closed) => self.close,
            Event::Window(
                Window::FileHovered(_) | Window::FileDropped(_) | Window::FilesHoveredLeft,
            ) => self.files,
            Event::InputMethod(_) => self.input_method,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Window {
    Focused,
    Unfocused,
    CloseRequested,
    Closed,
    FileHovered(#[serde(deserialize_with = "text")] String),
    FileDropped(#[serde(deserialize_with = "text")] String),
    FilesHoveredLeft,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputMethod {
    Opened,
    Preedit {
        #[serde(deserialize_with = "text")]
        content: String,
        /// UTF-8 byte offsets into the preedit, not the editor document.
        selection: Option<(u32, u32)>,
    },
    Commit(#[serde(deserialize_with = "text")] String),
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    Window(Window),
    InputMethod(InputMethod),
}

impl Event {
    pub fn validate(&self) -> Result<(), &'static str> {
        let text = match self {
            Self::Window(Window::FileHovered(path) | Window::FileDropped(path)) => path,
            Self::InputMethod(InputMethod::Commit(content)) => content,
            Self::InputMethod(InputMethod::Preedit { content, selection }) => {
                if let Some((start, end)) = selection
                    && (start > end
                        || !content.is_char_boundary(*start as usize)
                        || !content.is_char_boundary(*end as usize))
                {
                    return Err("invalid input-method selection");
                }
                content
            }
            _ => return Ok(()),
        };
        if text.len() > crate::MAX_STRING_BYTES {
            return Err("window/input-method observation text limit");
        }
        Ok(())
    }

    #[cfg(feature = "iced")]
    pub fn from_native(event: &iced_core::Event) -> Result<Option<Self>, &'static str> {
        use iced_core::{input_method::Event as I, window::Event as W};
        let event = match event {
            iced_core::Event::Window(event) => Self::Window(match event {
                W::Focused => Window::Focused,
                W::Unfocused => Window::Unfocused,
                W::CloseRequested => Window::CloseRequested,
                W::Closed => Window::Closed,
                W::FileHovered(path) => Window::FileHovered(
                    path.to_str()
                        .ok_or("window file path is not UTF-8")?
                        .to_owned(),
                ),
                W::FileDropped(path) => Window::FileDropped(
                    path.to_str()
                        .ok_or("window file path is not UTF-8")?
                        .to_owned(),
                ),
                W::FilesHoveredLeft => Window::FilesHoveredLeft,
                _ => return Ok(None),
            }),
            iced_core::Event::InputMethod(event) => Self::InputMethod(match event {
                I::Opened => InputMethod::Opened,
                I::Closed => InputMethod::Closed,
                I::Commit(content) => InputMethod::Commit(content.clone()),
                I::Preedit(content, selection) => InputMethod::Preedit {
                    content: content.clone(),
                    selection: selection
                        .as_ref()
                        .map(|range| {
                            Ok((
                                u32::try_from(range.start)
                                    .map_err(|_| "input-method offset limit")?,
                                u32::try_from(range.end)
                                    .map_err(|_| "input-method offset limit")?,
                            ))
                        })
                        .transpose()?,
                },
            }),
            _ => return Ok(None),
        };
        event.validate()?;
        Ok(Some(event))
    }
}

#[cfg(feature = "iced")]
impl From<Event> for iced_core::Event {
    fn from(event: Event) -> Self {
        use iced_core::{input_method::Event as I, window::Event as W};
        match event {
            Event::Window(event) => Self::Window(match event {
                Window::Focused => W::Focused,
                Window::Unfocused => W::Unfocused,
                Window::CloseRequested => W::CloseRequested,
                Window::Closed => W::Closed,
                Window::FileHovered(path) => W::FileHovered(path.into()),
                Window::FileDropped(path) => W::FileDropped(path.into()),
                Window::FilesHoveredLeft => W::FilesHoveredLeft,
            }),
            Event::InputMethod(event) => Self::InputMethod(match event {
                InputMethod::Opened => I::Opened,
                InputMethod::Closed => I::Closed,
                InputMethod::Commit(content) => I::Commit(content),
                InputMethod::Preedit { content, selection } => I::Preedit(
                    content,
                    selection.map(|(start, end)| start as usize..end as usize),
                ),
            }),
        }
    }
}

fn text<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    struct Text;
    impl<'de> serde::de::Visitor<'de> for Text {
        type Value = String;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("bounded window/input-method text")
        }
        fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<String, E> {
            if value.len() > crate::MAX_STRING_BYTES {
                return Err(E::custom("window/input-method observation text limit"));
            }
            Ok(value.to_owned())
        }
    }
    deserializer.deserialize_str(Text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(unix, feature = "iced"))]
    #[test]
    fn non_utf8_native_file_paths_are_refused_without_replacement_characters() {
        use std::os::unix::ffi::OsStringExt;
        let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(b"/tmp/bad\xff".to_vec()));
        for event in [
            iced_core::window::Event::FileHovered(path.clone()),
            iced_core::window::Event::FileDropped(path),
        ] {
            assert_eq!(
                Event::from_native(&iced_core::Event::Window(event)),
                Err("window file path is not UTF-8")
            );
        }
    }

    #[test]
    fn interests_do_not_subscribe_focus_listeners_to_dropped_paths() {
        let mut interest = Interest {
            focus: true,
            ..Default::default()
        };
        assert!(interest.accepts(&Event::Window(Window::Focused)));
        assert!(!interest.accepts(&Event::Window(Window::FileDropped("private.txt".into()))));
        interest.include(Interest {
            files: true,
            ..Default::default()
        });
        assert!(interest.accepts(&Event::Window(Window::FileDropped("private.txt".into()))));
        assert!(!interest.accepts(&Event::InputMethod(InputMethod::Opened)));
    }

    #[test]
    fn invalid_preedit_ranges_and_oversize_text_are_refused_without_truncation() {
        for selection in [Some((1, 3)), Some((6, 0)), Some((0, 7))] {
            assert!(
                Event::InputMethod(InputMethod::Preedit {
                    content: "한글".into(),
                    selection
                })
                .validate()
                .is_err()
            );
        }
        let event = Event::InputMethod(InputMethod::Preedit {
            content: "한글".into(),
            selection: Some((0, 6)),
        });
        assert!(event.validate().is_ok());
        assert_eq!(
            crate::decode::<Event>(&crate::encode(&event)).unwrap(),
            event
        );
        let event = Event::Window(Window::FileDropped("x".repeat(crate::MAX_STRING_BYTES + 1)));
        assert!(event.validate().is_err());
        assert!(crate::decode::<Event>(&crate::encode(&event)).is_err());
    }
}
