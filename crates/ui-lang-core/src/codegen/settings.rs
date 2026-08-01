use super::*;

fn marked_setting(program: &LoweredProgram, origin: OriginId, code: String) -> String {
    format!(
        "\n{}\n{code}\n{SOURCE_MARKER_END}\n",
        source_marker_for_origin(program, origin)
    )
}

fn marked_field(
    program: &LoweredProgram,
    settings: &ResolvedAppSettings,
    name: &str,
    code: String,
) -> String {
    marked_setting(
        program,
        settings
            .field_origins
            .get(name)
            .copied()
            .unwrap_or(settings.origin),
        code,
    )
}

fn marked_window_field(
    program: &LoweredProgram,
    settings: &ResolvedWindowSettings,
    name: &str,
    code: String,
) -> String {
    marked_setting(
        program,
        settings
            .field_origins
            .get(name)
            .copied()
            .unwrap_or(settings.origin),
        code,
    )
}

fn marked_platform_field(
    program: &LoweredProgram,
    origins: &HashMap<String, OriginId>,
    fallback: OriginId,
    name: &str,
    code: String,
) -> String {
    marked_setting(
        program,
        origins.get(name).copied().unwrap_or(fallback),
        code,
    )
}

pub(in crate::codegen) fn has_animations(program: &LoweredProgram) -> bool {
    program
        .app_states()
        .iter()
        .any(|state| matches!(state.ty, Type::Animation(_)))
}

pub(in crate::codegen) fn font_assets_code(
    program: &LoweredProgram,
    settings: &ResolvedAppSettings,
    source_path: &str,
) -> String {
    let parent = Path::new(source_path)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    settings
        .fonts
        .iter()
        .map(|font| {
            marked_setting(
                program,
                font.origin,
                format!(
                    ".font(include_bytes!({}).as_slice())",
                    rust_string(&parent.join(&font.path).display().to_string())
                ),
            )
        })
        .collect()
}

pub(in crate::codegen) fn app_settings_code(
    program: &LoweredProgram,
    settings: &ResolvedAppSettings,
) -> String {
    let mut fields = String::new();
    if let Some(id) = &settings.id {
        let value = format!(
            "id: ::std::option::Option::Some({}.to_owned()),",
            rust_string(id)
        );
        fields.push_str(&marked_field(program, settings, "id", value));
    }
    if let Some(size) = settings.default_text_size {
        fields.push_str(&marked_field(
            program,
            settings,
            "text-size",
            format!("default_text_size: ::iced::Pixels({size} as f32),"),
        ));
    }
    if let Some(value) = settings.antialiasing {
        fields.push_str(&marked_field(
            program,
            settings,
            "antialiasing",
            format!("antialiasing: {value},"),
        ));
    }
    if let Some(value) = settings.vsync {
        fields.push_str(&marked_field(
            program,
            settings,
            "vsync",
            format!("vsync: {value},"),
        ));
    }
    if fields.is_empty() {
        String::new()
    } else {
        format!(".settings(::iced::Settings {{ {fields} ..::std::default::Default::default() }})")
    }
}

pub(in crate::codegen) fn window_settings_code(
    program: &LoweredProgram,
    settings: &ResolvedWindowSettings,
    source_path: &str,
) -> String {
    let settings = window_settings_value_code(program, settings, source_path);
    format!(
        ".window({{ let mut __window = {settings}; #[cfg(target_os = \"windows\")] {{ __window.visible = false; __window.maximized = false; __window.fullscreen = false; }} __window }})"
    )
}

pub(in crate::codegen) fn generate_named_windows(
    out: &mut String,
    program: &LoweredProgram,
    settings: &ResolvedAppSettings,
    source_path: &str,
) {
    for window in &settings.named_windows {
        let index = window.id.0;
        writeln!(
            out,
            "{}\nfn __window_{index}() -> ::iced::window::Settings {{ {} }}\n{SOURCE_MARKER_END}",
            source_marker_for_origin(program, window.origin),
            window_settings_value_code(program, &window.settings, source_path)
        )
        .unwrap();
    }
}

pub(in crate::codegen) fn window_settings_value_code(
    program: &LoweredProgram,
    settings: &ResolvedWindowSettings,
    source_path: &str,
) -> String {
    let mut fields = String::new();
    let size =
        |(width, height): (f64, f64)| format!("::iced::Size::new({width} as f32, {height} as f32)");
    if let Some(value) = settings.size {
        fields.push_str(&marked_window_field(
            program,
            settings,
            "size",
            format!("size: {},", size(value)),
        ));
    }
    for (name, value) in [
        ("maximized", settings.maximized),
        ("fullscreen", settings.fullscreen),
        ("visible", settings.visible),
        ("resizable", settings.resizable),
        ("closeable", settings.closeable),
        ("minimizable", settings.minimizable),
        ("decorations", settings.decorations),
        ("transparent", settings.transparent),
        ("blur", settings.blur),
        ("exit_on_close_request", settings.exit_on_close_request),
    ] {
        if let Some(value) = value {
            let source_name = match name {
                "exit_on_close_request" => "exit-on-close",
                name => name,
            };
            fields.push_str(&marked_window_field(
                program,
                settings,
                source_name,
                format!("{name}: {value},"),
            ));
        }
    }
    if let Some(position) = settings.position {
        let position = match position {
            ResolvedWindowPosition::Default => "::iced::window::Position::Default".into(),
            ResolvedWindowPosition::Centered => "::iced::window::Position::Centered".into(),
            ResolvedWindowPosition::Specific(x, y) => format!(
                "::iced::window::Position::Specific(::iced::Point::new({x} as f32, {y} as f32))"
            ),
        };
        fields.push_str(&marked_window_field(
            program,
            settings,
            "position",
            format!("position: {position},"),
        ));
    }
    if let Some(value) = settings.min_size {
        fields.push_str(&marked_window_field(
            program,
            settings,
            "min-size",
            format!("min_size: ::std::option::Option::Some({}),", size(value)),
        ));
    }
    if let Some(value) = settings.max_size {
        fields.push_str(&marked_window_field(
            program,
            settings,
            "max-size",
            format!("max_size: ::std::option::Option::Some({}),", size(value)),
        ));
    }
    if let Some(level) = settings.level {
        let level = match level {
            ResolvedWindowLevel::Normal => "Normal",
            ResolvedWindowLevel::AlwaysOnBottom => "AlwaysOnBottom",
            ResolvedWindowLevel::AlwaysOnTop => "AlwaysOnTop",
        };
        fields.push_str(&marked_window_field(
            program,
            settings,
            "level",
            format!("level: ::iced::window::Level::{level},"),
        ));
    }
    if let Some(icon) = &settings.icon {
        let parent = Path::new(source_path)
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let path = parent.join(&icon.path).display().to_string();
        let value = format!(
            "icon: ::std::option::Option::Some({{ const __ICE_RGBA: &[u8] = include_bytes!({}); const _: () = ::std::assert!(__ICE_RGBA.len() == {}, \"window icon RGBA byte length does not match width × height × 4\"); ::iced::window::icon::from_rgba(__ICE_RGBA.to_vec(), {}, {}).expect(\"statically checked RGBA window icon\") }}),",
            rust_string(&path),
            icon.byte_len,
            icon.width,
            icon.height
        );
        fields.push_str(&marked_setting(program, icon.origin, value));
    }
    if settings.linux.is_some()
        || settings.windows.is_some()
        || settings.macos.is_some()
        || settings.wasm.is_some()
    {
        write!(
            fields,
            "platform_specific: {},",
            window_platform_code(program, settings)
        )
        .unwrap();
    }
    format!("::iced::window::Settings {{ {fields} ..::std::default::Default::default() }}")
}

pub(in crate::codegen) fn window_platform_code(
    program: &LoweredProgram,
    settings: &ResolvedWindowSettings,
) -> String {
    let mut linux = String::new();
    if let Some(settings) = &settings.linux {
        if let Some(value) = &settings.application_id {
            linux.push_str(&marked_platform_field(
                program,
                &settings.field_origins,
                settings.origin,
                "app-id",
                format!(
                    "__platform.application_id = {}.to_owned();",
                    rust_string(value)
                ),
            ));
        }
        if let Some(value) = settings.override_redirect {
            linux.push_str(&marked_platform_field(
                program,
                &settings.field_origins,
                settings.origin,
                "override-redirect",
                format!("__platform.override_redirect = {value};"),
            ));
        }
    }

    let mut windows = String::new();
    if let Some(settings) = &settings.windows {
        for (name, value) in [
            ("drag_and_drop", settings.drag_and_drop),
            ("skip_taskbar", settings.skip_taskbar),
            ("undecorated_shadow", settings.undecorated_shadow),
        ] {
            if let Some(value) = value {
                windows.push_str(&marked_platform_field(
                    program,
                    &settings.field_origins,
                    settings.origin,
                    &name.replace('_', "-"),
                    format!("__platform.{name} = {value};"),
                ));
            }
        }
        if let Some(value) = settings.corner {
            let value = match value {
                ResolvedWindowCorner::Default => "Default",
                ResolvedWindowCorner::DoNotRound => "DoNotRound",
                ResolvedWindowCorner::Round => "Round",
                ResolvedWindowCorner::RoundSmall => "RoundSmall",
            };
            windows.push_str(&marked_platform_field(
                program,
                &settings.field_origins,
                settings.origin,
                "corner",
                format!(
                    "__platform.corner_preference = ::iced::window::settings::platform::CornerPreference::{value};"
                ),
            ));
        }
    }

    let mut macos = String::new();
    if let Some(settings) = &settings.macos {
        for (name, value) in [
            ("title_hidden", settings.title_hidden),
            ("titlebar_transparent", settings.titlebar_transparent),
            ("fullsize_content_view", settings.fullsize_content_view),
        ] {
            if let Some(value) = value {
                macos.push_str(&marked_platform_field(
                    program,
                    &settings.field_origins,
                    settings.origin,
                    &name.replace('_', "-"),
                    format!("__platform.{name} = {value};"),
                ));
            }
        }
    }

    let mut wasm = String::new();
    if let Some(Some(target)) = settings
        .wasm
        .as_ref()
        .and_then(|settings| settings.target.as_ref())
    {
        let wasm_settings = settings.wasm.as_ref().unwrap();
        wasm.push_str(&marked_platform_field(
            program,
            &wasm_settings.field_origins,
            wasm_settings.origin,
            "target",
            format!(
                "__platform.target = ::std::option::Option::Some({}.to_owned());",
                rust_string(target)
            ),
        ));
    } else if settings
        .wasm
        .as_ref()
        .is_some_and(|settings| settings.target == Some(None))
    {
        let wasm_settings = settings.wasm.as_ref().unwrap();
        wasm.push_str(&marked_platform_field(
            program,
            &wasm_settings.field_origins,
            wasm_settings.origin,
            "target",
            "__platform.target = ::std::option::Option::None;".into(),
        ));
    }

    format!(
        "{{ #[cfg(target_os = \"linux\")] {{ #[allow(unused_mut)] let mut __platform: ::iced::window::settings::PlatformSpecific = ::std::default::Default::default(); {linux} __platform }} #[cfg(target_os = \"windows\")] {{ #[allow(unused_mut)] let mut __platform: ::iced::window::settings::PlatformSpecific = ::std::default::Default::default(); {windows} __platform }} #[cfg(target_os = \"macos\")] {{ #[allow(unused_mut)] let mut __platform: ::iced::window::settings::PlatformSpecific = ::std::default::Default::default(); {macos} __platform }} #[cfg(target_arch = \"wasm32\")] {{ #[allow(unused_mut)] let mut __platform: ::iced::window::settings::PlatformSpecific = ::std::default::Default::default(); {wasm} __platform }} #[cfg(not(any(target_os = \"linux\", target_os = \"windows\", target_os = \"macos\", target_arch = \"wasm32\")))] {{ ::std::default::Default::default() }} }}"
    )
}
