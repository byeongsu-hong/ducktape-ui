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

/// A source-relative asset path as the generated `include_bytes!` argument.
/// Paths in a declaration are relative to their own `.ice` file, which the
/// generated Rust does not live beside.
fn embedded_asset_path(source_path: &str, relative: &str) -> String {
    let parent = Path::new(source_path)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    rust_string(&parent.join(relative).display().to_string())
}

/// Embeds a raw RGBA asset and restates its `width × height × 4` contract as
/// a compile-time assertion, so a file that stops matching its declared
/// dimensions fails the build rather than the pixel decode. `kind` names the
/// declaration in the assertion message.
fn rgba_embed_code(
    kind: &str,
    icon: &crate::lower::ResolvedWindowIcon,
    source_path: &str,
    binding: &str,
) -> String {
    format!(
        "{{ const {binding}: &[u8] = include_bytes!({}); const _: () = ::std::assert!({binding}.len() == {}, \"{kind} icon RGBA byte length does not match width × height × 4\"); {binding} }}",
        embedded_asset_path(source_path, &icon.path),
        icon.byte_len,
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
    settings
        .fonts
        .iter()
        .map(|font| {
            marked_setting(
                program,
                font.origin,
                format!(
                    ".font(include_bytes!({}).as_slice())",
                    embedded_asset_path(source_path, &font.path)
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

/// The status item's compile-time shape, plus every setting whose value is a
/// literal.
///
/// A literal is applied here, once, and left out of `__tray_sync` entirely: a
/// constant cannot become stale, and a tray of nothing but constants needs no
/// sync at all. The sync itself is appended when anything can still change, so
/// the item shows the program's first answer instead of an empty label until
/// the first message arrives.
///
/// Not gated on `cfg(not(test))`: `init` is portable and, off macOS, only
/// sizes the record that `expect tray label` reads.
pub(in crate::codegen) fn tray_init_code(
    program: &LoweredProgram,
    settings: &ResolvedAppSettings,
    source_path: &str,
) -> String {
    let Some(tray) = &settings.tray else {
        return String::new();
    };
    let icons = tray
        .icons
        .iter()
        .enumerate()
        .map(|(index, icon)| {
            format!(
                "::ui_lang_runtime::tray::TrayIcon {{ path: {}, rgba: {}, width: {}u32, height: {}u32 }},",
                rust_string(&icon.icon.path),
                rgba_embed_code(
                    "tray",
                    &icon.icon,
                    source_path,
                    &format!("__ICE_TRAY_RGBA_{index}")
                ),
                icon.icon.width,
                icon.icon.height,
            )
        })
        .collect::<String>();
    let rows = tray
        .menu
        .iter()
        .map(|row| match row {
            // A routed row is a command; an unrouted one is a figure to read,
            // and the platform draws one of those by disabling it.
            ResolvedTrayRow::Item { route, .. } => format!(
                "::ui_lang_runtime::tray::TrayRow::Item {{ command: {} }},",
                route.is_some()
            ),
            ResolvedTrayRow::Separator => "::ui_lang_runtime::tray::TrayRow::Separator,".to_owned(),
        })
        .collect::<String>();
    let mut literals = String::new();
    for (text, apply) in [(&tray.label, "set_label"), (&tray.tooltip, "set_tooltip")] {
        if let Some(ResolvedTrayText::Literal(value)) = text {
            literals.push_str(&format!(
                "::ui_lang_runtime::tray::{apply}({});\n",
                rust_string(value)
            ));
        }
    }
    for (index, row) in tray.menu.iter().enumerate() {
        if let ResolvedTrayRow::Item {
            text: ResolvedTrayText::Literal(value),
            ..
        } = row
        {
            literals.push_str(&format!(
                "::ui_lang_runtime::tray::set_item({index}usize, {});\n",
                rust_string(value)
            ));
        }
    }
    marked_setting(
        program,
        tray.origin,
        format!(
            "{{\nconst __ICE_TRAY_ICONS: &[::ui_lang_runtime::tray::TrayIcon] = &[{icons}];\nconst __ICE_TRAY_ROWS: &[::ui_lang_runtime::tray::TrayRow] = &[{rows}];\n::ui_lang_runtime::tray::init(::ui_lang_runtime::tray::TrayConfig {{ icons: __ICE_TRAY_ICONS, rows: __ICE_TRAY_ROWS, icon_template: {}, }});\n{literals}}}",
            tray.icon_template,
        ),
    )
}

/// The first sync, for `__boot` and every `__preset_N`.
///
/// It has to run after the boot task and after a preset's state overrides,
/// not beside `init`: syncing against `__state()` shows the tray whatever the
/// declared defaults were, which for a preset is never what the preset says.
/// Without it the item stays blank until the first message arrives.
pub(in crate::codegen) fn tray_boot_sync_code(settings: &ResolvedAppSettings) -> &'static str {
    match &settings.tray {
        Some(tray) if tray.reactive() => "state.__tray_sync();\n",
        _ => "",
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
        let value = format!(
            "icon: ::std::option::Option::Some(::iced::window::icon::from_rgba({}.to_vec(), {}, {}).expect(\"statically checked RGBA window icon\")),",
            rgba_embed_code("window", icon, source_path, "__ICE_RGBA"),
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
