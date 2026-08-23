use super::*;
use crate::hir::view_children;

pub(in crate::check) fn check_declared_types(document: &Document) -> Result<(), Error> {
    let known = document
        .structs
        .iter()
        .map(|item| item.name.as_str())
        .chain(document.enums.iter().map(|item| item.name.as_str()))
        .collect::<HashSet<_>>();
    let palette_contract = document
        .theme_contract
        .as_ref()
        .map(|item| item.name.as_str());
    let check = |ty: &Type, span: &Span| check_declared_type(ty, span, &known, palette_contract);
    // `secret` names a reading of a buffer, not a value. It is legal in
    // exactly one place — an extern function's parameter list — because that
    // is the only place a reading is handed over and then dropped. Anywhere it
    // could be stored, returned, or copied is refused here.
    let reject_secret = |ty: &Type, span: &Span, place: &str| {
        if contains_secret(ty) {
            Err(Error::new(
                "E103",
                span,
                format!("`secret` cannot be {place}"),
            )
            .hint("declare `secret <name>` beside `state`, bind one `input` to it, and take it as an extern function parameter"))
        } else {
            Ok(())
        }
    };
    let reject_debug_span = |ty: &Type, span: &Span| {
        if contains_debug_span(ty) {
            Err(Error::new(
                "E103",
                span,
                "debug-span is non-clone state and must be declared as `debug-span?` state",
            ))
        } else {
            Ok(())
        }
    };

    for item in &document.structs {
        for (_, ty) in &item.fields {
            reject_secret(ty, &item.span, "an extern struct field")?;
            reject_debug_span(ty, &item.span)?;
            check(ty, &item.span)?;
        }
    }
    for item in &document.enums {
        for variant in &item.variants {
            if let Some(payload) = &variant.payload {
                if !component_value_is_cloneable(payload) {
                    return Err(Error::new(
                        "E103",
                        &variant.span,
                        "enum payloads support ordinary cloneable data only",
                    ));
                }
                reject_secret(payload, &variant.span, "an enum payload")?;
                reject_debug_span(payload, &variant.span)?;
                check(payload, &variant.span)?;
            }
        }
    }
    check_recursive_enums(document)?;
    for item in &document.functions {
        for ((_, ty), borrowed) in item.params.iter().zip(&item.borrowed) {
            // A reading is handed over once and wiped on return; a shared
            // reference cannot honor that, and the generated call would not
            // compile anyway.
            if *borrowed {
                reject_secret(ty, &item.span, "borrowed with `&`")?;
            }
            reject_debug_span(ty, &item.span)?;
            check(ty, &item.span)?;
        }
        if let Some(progress) = &item.progress {
            reject_secret(progress, &item.span, "extern progress")?;
            reject_debug_span(progress, &item.span)?;
            check(progress, &item.span)?;
        }
        reject_secret(&item.output, &item.span, "an extern return type")?;
        reject_debug_span(&item.output, &item.span)?;
        check(&item.output, &item.span)?;
        if let Some(error) = &item.error {
            reject_secret(error, &item.span, "an extern error type")?;
            reject_debug_span(error, &item.span)?;
            check(error, &item.span)?;
        }
    }
    for state in &document.states {
        reject_secret(&state.ty, &state.span, "application state")?;
        if contains_debug_span(&state.ty) && state.ty != Type::Option(Box::new(Type::DebugSpan)) {
            return Err(Error::new(
                "E103",
                &state.span,
                "debug span state must have type `debug-span?`",
            ));
        }
        check(&state.ty, &state.span)?;
    }
    for component in &document.components {
        for param in &component.params {
            reject_secret(&param.ty, &component.span, "a component input")?;
            reject_debug_span(&param.ty, &component.span)?;
            check(&param.ty, &component.span)?;
        }
        reject_debug_span(&component.output, &component.span)?;
        check(&component.output, &component.span)?;
        for event in &component.events {
            for payload in &event.payloads {
                reject_debug_span(payload, &event.span)?;
                check(payload, &event.span)?;
            }
        }
        if let Some(boot) = component
            .handlers
            .iter()
            .find(|handler| handler.name == "boot")
        {
            // Boot fires once per materialized instance, and only `mounted`
            // storage announces an instance the first time it renders and
            // drops it — with its booted mark — when it leaves.
            if component.lifetime != ComponentLifetime::Mounted {
                return Err(Error::new(
                    "E103",
                    &boot.span,
                    "component `boot` needs `lifetime mounted`, so it fires when the instance appears and can fire again after the instance leaves",
                ));
            }
        }
        for state in &component.states {
            if matches!(state.ty, Type::Animation(_)) {
                // An animation is per-instance motion, so its storage has to be
                // created the first time the instance renders and dropped when
                // the instance leaves. Only `mounted` storage does both.
                if component.lifetime != ComponentLifetime::Mounted {
                    return Err(Error::new(
                        "E103",
                        &state.span,
                        format!(
                            "animation state `{}` needs `lifetime mounted`, so its motion starts when the instance appears and is dropped when it leaves",
                            state.name
                        ),
                    ));
                }
            } else if state.ty == Type::Editor {
                // An editor's content is widget-backed and must outlive the
                // instance leaving the tree — a draft is only useful if it is
                // still there when its instance comes back. Only `retained`
                // storage keeps state across unmounts, and only a retained
                // map hands the view a plain borrow of the content.
                if component.lifetime != ComponentLifetime::Retained {
                    return Err(Error::new(
                        "E103",
                        &state.span,
                        format!(
                            "editor state `{}` needs `lifetime retained`, so its content survives the instance leaving the tree",
                            state.name
                        ),
                    ));
                }
            } else if !component_value_is_cloneable(&state.ty) {
                return Err(Error::new(
                    "E103",
                    &state.span,
                    "component state supports ordinary cloneable values only",
                ));
            }
            check(&state.ty, &state.span)?;
        }
    }
    Ok(())
}

pub(crate) fn component_value_is_cloneable(ty: &Type) -> bool {
    match ty {
        Type::Animation(_)
        | Type::Combo(_)
        | Type::DebugSpan
        | Type::Editor
        | Type::Markdown
        | Type::TaskHandle => false,
        Type::List(inner) | Type::Option(inner) => component_value_is_cloneable(inner),
        Type::Result(output, error) => {
            component_value_is_cloneable(output) && component_value_is_cloneable(error)
        }
        _ => true,
    }
}

fn check_recursive_enums(document: &Document) -> Result<(), Error> {
    fn enum_references<'a>(ty: &'a Type, names: &HashSet<&str>, output: &mut Vec<&'a str>) {
        match ty {
            Type::Named(name) if names.contains(name.as_str()) => output.push(name),
            Type::List(inner)
            | Type::Option(inner)
            | Type::Combo(inner)
            | Type::Animation(inner) => {
                enum_references(inner, names, output);
            }
            Type::Result(output_ty, error_ty) => {
                enum_references(output_ty, names, output);
                enum_references(error_ty, names, output);
            }
            _ => {}
        }
    }

    fn visit<'a>(
        name: &'a str,
        document: &'a Document,
        names: &HashSet<&str>,
        visiting: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
    ) -> bool {
        if visited.contains(name) {
            return false;
        }
        if !visiting.insert(name) {
            return true;
        }
        let item = document
            .enums
            .iter()
            .find(|item| item.name == name)
            .expect("enum reference resolves to a declaration");
        let mut references = Vec::new();
        for payload in item
            .variants
            .iter()
            .filter_map(|variant| variant.payload.as_ref())
        {
            enum_references(payload, names, &mut references);
        }
        let recursive = references
            .into_iter()
            .any(|reference| visit(reference, document, names, visiting, visited));
        visiting.remove(name);
        visited.insert(name);
        recursive
    }

    let names = document
        .enums
        .iter()
        .map(|item| item.name.as_str())
        .collect::<HashSet<_>>();
    let mut visited = HashSet::new();
    for item in &document.enums {
        if visit(
            &item.name,
            document,
            &names,
            &mut HashSet::new(),
            &mut visited,
        ) {
            return Err(Error::new(
                "E103",
                &item.span,
                format!("recursive enum `{}` is not supported", item.name),
            ));
        }
    }
    Ok(())
}

pub(in crate::check) fn contains_debug_span(ty: &Type) -> bool {
    match ty {
        Type::DebugSpan => true,
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) | Type::Animation(inner) => {
            contains_debug_span(inner)
        }
        Type::Result(output, error) => contains_debug_span(output) || contains_debug_span(error),
        _ => false,
    }
}

/// Whether a declared type reaches a `secret` anywhere inside it. A secret in
/// a list or an option would be a secret a program can keep, so the nesting
/// counts.
pub(in crate::check) fn contains_secret(ty: &Type) -> bool {
    match ty {
        Type::Secret => true,
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) | Type::Animation(inner) => {
            contains_secret(inner)
        }
        Type::Result(output, error) => contains_secret(output) || contains_secret(error),
        _ => false,
    }
}

pub(in crate::check) fn check_declared_type(
    ty: &Type,
    span: &Span,
    known: &HashSet<&str>,
    palette_contract: Option<&str>,
) -> Result<(), Error> {
    match ty {
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) => {
            check_declared_type(inner, span, known, palette_contract)
        }
        Type::Result(output, error) => {
            check_declared_type(output, span, known, palette_contract)?;
            check_declared_type(error, span, known, palette_contract)
        }
        Type::Animation(inner) if matches!(inner.as_ref(), Type::Bool | Type::F64) => Ok(()),
        Type::Animation(inner) if matches!(inner.as_ref(), Type::Named(_)) => {
            check_declared_type(inner, span, known, palette_contract)
        }
        Type::Animation(inner) => Err(Error::new(
            "E103",
            span,
            format!(
                "animation state supports `bool`, `f64`, or a named extern type, not `{}`",
                inner.display()
            ),
        )),
        Type::Named(name) if !known.contains(name.as_str()) => {
            Err(
                Error::new("E103", span, format!("unknown extern type `{name}`")).hint(format!(
                    "declare `{name}(...)` inside the extern block before using it"
                )),
            )
        }
        Type::Palette(contract) if Some(contract.as_str()) != palette_contract => Err(Error::new(
            "E103",
            span,
            format!("unknown theme contract `{contract}`"),
        )),
        _ => Ok(()),
    }
}

pub(in crate::check) fn check_unique(document: &Document) -> Result<(), Error> {
    let mut names = HashSet::new();
    for item in &document.structs {
        if !names.insert(("type", item.name.as_str())) {
            return Err(Error::new(
                "E100",
                &item.span,
                format!("duplicate struct `{}`", item.name),
            ));
        }
        let mut fields = HashSet::new();
        for (field, _) in &item.fields {
            if !fields.insert(field) {
                return Err(Error::new(
                    "E100",
                    &item.span,
                    format!("duplicate field `{field}`"),
                ));
            }
        }
    }
    for item in &document.enums {
        if !names.insert(("type", item.name.as_str())) || item.name == document.app {
            return Err(Error::new(
                "E100",
                &item.span,
                format!("duplicate type `{}`", item.name),
            ));
        }
        let mut variants = HashSet::new();
        let mut rust_variants = HashSet::new();
        for variant in &item.variants {
            if !variants.insert(&variant.name) {
                return Err(Error::new(
                    "E100",
                    &variant.span,
                    format!("duplicate enum variant `{}`", variant.name),
                ));
            }
            let rust_name = variant
                .name
                .split('_')
                .map(|part| {
                    let mut chars = part.chars();
                    chars.next().map_or_else(String::new, |first| {
                        first.to_uppercase().collect::<String>() + chars.as_str()
                    })
                })
                .collect::<String>();
            if !rust_variants.insert(rust_name) {
                return Err(Error::new(
                    "E100",
                    &variant.span,
                    format!(
                        "enum variant `{}` conflicts with another generated variant name",
                        variant.name
                    ),
                ));
            }
        }
    }
    for item in &document.functions {
        if !names.insert(("fn", item.name.as_str())) {
            return Err(Error::new(
                "E100",
                &item.span,
                format!("duplicate function `{}`", item.name),
            ));
        }
    }
    let mut presets = HashSet::new();
    for preset in &document.presets {
        if !presets.insert(&preset.name) {
            return Err(Error::new(
                "E100",
                &preset.span,
                format!("duplicate preset `{}`", preset.name),
            ));
        }
    }
    let mut recipes = HashSet::new();
    for recipe in &document.recipes {
        if !recipes.insert(&recipe.name) {
            return Err(Error::new(
                "E100",
                &recipe.span,
                format!("duplicate recipe `{}`", recipe.name),
            ));
        }
    }
    let mut fields = HashSet::new();
    for state in &document.states {
        if document.daemon && state.name == "window" {
            return Err(
                Error::new("E100", &state.span, "daemon state cannot be named `window`")
                    .hint("`window` is the current window-id inside daemon views and callbacks"),
            );
        }
        if !fields.insert(&state.name) {
            return Err(Error::new(
                "E100",
                &state.span,
                format!("duplicate app field `{}`", state.name),
            ));
        }
    }
    for derived in &document.derived {
        if document.daemon && derived.name == "window" {
            return Err(Error::new(
                "E100",
                &derived.span,
                "daemon derived value cannot be named `window`",
            ));
        }
        if !fields.insert(&derived.name) {
            return Err(Error::new(
                "E100",
                &derived.span,
                format!("duplicate app value `{}`", derived.name),
            ));
        }
    }
    for secret in &document.secrets {
        if !fields.insert(&secret.name) {
            return Err(Error::new(
                "E100",
                &secret.span,
                format!("duplicate app value `{}`", secret.name),
            ));
        }
    }
    let mut handlers = HashSet::new();
    for handler in &document.handlers {
        if !handlers.insert(&handler.name) {
            return Err(Error::new(
                "E100",
                &handler.span,
                format!("duplicate handler `{}`", handler.name),
            ));
        }
    }
    let mut tests = HashSet::new();
    for test in &document.tests {
        if !tests.insert(&test.name) {
            return Err(Error::new(
                "E100",
                &test.span,
                format!("duplicate test `{}`", test.name),
            ));
        }
        let mut aliases = HashSet::new();
        for target in &test.targets {
            if !aliases.insert(&target.name) {
                return Err(Error::new(
                    "E100",
                    &target.span,
                    format!(
                        "duplicate target alias `{}` in test `{}`",
                        target.name, test.name
                    ),
                ));
            }
            if document
                .states
                .iter()
                .any(|state| state.name == target.name)
                || document
                    .derived
                    .iter()
                    .any(|derived| derived.name == target.name)
                || document.daemon && target.name == "window"
            {
                return Err(Error::new(
                    "E100",
                    &target.span,
                    format!(
                        "target alias `{}` conflicts with app state in test `{}`",
                        target.name, test.name
                    ),
                ));
            }
        }
    }
    let mut components = HashSet::new();
    for component in &document.components {
        if !components.insert(&component.name) {
            return Err(Error::new(
                "E100",
                &component.span,
                format!("duplicate component `{}`", component.name),
            ));
        }
        let mut params = HashSet::new();
        for param in &component.params {
            if !params.insert(&param.name) {
                return Err(Error::new(
                    "E100",
                    &component.span,
                    format!("duplicate component prop `{}`", param.name),
                ));
            }
        }
        for state in &component.states {
            if !params.insert(&state.name) {
                return Err(Error::new(
                    "E100",
                    &state.span,
                    format!("duplicate component value `{}`", state.name),
                ));
            }
        }
        let mut local_handlers = HashSet::new();
        for handler in &component.handlers {
            if matches!(handler.name.as_str(), "mount" | "emit") {
                return Err(Error::new(
                    "E100",
                    &handler.span,
                    format!("component handlers cannot be named `{}`", handler.name),
                ));
            }
            if !local_handlers.insert(&handler.name) {
                return Err(Error::new(
                    "E100",
                    &handler.span,
                    format!("duplicate component handler `{}`", handler.name),
                ));
            }
        }
        let mut events = HashSet::new();
        for event in &component.events {
            if !events.insert(&event.name) {
                return Err(Error::new(
                    "E100",
                    &event.span,
                    format!("duplicate component event `{}`", event.name),
                ));
            }
        }
    }
    for handler in document.handlers.iter().chain(
        document
            .components
            .iter()
            .flat_map(|component| &component.handlers),
    ) {
        let mut params = HashSet::new();
        if let Some(param) = handler
            .params
            .iter()
            .find(|param| !params.insert(&param.name))
        {
            return Err(Error::new(
                "E100",
                &handler.span,
                format!("duplicate handler parameter `{}`", param.name),
            ));
        }
    }
    Ok(())
}

pub(in crate::check) fn check_fonts(document: &Document) -> Result<(), Error> {
    let mut names = HashSet::new();
    let mut default = None;
    for font in &document.fonts {
        if !names.insert(&font.name) {
            return Err(Error::new(
                "E100",
                &font.span,
                format!("duplicate font `{}`", font.name),
            ));
        }
        if font.default && default.replace(&font.name).is_some() {
            return Err(Error::new(
                "E114",
                &font.span,
                "only one font may be default",
            ));
        }
    }
    Ok(())
}

pub(in crate::check) fn check_font(
    font: Option<&FontPreset>,
    document: &Document,
    span: &Span,
) -> Result<(), Error> {
    if let Some(FontPreset::Named(name)) = font
        && !document.fonts.iter().any(|font| font.name == *name)
    {
        return Err(Error::new("E114", span, format!("unknown font `{name}`"))
            .hint(format!("declare `font {name} ...` before using it")));
    }
    Ok(())
}

pub(in crate::check) fn check_slots(document: &Document) -> Result<(), Error> {
    let view_slots = slots(&document.view);
    if let Some((_, _, span)) = view_slots.first() {
        return Err(Error::new(
            "E124",
            span,
            "slot is only valid inside a component definition",
        ));
    }
    for test in &document.tests {
        if let Some(mount) = &test.mount
            && let Some((_, _, span)) = slots(mount).first()
        {
            return Err(Error::new(
                "E124",
                span,
                "slot is only valid inside a component definition",
            ));
        }
    }
    for component in &document.components {
        let mut names = HashSet::new();
        for (name, _, span) in slots(&component.root) {
            if !names.insert(name) {
                return Err(Error::new(
                    "E124",
                    span,
                    format!(
                        "component `{}` declares slot `{name}` more than once",
                        component.name
                    ),
                ));
            }
        }
    }
    Ok(())
}

pub(in crate::check) fn slots(node: &ViewNode) -> Vec<(&str, bool, &Span)> {
    fn collect<'a>(node: &'a ViewNode, output: &mut Vec<(&'a str, bool, &'a Span)>) {
        if let ViewNode::Slot {
            name,
            optional,
            span,
        } = node
        {
            output.push((name, *optional, span));
        }
        for child in view_children(node) {
            collect(child, output);
        }
    }

    let mut output = Vec::new();
    collect(node, &mut output);
    output
}

#[cfg(test)]
mod unique_tests {
    use super::*;

    #[test]
    fn extern_structs_and_enums_share_a_type_namespace() {
        let document = crate::parse(
            "app UniqueTypes\nextern crate::backend\n  Status()\nenum Status\n  ready\nview\n  text \"ok\"\n",
        )
        .unwrap();

        let error = check_unique(&document).unwrap_err();
        assert_eq!(error.code, "E100");
        assert_eq!(error.message, "duplicate type `Status`");
    }
}
