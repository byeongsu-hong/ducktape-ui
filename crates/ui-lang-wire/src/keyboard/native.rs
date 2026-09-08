use super::*;
use iced_core::keyboard as native;

impl From<native::Key> for Key {
    fn from(value: native::Key) -> Self {
        match value {
            native::Key::Named(value) => Self::Named(value.into()),
            native::Key::Character(value) => Self::Character(value.to_string()),
            native::Key::Unidentified => Self::Unidentified,
        }
    }
}
impl From<Key> for native::Key {
    fn from(value: Key) -> Self {
        match value {
            Key::Named(value) => Self::Named(value.into()),
            Key::Character(value) => Self::Character(value.into()),
            Key::Unidentified => Self::Unidentified,
        }
    }
}
impl From<native::Modifiers> for Modifiers {
    fn from(value: native::Modifiers) -> Self {
        Self {
            shift: value.shift(),
            control: value.control(),
            alt: value.alt(),
            logo: value.logo(),
        }
    }
}
impl From<Modifiers> for native::Modifiers {
    fn from(value: Modifiers) -> Self {
        let mut out = Self::empty();
        out.set(Self::SHIFT, value.shift);
        out.set(Self::CTRL, value.control);
        out.set(Self::ALT, value.alt);
        out.set(Self::LOGO, value.logo);
        out
    }
}
impl From<native::key::NativeCode> for NativeCode {
    fn from(value: native::key::NativeCode) -> Self {
        match value {
            native::key::NativeCode::Unidentified => Self::Unidentified,
            native::key::NativeCode::Android(value) => Self::Android(value),
            native::key::NativeCode::MacOS(value) => Self::MacOS(value),
            native::key::NativeCode::Windows(value) => Self::Windows(value),
            native::key::NativeCode::Xkb(value) => Self::Xkb(value),
        }
    }
}
impl From<NativeCode> for native::key::NativeCode {
    fn from(value: NativeCode) -> Self {
        match value {
            NativeCode::Unidentified => Self::Unidentified,
            NativeCode::Android(value) => Self::Android(value),
            NativeCode::MacOS(value) => Self::MacOS(value),
            NativeCode::Windows(value) => Self::Windows(value),
            NativeCode::Xkb(value) => Self::Xkb(value),
        }
    }
}
impl From<native::key::Physical> for Physical {
    fn from(value: native::key::Physical) -> Self {
        match value {
            native::key::Physical::Code(value) => Self::Code(value.into()),
            native::key::Physical::Unidentified(value) => Self::Unidentified(value.into()),
        }
    }
}
impl From<Physical> for native::key::Physical {
    fn from(value: Physical) -> Self {
        match value {
            Physical::Code(value) => Self::Code(value.into()),
            Physical::Unidentified(value) => Self::Unidentified(value.into()),
        }
    }
}
impl From<native::Event> for Event {
    fn from(value: native::Event) -> Self {
        match value {
            native::Event::KeyPressed {
                key,
                modified_key,
                physical_key,
                location,
                modifiers,
                text,
                repeat,
            } => Self::Press {
                state: KeyState {
                    key: key.into(),
                    modified_key: modified_key.into(),
                    physical_key: physical_key.into(),
                    location: location.into(),
                    modifiers: modifiers.into(),
                },
                text: text.map(|value| value.to_string()),
                repeat,
            },
            native::Event::KeyReleased {
                key,
                modified_key,
                physical_key,
                location,
                modifiers,
            } => Self::Release(KeyState {
                key: key.into(),
                modified_key: modified_key.into(),
                physical_key: physical_key.into(),
                location: location.into(),
                modifiers: modifiers.into(),
            }),
            native::Event::ModifiersChanged(value) => Self::Modifiers(value.into()),
        }
    }
}
impl From<Event> for native::Event {
    fn from(value: Event) -> Self {
        match value {
            Event::Press {
                state,
                text,
                repeat,
            } => Self::KeyPressed {
                key: state.key.into(),
                modified_key: state.modified_key.into(),
                physical_key: state.physical_key.into(),
                location: state.location.into(),
                modifiers: state.modifiers.into(),
                text: text.map(Into::into),
                repeat,
            },
            Event::Release(state) => Self::KeyReleased {
                key: state.key.into(),
                modified_key: state.modified_key.into(),
                physical_key: state.physical_key.into(),
                location: state.location.into(),
                modifiers: state.modifiers.into(),
            },
            Event::Modifiers(value) => Self::ModifiersChanged(value.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keyboard_codec_preserves_native_metadata() {
        for modifiers in 0..16 {
            let mods = Modifiers {
                shift: modifiers & 1 != 0,
                control: modifiers & 2 != 0,
                alt: modifiers & 4 != 0,
                logo: modifiers & 8 != 0,
            };
            for physical in [
                Physical::Code(Code::KeyQ),
                Physical::Code(Code::F35),
                Physical::Unidentified(NativeCode::Unidentified),
                Physical::Unidentified(NativeCode::MacOS(u16::MAX)),
                Physical::Unidentified(NativeCode::Windows(23)),
                Physical::Unidentified(NativeCode::Android(u32::MAX)),
                Physical::Unidentified(NativeCode::Xkb(24)),
            ] {
                let state = KeyState {
                    key: Key::Character("å".into()),
                    modified_key: Key::Character("Å".into()),
                    physical_key: physical,
                    location: Location::Right,
                    modifiers: mods,
                };
                for event in [
                    Event::Press {
                        state: state.clone(),
                        text: Some("Å".into()),
                        repeat: true,
                    },
                    Event::Release(state),
                    Event::Modifiers(mods),
                ] {
                    let expected = event.clone();
                    let native: native::Event = event.clone().into();
                    let wire = crate::Event::Keyboard {
                        event,
                        captured: true,
                    };
                    let decoded: crate::Event = crate::decode(&crate::encode(&wire)).unwrap();
                    assert_eq!(decoded, wire);
                    let crate::Event::Keyboard { event, .. } = decoded else {
                        unreachable!()
                    };
                    assert_eq!(native::Event::from(event), native);
                    assert_eq!(Event::from(native), expected);
                }
            }
        }
    }
}
