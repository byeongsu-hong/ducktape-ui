use super::*;

pub(in crate::codegen) fn generate_view(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
    let mounted = document
        .components
        .iter()
        .filter(|component| {
            component.lifetime == ComponentLifetime::Mounted
                && (!component.states.is_empty() || !component.handlers.is_empty())
        })
        .map(|component| component_state_field(&component.name))
        .collect::<Vec<_>>();
    let mut env = state_env(document, "self");
    if document.daemon {
        env.insert(
            "window".into(),
            Binding {
                code: "window".into(),
                ty: Type::WindowId,
                local: true,
                state: None,
            },
        );
    }
    let root_scope = if mounted.is_empty() {
        rust_string(&document.app)
    } else {
        "__ice_root_scope.as_str()".into()
    };
    let rendered_root =
        render_node_if_present(&document.view, document, message, &env, &root_scope, None)?
            .unwrap_or_else(|| "::iced::widget::Column::new().into()".into());
    let window_arg = if document.daemon {
        ", window: ::iced::window::Id"
    } else {
        ""
    };
    let callback_value = if document.daemon { "window" } else { "" };
    let palette = format!(
        "let __ice_palette = self.__palette({callback_value}); let __ice_app_theme = Self::__app_theme(__ice_palette);"
    );
    if mounted.is_empty() && document.daemon {
        writeln!(
            out,
            "fn __view(&self{window_arg}) -> __IceElement<'_, {message}> {{ {palette} {rendered_root} }}"
        )
        .unwrap();
    } else if mounted.is_empty() {
        writeln!(
            out,
            "fn __view(&self{window_arg}) -> __IceElement<'_, {message}> {{ {palette} let __ice_content: __IceElement<'_, {message}> = {rendered_root}; ::ui_lang_runtime::navigation(__ice_content, {message}::__AccessibilityFocusNext, {message}::__AccessibilityFocusPrevious).into() }}"
        )
        .unwrap();
    } else {
        let root_scope_code = if document.daemon {
            format!(
                "format!(\"{{}}/{{:?}}\", {}, window)",
                rust_string(&document.app)
            )
        } else {
            format!("{}.to_owned()", rust_string(&document.app))
        };
        let begin = mounted
            .iter()
            .map(|field| format!("self.{field}.begin_render();"))
            .collect::<String>();
        let finish = mounted
            .iter()
            .map(|field| format!("self.{field}.finish_render(&__ice_root_scope);"))
            .collect::<String>();
        let result = if document.daemon {
            "__ice_content".into()
        } else {
            format!(
                "::ui_lang_runtime::navigation(__ice_content, {message}::__AccessibilityFocusNext, {message}::__AccessibilityFocusPrevious).into()"
            )
        };
        writeln!(
            out,
            "fn __view(&self{window_arg}) -> __IceElement<'_, {message}> {{ {palette} let __ice_root_scope = {root_scope_code}; {begin} let __ice_content: __IceElement<'_, {message}> = {rendered_root}; {finish} {result} }}"
        )
        .unwrap();
    }
    Ok(())
}
