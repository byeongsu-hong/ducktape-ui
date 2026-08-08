use super::*;

pub(in crate::codegen) fn generate_view(
    out: &mut String,
    program: &LoweredProgram,
    message: &str,
) -> Result<(), Error> {
    let daemon = program.settings().kind == ProgramKind::Daemon;
    let mounted = program
        .components()
        .iter()
        .filter(|component| component.storage == ComponentStorage::Mounted)
        .map(|component| component_state_field(&component.name))
        .collect::<Vec<_>>();
    let mut env = checked_state_env(program, "self");
    if daemon {
        env.insert(
            "window".into(),
            Binding {
                code: "window".into(),
                ty: Type::WindowId,
                local: true,
                state: None,
                owner: program
                    .expressions()
                    .daemon_window_local()
                    .map(BindingOwner::Local),
            },
        );
    }
    let root_scope = if mounted.is_empty() {
        rust_string(program.app_name())
    } else {
        "__ice_root_scope_ref".into()
    };
    let rendered_root = render_node_if_present(
        program.app_view(),
        program,
        message,
        &env,
        &root_scope,
        None,
    )?
    .unwrap_or_else(|| "::iced::widget::Column::new().into()".into());
    let window_arg = if daemon {
        ", window: ::iced::window::Id"
    } else {
        ""
    };
    let callback_value = if daemon { "window" } else { "" };
    let palette = format!(
        "let __ice_palette = self.__palette({callback_value}); let __ice_app_theme = Self::__app_theme(__ice_palette);"
    );
    if mounted.is_empty() && daemon {
        writeln!(
            out,
            "pub(super) fn __view(&self{window_arg}) -> __IceElement<'_, {message}> {{ {palette} let __ice_root: __IceElement<'_, {message}> = {rendered_root}; ::ui_lang_runtime::dev::ready(__ice_root) }}"
        )
        .unwrap();
    } else if mounted.is_empty() {
        writeln!(
            out,
            "pub(super) fn __view(&self{window_arg}) -> __IceElement<'_, {message}> {{ {palette} let __ice_content: __IceElement<'_, {message}> = {rendered_root}; let __ice_root: __IceElement<'_, {message}> = ::ui_lang_runtime::navigation(__ice_content, {message}::__AccessibilityFocusNext, {message}::__AccessibilityFocusPrevious).into(); ::ui_lang_runtime::dev::ready(__ice_root) }}"
        )
        .unwrap();
    } else {
        let root_scope_code = if daemon {
            format!(
                "format!(\"{{}}/{{:?}}\", {}, window)",
                rust_string(program.app_name())
            )
        } else {
            format!("{}.to_owned()", rust_string(program.app_name()))
        };
        let begin = mounted
            .iter()
            .map(|field| format!("self.{field}.begin_render();"))
            .collect::<String>();
        let finish = mounted
            .iter()
            .map(|field| format!("self.{field}.finish_render(__ice_root_scope_ref);"))
            .collect::<String>();
        let result = if daemon {
            "__ice_content".into()
        } else {
            format!(
                "::ui_lang_runtime::navigation(__ice_content, {message}::__AccessibilityFocusNext, {message}::__AccessibilityFocusPrevious).into()"
            )
        };
        writeln!(
            out,
            "pub(super) fn __view(&self{window_arg}) -> __IceElement<'_, {message}> {{ {palette} let __ice_root_scope = {root_scope_code}; let __ice_root_scope_ref = __ice_root_scope.as_str(); {begin} let __ice_content: __IceElement<'_, {message}> = {rendered_root}; {finish} let __ice_root: __IceElement<'_, {message}> = {result}; ::ui_lang_runtime::dev::ready(__ice_root) }}"
        )
        .unwrap();
    }
    Ok(())
}
