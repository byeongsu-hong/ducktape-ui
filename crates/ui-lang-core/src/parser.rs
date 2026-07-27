use crate::ast::*;
use crate::{Error, SourceRange, SymbolKind};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub(crate) struct ParsedSymbol {
    pub kind: SymbolKind,
    pub scope: Option<String>,
    pub name: String,
    pub definition: bool,
    pub range: Option<SourceRange>,
}

#[derive(Clone, Debug)]
struct Line {
    number: usize,
    indent: usize,
    text: String,
    children: Vec<Line>,
    namespace: Option<String>,
    symbols: Rc<RefCell<Vec<ParsedSymbol>>>,
    track_symbols: bool,
}

impl Line {
    fn qualify(&self, name: &str) -> String {
        match &self.namespace {
            Some(namespace)
                if name != namespace && !name.starts_with(&format!("{namespace}::")) =>
            {
                format!("{namespace}::{name}")
            }
            _ => name.to_owned(),
        }
    }

    fn record_symbol(&self, kind: SymbolKind, name: &str, definition: bool, source: &str) {
        self.record_scoped_symbol(kind, None, name, definition, source);
    }

    fn record_scoped_symbol(
        &self,
        kind: SymbolKind,
        scope: Option<&str>,
        name: &str,
        definition: bool,
        source: &str,
    ) {
        if !self.track_symbols {
            return;
        }
        let source_name = name.rsplit("::").next().unwrap_or(name);
        let relative = source.find(source_name);
        let line_start = self.text.as_ptr() as usize;
        let source_start = source.as_ptr() as usize;
        let direct = (source_start >= line_start
            && source_start + source.len() <= line_start + self.text.len())
        .then(|| source_start - line_start);
        let used = |offset: usize| {
            let column = self.indent + self.text[..offset].chars().count() + 1;
            self.symbols.borrow().iter().any(|symbol| {
                symbol
                    .range
                    .as_ref()
                    .is_some_and(|range| range.line == self.number && range.start_column == column)
            })
        };
        let fallback = relative.and_then(|relative| {
            let candidates = self
                .text
                .match_indices(source)
                .map(|(offset, _)| offset + relative)
                .filter(|offset| {
                    !used(*offset) && !self.text[*offset + source_name.len()..].starts_with('=')
                })
                .collect::<Vec<_>>();
            let assigned = candidates
                .iter()
                .copied()
                .filter(|offset| self.text[..*offset].trim_end().ends_with('='))
                .collect::<Vec<_>>();
            match (assigned.as_slice(), candidates.as_slice()) {
                ([offset], _) | ([], [offset]) => Some(*offset),
                _ => None,
            }
        });
        let range = relative
            .and_then(|relative| direct.map(|offset| offset + relative))
            .filter(|offset| !used(*offset))
            .or(fallback)
            .map(|offset| {
                let start_column = self.indent + self.text[..offset].chars().count() + 1;
                SourceRange {
                    path: None,
                    line: self.number,
                    start_column,
                    end_column: start_column + source_name.chars().count(),
                }
            });
        self.symbols.borrow_mut().push(ParsedSymbol {
            kind,
            scope: scope.map(str::to_owned),
            name: name.to_owned(),
            definition,
            range,
        });
    }

    fn record_component_reference(&self, name: &str) {
        if !self.track_symbols {
            return;
        }
        let source_name = name.rsplit("::").next().unwrap_or(name);
        let offset = self.text.find(source_name);
        let range = offset.map(|offset| SourceRange {
            path: None,
            line: self.number,
            start_column: self.indent + self.text[..offset].chars().count() + 1,
            end_column: self.indent
                + self.text[..offset].chars().count()
                + 1
                + source_name.chars().count(),
        });
        self.symbols.borrow_mut().push(ParsedSymbol {
            kind: SymbolKind::Component,
            scope: None,
            name: name.to_owned(),
            definition: false,
            range,
        });
    }
}

pub fn parse(source: &str) -> Result<Document, Error> {
    parse_with_symbols(source).map(|(document, _)| document)
}

fn parse_padding_option(
    part: &str,
    padding: &mut PaddingOptions,
    line: &Line,
) -> Result<bool, Error> {
    let Some((name, value)) = part.split_once('=') else {
        return Ok(false);
    };
    let slot = match name {
        "p" => &mut padding.all,
        "px" => &mut padding.x,
        "py" => &mut padding.y,
        "pt" => &mut padding.top,
        "pr" => &mut padding.right,
        "pb" => &mut padding.bottom,
        "pl" => &mut padding.left,
        _ => return Ok(false),
    };
    *slot = Some(parse_expr(strip_wrapping_parens(value), line)?);
    Ok(true)
}

fn parse_accessibility_option(
    part: &str,
    accessibility: &mut AccessibilityOptions,
    line: &Line,
) -> Result<bool, Error> {
    let Some((name, value)) = part.split_once('=') else {
        return Ok(false);
    };
    let slot = match name {
        "label" => &mut accessibility.label,
        "description" => &mut accessibility.description,
        _ => return Ok(false),
    };
    *slot = Some(parse_expr(strip_wrapping_parens(value), line)?);
    Ok(true)
}

fn parse_unique_id(
    source: &str,
    id: &mut Option<Id>,
    line: &Line,
    code: &'static str,
    widget: &str,
) -> Result<(), Error> {
    if id.is_some() {
        return Err(error(code, line, format!("{widget} has more than one ID")));
    }
    *id = Some(parse_id(source, line)?);
    Ok(())
}

fn parse_line_height_option(
    part: &str,
    line_height: &mut Option<TextLineHeight>,
    line: &Line,
) -> Result<bool, Error> {
    let (value, absolute) = if let Some(value) = part.strip_prefix("line-h=") {
        (value, false)
    } else if let Some(value) = part.strip_prefix("line-h-px=") {
        (value, true)
    } else {
        return Ok(false);
    };
    let value = parse_expr(strip_wrapping_parens(value), line)?;
    *line_height = Some(if absolute {
        TextLineHeight::Absolute(value)
    } else {
        TextLineHeight::Relative(value)
    });
    Ok(true)
}

fn parse_char_literal(
    source: &str,
    line: &Line,
    code: &'static str,
    message: impl Into<String>,
) -> Result<char, Error> {
    let value = string_literal(source, line)?;
    let mut chars = value.chars();
    match (chars.next(), chars.next()) {
        (Some(value), None) => Ok(value),
        _ => Err(error(code, line, message)),
    }
}

fn parse_extern_call(
    source: &str,
    line: &Line,
    code: &'static str,
    message: impl Into<String>,
) -> Result<ExternCall, Error> {
    let (function, args) = parse_signature(source, line).map_err(|_| error(code, line, message))?;
    Ok(ExternCall {
        function,
        args: parse_expr_list(&args, line)?,
    })
}

fn parse_style_recipe(source: &str, line: &Line) -> Result<StyleRecipe, Error> {
    let parts = split_words(source);
    let (name, separator, target, base) = match parts.as_slice() {
        [name, separator, target] => (name, separator, target, None),
        [name, separator, target, extends, base] if extends == "extends" => {
            (name, separator, target, Some(base))
        }
        _ => {
            return Err(error(
                "E046",
                line,
                "recipe uses `recipe name for target` or `recipe name for target extends base`",
            ));
        }
    };
    if separator != "for" {
        return Err(error(
            "E046",
            line,
            "recipe uses `recipe name for target` or `recipe name for target extends base`",
        ));
    }
    if line.children.is_empty() {
        return Err(error("E046", line, "recipe requires at least one utility"));
    }
    let target = match target.as_str() {
        "col" => StyleRecipeTarget::Column,
        "row" => StyleRecipeTarget::Row,
        "flex" => StyleRecipeTarget::Flex,
        "grid" => StyleRecipeTarget::Grid,
        "stack" => StyleRecipeTarget::Stack,
        "box" => StyleRecipeTarget::Container,
        "text" => StyleRecipeTarget::Text,
        "input" => StyleRecipeTarget::Input,
        "button" => StyleRecipeTarget::Button,
        _ => {
            return Err(error(
                "E046",
                line,
                "recipe target must be col, row, flex, grid, stack, box, text, input, or button",
            ));
        }
    };
    let mut utilities = Vec::new();
    for child in &line.children {
        ensure_leaf(child)?;
        let source = child.text.strip_prefix('@').unwrap_or(&child.text);
        utilities.extend(source.split_ascii_whitespace().map(str::to_owned));
    }
    if utilities.is_empty() {
        return Err(error("E046", line, "recipe requires at least one utility"));
    }
    let name = line.qualify(&identifier(name, line)?);
    line.record_symbol(SymbolKind::Recipe, &name, true, source);
    let base = base.map(|base| identifier(base, line)).transpose()?;
    if let Some(base) = &base {
        line.record_symbol(SymbolKind::Recipe, base, false, source);
    }
    Ok(StyleRecipe {
        name,
        target,
        base,
        utilities,
        span: Span::line(line.number),
    })
}

fn parse_theme_contract(name: &str, line: &Line) -> Result<ThemeContract, Error> {
    let name = identifier(name.trim(), line)?;
    if !name
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        return Err(error(
            "E072",
            line,
            format!("invalid theme contract name `{name}`"),
        ));
    }
    let mut tokens = Vec::new();
    for item in &line.children {
        ensure_leaf(item)?;
        let token = identifier(&item.text, item)?;
        if matches!(token.as_str(), "white" | "black" | "transparent") {
            return Err(error(
                "E012",
                item,
                format!("theme token `{token}` is built in and cannot be declared"),
            ));
        }
        if tokens.contains(&token) {
            return Err(error(
                "E012",
                item,
                format!("duplicate theme contract token `{token}`"),
            ));
        }
        tokens.push(token);
    }
    Ok(ThemeContract {
        name,
        tokens,
        span: Span::line(line.number),
    })
}

fn parse_palette(source: &str, line: &Line) -> Result<Palette, Error> {
    let parts = split_words(source);
    let [name, separator, contract] = parts.as_slice() else {
        return Err(error(
            "E010",
            line,
            "palette uses `palette name for Contract`",
        ));
    };
    if separator != "for" {
        return Err(error(
            "E010",
            line,
            "palette uses `palette name for Contract`",
        ));
    }
    let mut colors = BTreeMap::new();
    for item in &line.children {
        ensure_leaf(item)?;
        let Some((token, value)) = item.text.split_once(char::is_whitespace) else {
            return Err(error("E010", item, "expected `token #RRGGBB`"));
        };
        let token = identifier(token, item)?;
        let value = value.trim();
        if !valid_color(value) {
            return Err(error(
                "E011",
                item,
                "palette colors use #RRGGBB or #RRGGBBAA",
            ));
        }
        if colors.insert(token.clone(), value.into()).is_some() {
            return Err(error(
                "E012",
                item,
                format!("duplicate palette token `{token}`"),
            ));
        }
    }
    Ok(Palette {
        name: identifier(name, line)?,
        contract: identifier(contract, line)?,
        colors,
        span: Span::line(line.number),
    })
}

pub(crate) fn parse_with_symbols(source: &str) -> Result<(Document, Vec<ParsedSymbol>), Error> {
    parse_with_symbols_and_namespaces(source, &vec![None; source.lines().count()])
}

pub(crate) fn parse_with_symbols_and_namespaces(
    source: &str,
    namespaces: &[Option<String>],
) -> Result<(Document, Vec<ParsedSymbol>), Error> {
    let symbols = Rc::new(RefCell::new(Vec::new()));
    let lines = line_tree(source, namespaces, Rc::clone(&symbols))?;
    let mut app = None;
    let mut daemon = false;
    let mut settings = AppSettings::default();
    let mut presets = Vec::new();
    let mut recipes = Vec::new();
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut functions = Vec::new();
    let mut subscriptions = Vec::new();
    let mut theme_contract = None;
    let mut palettes = Vec::new();
    let mut fonts = Vec::new();
    let mut qr_codes = Vec::new();
    let mut states = Vec::new();
    let mut derived = Vec::new();
    let mut components = Vec::new();
    let mut handlers = Vec::new();
    let mut tests = Vec::new();
    let mut view = None;

    for line in &lines {
        if line.namespace.is_some()
            && !(line.text.starts_with("recipe ")
                || line.text.starts_with("extern ")
                || line.text.starts_with("enum ")
                || line.text.starts_with("theme contract ")
                || line.text.starts_with("palette ")
                || line.text.starts_with("font ")
                || line.text.starts_with("component "))
        {
            return Err(error(
                "E180",
                line,
                "aliased imports may declare components, recipes, extern items, enums, fonts, and the global theme contract only",
            ));
        }
        let root = line
            .text
            .strip_prefix("app ")
            .map(|name| (name, false))
            .or_else(|| line.text.strip_prefix("daemon ").map(|name| (name, true)));
        if let Some((name, is_daemon)) = root {
            if app.replace(identifier(name.trim(), line)?).is_some() {
                return Err(error(
                    "E002",
                    line,
                    "an app or daemon may only be declared once",
                ));
            }
            settings = parse_app_settings(line)?;
            if is_daemon
                && let Some(window) = line.children.iter().find(|item| item.text == "window")
            {
                return Err(error(
                    "E014",
                    window,
                    "a daemon has no initial window",
                )
                .hint("declare a named `window name` and open it with `task window open name -> handler _`"));
            }
            daemon = is_daemon;
        } else if let Some(source) = line.text.strip_prefix("recipe ") {
            recipes.push(parse_style_recipe(source, line)?);
        } else if let Some(name) = line.text.strip_prefix("enum ") {
            enums.push(parse_ui_enum(name.trim(), line)?);
        } else if let Some(name) = line.text.strip_prefix("preset ") {
            presets.push(parse_preset(name.trim(), line)?);
        } else if let Some(path) = line.text.strip_prefix("extern ") {
            let path = rust_path(path.trim(), line)?;
            for item in &line.children {
                if let Some(source) = item.text.strip_prefix("component ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Component)?);
                } else if let Some(source) = item.text.strip_prefix("shader ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Shader)?);
                } else if let Some(source) = item.text.strip_prefix("task ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Task)?);
                } else if let Some(source) = item.text.strip_prefix("stream ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Stream)?);
                } else if let Some(source) = item.text.strip_prefix("sip ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Sip)?);
                } else if let Some(source) = item.text.strip_prefix("recipe ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Recipe)?);
                } else if let Some(source) = item.text.strip_prefix("selector ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Selector)?);
                } else if let Some(source) = item.text.strip_prefix("event-filter ") {
                    let function = parse_extern_fn(source, item, &path, ExternKind::EventFilter)?;
                    if !function.params.is_empty() {
                        return Err(error(
                            "E022",
                            item,
                            "event filters receive the iced runtime event implicitly and declare no parameters",
                        ));
                    }
                    functions.push(function);
                } else if let Some(source) = item.text.strip_prefix("sync ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Sync)?);
                } else if let Some(source) = item.text.strip_prefix("subscription ") {
                    functions.push(parse_extern_fn(
                        source,
                        item,
                        &path,
                        ExternKind::Subscription,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("theme ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::Theme,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("themer ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Themer)?);
                } else if let Some(source) = item.text.strip_prefix("window ") {
                    functions.push(parse_extern_fn(source, item, &path, ExternKind::Window)?);
                } else if let Some(source) = item.text.strip_prefix("markdown-viewer ") {
                    functions.push(parse_extern_fn(
                        source,
                        item,
                        &path,
                        ExternKind::MarkdownViewer,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("editor-binding ") {
                    functions.push(parse_extern_fn(
                        source,
                        item,
                        &path,
                        ExternKind::EditorBinding,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("editor-highlighter ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::EditorHighlighter,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("editor-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::EditorStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("text-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::TextStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("slider-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::SliderStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("progress-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::ProgressStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("button-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::ButtonStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("checkbox-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::CheckboxStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("toggler-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::TogglerStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("radio-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::RadioStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("box-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::ContainerStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("svg-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::SvgStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("input-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::InputStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("scroll-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::ScrollStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("pick-list-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::PickListStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("menu-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::MenuStyle,
                    )?);
                } else if let Some(source) = item.text.strip_prefix("panes-style ") {
                    functions.push(parse_extern_fn(
                        &format!("{source} -> unit"),
                        item,
                        &path,
                        ExternKind::PaneGridStyle,
                    )?);
                } else if item.text.chars().next().is_some_and(char::is_uppercase) {
                    structs.push(parse_extern_struct(item, &path)?);
                } else {
                    functions.push(parse_extern_fn(
                        &item.text,
                        item,
                        &path,
                        ExternKind::Future,
                    )?);
                }
            }
        } else if let Some(name) = line.text.strip_prefix("theme contract ") {
            if theme_contract.is_some() {
                return Err(error("E012", line, "duplicate theme contract"));
            }
            theme_contract = Some(parse_theme_contract(name, line)?);
        } else if let Some(source) = line.text.strip_prefix("palette ") {
            let palette = parse_palette(source, line)?;
            if palettes
                .iter()
                .any(|existing: &Palette| existing.name == palette.name)
            {
                return Err(error(
                    "E012",
                    line,
                    format!("duplicate palette `{}`", palette.name),
                ));
            }
            palettes.push(palette);
        } else if line.text == "state" {
            states.extend(
                line.children
                    .iter()
                    .map(parse_state)
                    .collect::<Result<Vec<_>, _>>()?,
            );
        } else if line.text == "derived" {
            if line.children.is_empty() {
                return Err(error("E032", line, "derived cannot be empty"));
            }
            for item in &line.children {
                ensure_leaf(item)?;
                let Some((name, value)) = split_top_once(&item.text, '=') else {
                    return Err(error(
                        "E032",
                        item,
                        "derived values use `name = expression`",
                    ));
                };
                derived.push(Derived {
                    name: identifier(name.trim(), item)?,
                    value: parse_expr(value.trim(), item)?,
                    ty: Type::Unknown,
                    span: Span::line(item.number),
                });
            }
        } else if let Some(source) = line.text.strip_prefix("font ") {
            fonts.push(parse_font(source, line)?);
        } else if line.text == "qr" || line.text.starts_with("qr ") {
            qr_codes.push(parse_qr_data(line.text[2..].trim(), line)?);
        } else if let Some(header) = line.text.strip_prefix("component ") {
            components.push(parse_component(header, line)?);
        } else if let Some(header) = line.text.strip_prefix("on ") {
            handlers.push(parse_handler(header, line)?);
        } else if line.text == "test" || line.text.starts_with("test ") {
            tests.push(parse_test_decl(
                line.text.strip_prefix("test").unwrap().trim(),
                line,
            )?);
        } else if line.text == "subscribe" {
            subscriptions.extend(
                line.children
                    .iter()
                    .map(parse_subscription)
                    .collect::<Result<Vec<_>, _>>()?,
            );
        } else if line.text == "view" {
            if view.is_some() {
                return Err(error("E004", line, "an app may only have one view"));
            }
            if line.children.len() != 1 {
                return Err(error(
                    "E005",
                    line,
                    "view must contain exactly one root node",
                ));
            }
            view = Some(parse_view(&line.children[0])?);
        } else {
            return Err(error(
                "E001",
                line,
                format!("unknown declaration `{}`", line.text),
            ));
        }
    }

    let shadows_image_constructor = |state: &State| {
        fn contains_shadowed_call(expr: &Expr, functions: &[ExternFn]) -> bool {
            match expr {
                Expr::Call { name, .. }
                    if matches!(crate::unqualified_name(name), "encoded" | "rgba") =>
                {
                    functions
                        .iter()
                        .any(|function| function.kind == ExternKind::Sync && function.name == *name)
                }
                Expr::List(values) => values
                    .iter()
                    .any(|value| contains_shadowed_call(value, functions)),
                _ => false,
            }
        }

        let declared = state
            .span
            .line
            .checked_sub(1)
            .and_then(|line| source.lines().nth(line))
            .and_then(|line| split_top_once(line.trim(), '='))
            .is_some_and(|(left, _)| left.contains(':'));
        !declared && contains_shadowed_call(&state.initial, &functions)
    };
    if let Some(state) = states
        .iter()
        .chain(components.iter().flat_map(|component| &component.states))
        .find(|state| shadows_image_constructor(state))
    {
        return Err(Error::new(
            "E031",
            &state.span,
            "state requires an explicit type when a sync function shadows `encoded` or `rgba`",
        ));
    }

    let span = Span::line(1);
    let document = Document {
        app: app.ok_or_else(|| {
            Error::new(
                "E006",
                &span,
                "missing `app Name` or `daemon Name` declaration",
            )
        })?,
        daemon,
        settings,
        presets,
        recipes,
        structs,
        enums,
        functions,
        subscriptions,
        theme_contract,
        palettes,
        fonts,
        qr_codes,
        states,
        derived,
        components,
        handlers,
        tests,
        view: view.ok_or_else(|| Error::new("E008", &span, "missing `view` block"))?,
    };
    Ok((document, symbols.borrow().clone()))
}

mod canvas;
mod controls;
mod declarations;
mod expression;
mod settings;
mod statement;
pub(crate) mod syntax;
mod testing;
pub(crate) mod view;

use canvas::*;
use controls::*;
use declarations::*;
use expression::*;
use settings::*;
use statement::*;
use syntax::*;
use testing::*;
use view::*;

#[cfg(test)]
#[path = "parser/tests.rs"]
mod tests;
