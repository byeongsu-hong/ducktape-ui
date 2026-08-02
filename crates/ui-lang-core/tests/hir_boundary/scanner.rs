use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SourceFile {
    pub(super) relative: String,
    pub(super) source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Token {
    text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Occurrence {
    path: String,
    fingerprint: String,
    normalized_item: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AstBinding {
    Glob,
    Name(String),
    Module(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedAstBinding {
    binding: AstBinding,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct UseLeaf {
    path: Vec<String>,
    alias: Option<String>,
    glob: bool,
    use_index: usize,
    start: usize,
    end: usize,
}

#[derive(Default)]
struct AstUseMarkers {
    import_indices: BTreeSet<usize>,
    use_ranges: Vec<(usize, usize)>,
    bindings: Vec<ScopedAstBinding>,
}

const CATEGORIES: &[&str] = &[
    "source AST import",
    "source AST semantic reference",
    "checked-document escape",
    "raw document wrapper",
    "checker semantic reference",
    "checked-facts escape",
    "declaration-index escape",
    "type re-analysis",
    "extern re-resolution",
    "raw expression fallback",
    "Document reference",
    "Expr reference",
    "Route reference",
    "Statement reference",
];

pub(super) fn is_production_codegen_path(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rs")
        && path.file_name().is_none_or(|name| name != "tests.rs")
        && !path.components().any(|part| part.as_os_str() == "tests")
}

pub(super) fn exported_ast_types(ast_sources: &[String]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for source in ast_sources {
        let tokens = lex(source);
        for (start, end) in item_ranges(&tokens) {
            let item = &tokens[start..end];
            let Some(mut index) = top_level_token(item, "pub") else {
                continue;
            };
            index += 1;
            if item.get(index).is_some_and(|token| token.text == "(") {
                let mut depth = 1;
                index += 1;
                while index < item.len() && depth != 0 {
                    match item[index].text.as_str() {
                        "(" => depth += 1,
                        ")" => depth -= 1,
                        _ => {}
                    }
                    index += 1;
                }
            }
            let declaration = item.get(index).map(|token| token.text.as_str());
            let name_index = index
                + 1
                + usize::from(
                    declaration == Some("static")
                        && item.get(index + 1).is_some_and(|token| token.text == "mut"),
                );
            if declaration.is_some_and(|declaration| {
                matches!(
                    declaration,
                    "struct" | "enum" | "type" | "trait" | "fn" | "const" | "static"
                )
            }) && let Some(name) = item.get(name_index)
                && is_identifier(&name.text)
            {
                names.insert(name.text.clone());
            }
        }
    }
    names
}

pub(super) fn inventory(
    files: &[SourceFile],
    ast_types: &BTreeSet<String>,
) -> Result<String, String> {
    let checker_symbols = checker_symbols(files)?;
    let mut by_category: BTreeMap<&str, Vec<Occurrence>> = CATEGORIES
        .iter()
        .map(|category| (*category, Vec::new()))
        .collect();

    for file in files {
        let tokens = lex(&file.source);
        let items = item_ranges(&tokens);
        let ast_uses = ast_use_markers(&tokens, &items, ast_types);
        for index in 0..tokens.len() {
            let text = tokens[index].text.as_str();
            if ast_uses.import_indices.contains(&index) {
                record(
                    &mut by_category,
                    "source AST import",
                    file,
                    &tokens,
                    &items,
                    index,
                );
            }
            let ast_reference = ast_semantic_reference_at(&tokens, index, ast_types, &ast_uses);
            if ast_reference {
                record(
                    &mut by_category,
                    "source AST semantic reference",
                    file,
                    &tokens,
                    &items,
                    index,
                );
            }
            if method_call_at(&tokens, index, "document")
                || associated_call_at(&tokens, index, "LoweredProgram", "document")
            {
                record(
                    &mut by_category,
                    "checked-document escape",
                    file,
                    &tokens,
                    &items,
                    index,
                );
            }
            if text == "RenderDocument" {
                record(
                    &mut by_category,
                    "raw document wrapper",
                    file,
                    &tokens,
                    &items,
                    index,
                );
            }
            if checker_symbols.contains(text) || checker_path_at(&tokens, index) {
                record(
                    &mut by_category,
                    "checker semantic reference",
                    file,
                    &tokens,
                    &items,
                    index,
                );
            }
            for (category, token) in [
                ("Document reference", "Document"),
                ("Expr reference", "Expr"),
                ("Route reference", "Route"),
                ("Statement reference", "Statement"),
            ] {
                if text == token && ast_reference {
                    record(&mut by_category, category, file, &tokens, &items, index);
                }
            }
            for (category, method) in [
                ("checked-facts escape", "checked_facts"),
                ("declaration-index escape", "declarations"),
            ] {
                if method_call_at(&tokens, index, method) {
                    record(&mut by_category, category, file, &tokens, &items, index);
                }
            }
            for (category, function) in [
                ("type re-analysis", "expr_type"),
                ("extern re-resolution", "find_extern_function"),
            ] {
                if function_call_at(&tokens, index, function) {
                    record(&mut by_category, category, file, &tokens, &items, index);
                }
            }
            if sequence_at(&tokens, index, &["ExprNode", "::", "Ast"]) {
                record(
                    &mut by_category,
                    "raw expression fallback",
                    file,
                    &tokens,
                    &items,
                    index,
                );
            }
        }
    }

    let mut item_hashes = BTreeMap::<u128, String>::new();
    for occurrences in by_category.values() {
        for occurrence in occurrences {
            let digest = fnv1a_128(occurrence.normalized_item.as_bytes());
            if let Some(previous) = item_hashes.insert(digest, occurrence.normalized_item.clone())
                && previous != occurrence.normalized_item
            {
                return Err(format!(
                    "boundary item fingerprint hash collision: {digest:032x}"
                ));
            }
        }
    }

    let mut output = String::new();
    let mut seen_hashes = BTreeMap::<u128, String>::new();
    for category in CATEGORIES {
        output.push_str(&format!("[{category}]\n"));
        let mut by_path = BTreeMap::<&str, Vec<&str>>::new();
        for occurrence in &by_category[category] {
            by_path
                .entry(&occurrence.path)
                .or_default()
                .push(&occurrence.fingerprint);
        }
        for (path, fingerprints) in by_path {
            let joined = fingerprints.join("\n");
            let digest = fnv1a_128(joined.as_bytes());
            if let Some(previous) = seen_hashes.insert(digest, joined.clone())
                && previous != joined
            {
                return Err(format!(
                    "boundary fingerprint hash collision: {digest:032x}"
                ));
            }
            output.push_str(&format!("{path} {} {digest:032x}\n", fingerprints.len()));
        }
    }
    Ok(output.trim_end().to_owned())
}

fn record(
    categories: &mut BTreeMap<&str, Vec<Occurrence>>,
    category: &'static str,
    file: &SourceFile,
    tokens: &[Token],
    items: &[(usize, usize)],
    index: usize,
) {
    let (start, end) = items
        .iter()
        .copied()
        .find(|(start, end)| *start <= index && index < *end)
        .unwrap_or((0, tokens.len()));
    let normalized_item = normalize(&tokens[start..end]);
    let context_start = index.saturating_sub(4).max(start);
    let context_end = (index + 5).min(end);
    let context = normalize(&tokens[context_start..context_end]);
    categories.get_mut(category).unwrap().push(Occurrence {
        path: file.relative.clone(),
        fingerprint: format!(
            "{}:{:032x}:{context}",
            index - start,
            fnv1a_128(normalized_item.as_bytes())
        ),
        normalized_item,
    });
}

fn checker_symbols(files: &[SourceFile]) -> Result<BTreeSet<String>, String> {
    let mut symbols = BTreeSet::new();
    for file in files {
        let tokens = lex(&file.source);
        for (start, end) in item_ranges(&tokens) {
            let item = &tokens[start..end];
            let Some(check) = rooted_use_module(item, "check") else {
                continue;
            };
            if item.iter().skip(check + 3).any(|token| token.text == "*") {
                return Err(format!(
                    "{} uses a checker glob import; list checker symbols explicitly",
                    file.relative
                ));
            }
            let mut index = check + 3;
            while index < item.len() {
                if is_identifier(&item[index].text)
                    && !matches!(item[index].text.as_str(), "as" | "self" | "super" | "crate")
                {
                    let name = if item.get(index + 1).is_some_and(|token| token.text == "as") {
                        item.get(index + 2).map_or(&item[index], |alias| alias)
                    } else {
                        &item[index]
                    };
                    symbols.insert(name.text.clone());
                }
                index += 1;
            }
        }
    }
    Ok(symbols)
}

fn ast_use_markers(
    tokens: &[Token],
    items: &[(usize, usize)],
    ast_types: &BTreeSet<String>,
) -> AstUseMarkers {
    let mut markers = AstUseMarkers::default();
    let depths = token_depths(tokens);
    let mut leaves = Vec::new();
    for (use_index, token) in tokens.iter().enumerate() {
        if token.text != "use" {
            continue;
        }
        let Some(statement_end) = tokens[use_index..]
            .iter()
            .position(|token| token.text == ";")
            .map(|offset| use_index + offset + 1)
        else {
            continue;
        };
        markers.use_ranges.push((use_index, statement_end));
        let (start, end) = if depths[use_index] == 0 {
            (0, tokens.len())
        } else {
            items
                .iter()
                .copied()
                .find(|(start, end)| *start <= use_index && use_index < *end)
                .unwrap_or((use_index, statement_end))
        };
        let mut index = use_index + 1;
        parse_use_tree(
            tokens,
            &mut index,
            statement_end,
            &[],
            use_index,
            start,
            end,
            &mut leaves,
        );
    }

    // A module alias can be consumed by another use declaration, so resolve
    // the leaves to a fixed point instead of depending on source order.
    loop {
        let mut changed = false;
        for leaf in &leaves {
            let Some(binding) = resolved_ast_binding(leaf, ast_types, &markers) else {
                continue;
            };
            changed |= markers.import_indices.insert(leaf.use_index);
            if let Some(binding) = binding
                && !markers.bindings.iter().any(|existing| {
                    existing.binding == binding
                        && existing.start == leaf.start
                        && existing.end == leaf.end
                })
            {
                markers.bindings.push(ScopedAstBinding {
                    binding,
                    start: leaf.start,
                    end: leaf.end,
                });
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    markers
}

#[allow(clippy::too_many_arguments)]
fn parse_use_tree(
    tokens: &[Token],
    index: &mut usize,
    statement_end: usize,
    prefix: &[String],
    use_index: usize,
    start: usize,
    end: usize,
    leaves: &mut Vec<UseLeaf>,
) {
    if *index >= statement_end {
        return;
    }
    if tokens[*index].text == "{" {
        *index += 1;
        while *index < statement_end && tokens[*index].text != "}" {
            parse_use_tree(
                tokens,
                index,
                statement_end,
                prefix,
                use_index,
                start,
                end,
                leaves,
            );
            if tokens.get(*index).is_some_and(|token| token.text == ",") {
                *index += 1;
            }
        }
        if tokens.get(*index).is_some_and(|token| token.text == "}") {
            *index += 1;
        }
        return;
    }
    if tokens[*index].text == "*" {
        *index += 1;
        leaves.push(UseLeaf {
            path: prefix.to_vec(),
            alias: None,
            glob: true,
            use_index,
            start,
            end,
        });
        return;
    }
    if !is_identifier(&tokens[*index].text) {
        *index += 1;
        return;
    }

    let mut path = prefix.to_vec();
    path.push(tokens[*index].text.clone());
    *index += 1;
    if tokens.get(*index).is_some_and(|token| token.text == "as") {
        *index += 1;
        let alias = tokens.get(*index).map(|token| token.text.clone());
        *index += usize::from(*index < statement_end);
        leaves.push(UseLeaf {
            path,
            alias,
            glob: false,
            use_index,
            start,
            end,
        });
        return;
    }
    if tokens.get(*index).is_some_and(|token| token.text == "::") {
        *index += 1;
        if *index < statement_end {
            parse_use_tree(
                tokens,
                index,
                statement_end,
                &path,
                use_index,
                start,
                end,
                leaves,
            );
            return;
        }
    }
    leaves.push(UseLeaf {
        path,
        alias: None,
        glob: false,
        use_index,
        start,
        end,
    });
}

fn resolved_ast_binding(
    leaf: &UseLeaf,
    ast_types: &BTreeSet<String>,
    markers: &AstUseMarkers,
) -> Option<Option<AstBinding>> {
    let mut path = leaf.path.as_slice();
    let mut module_default = None;
    let ast_path = if path.first().is_some_and(|segment| segment == "crate")
        && path.get(1).is_some_and(|segment| segment == "ast")
    {
        module_default = Some("ast");
        path = &path[2..];
        true
    } else if let Some(module) = path
        .first()
        .filter(|segment| markers.has_module(segment, leaf.use_index))
    {
        module_default = Some(module.as_str());
        path = &path[1..];
        true
    } else if path.first().is_some_and(|segment| segment == "crate") {
        path = &path[1..];
        path.is_empty() && leaf.glob
            || !leaf.glob && path.len() == 1 && ast_types.contains(&path[0])
    } else {
        false
    };
    if !ast_path {
        return None;
    }
    if leaf.alias.as_deref() == Some("_") {
        return Some(None);
    }
    if leaf.glob {
        return Some(Some(AstBinding::Glob));
    }

    let imports_self = path.last().is_some_and(|segment| segment == "self");
    if imports_self {
        path = &path[..path.len() - 1];
    }
    let name = leaf
        .alias
        .as_deref()
        .or_else(|| path.last().map(String::as_str))
        .or(module_default)?
        .to_owned();
    let item_name = path.last();
    if imports_self || path.is_empty() || item_name.is_none_or(|name| !ast_types.contains(name)) {
        Some(Some(AstBinding::Module(name)))
    } else {
        Some(Some(AstBinding::Name(name)))
    }
}

impl AstUseMarkers {
    fn in_use(&self, index: usize) -> bool {
        self.use_ranges
            .iter()
            .any(|(start, end)| *start <= index && index < *end)
    }

    fn has_glob(&self, index: usize) -> bool {
        self.bindings.iter().any(|binding| {
            binding.start <= index && index < binding.end && binding.binding == AstBinding::Glob
        })
    }

    fn has_name(&self, name: &str, index: usize) -> bool {
        self.bindings.iter().any(|binding| {
            binding.start <= index
                && index < binding.end
                && matches!(&binding.binding, AstBinding::Name(bound) if bound == name)
        })
    }

    fn has_module(&self, name: &str, index: usize) -> bool {
        self.bindings.iter().any(|binding| {
            binding.start <= index
                && index < binding.end
                && matches!(&binding.binding, AstBinding::Module(bound) if bound == name)
        })
    }
}

fn rooted_use_module(tokens: &[Token], module: &str) -> Option<usize> {
    let use_index = tokens.iter().position(|token| token.text == "use")?;
    let module_index = tokens
        .windows(3)
        .position(|tokens| token_texts(tokens) == ["crate", "::", module])?;
    (use_index < module_index
        && !tokens[..use_index]
            .iter()
            .any(|token| matches!(token.text.as_str(), "{" | ";")))
    .then_some(module_index)
}

fn item_ranges(tokens: &[Token]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0;
    let mut depth: usize = 0;
    for (index, token) in tokens.iter().enumerate() {
        match token.text.as_str() {
            "{" | "(" | "[" => depth += 1,
            "}" | ")" | "]" => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth == 0 && matches!(token.text.as_str(), ";" | "}") {
            ranges.push((start, index + 1));
            start = index + 1;
        }
    }
    if start < tokens.len() {
        ranges.push((start, tokens.len()));
    }
    ranges
}

fn top_level_token(tokens: &[Token], expected: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        if depth == 0 && token.text == expected {
            return Some(index);
        }
        match token.text.as_str() {
            "{" | "(" | "[" => depth += 1,
            "}" | ")" | "]" => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}

fn token_depths(tokens: &[Token]) -> Vec<usize> {
    let mut depth = 0usize;
    tokens
        .iter()
        .map(|token| {
            let current = depth;
            match token.text.as_str() {
                "{" | "(" | "[" => depth += 1,
                "}" | ")" | "]" => depth = depth.saturating_sub(1),
                _ => {}
            }
            current
        })
        .collect()
}

fn ast_semantic_reference_at(
    tokens: &[Token],
    index: usize,
    ast_types: &BTreeSet<String>,
    markers: &AstUseMarkers,
) -> bool {
    if markers.in_use(index) {
        return false;
    }
    let name = tokens[index].text.as_str();
    let path = path_prefix(tokens, index);
    if path.is_empty() {
        return markers.has_name(name, index)
            || ast_types.contains(name) && markers.has_glob(index);
    }
    if path.starts_with(&["crate", "ast"]) {
        return ast_types.contains(name);
    }
    if path == ["crate"] {
        return ast_types.contains(name);
    }
    if markers.has_module(path[0], index) {
        return ast_types.contains(name);
    }
    if matches!(path[0], "self" | "super") {
        return markers.has_name(name, index)
            || ast_types.contains(name) && markers.has_glob(index);
    }
    false
}

fn path_prefix(tokens: &[Token], index: usize) -> Vec<&str> {
    if index < 2 || tokens[index - 1].text != "::" {
        return Vec::new();
    }
    let mut cursor = index - 2;
    let mut path = Vec::new();
    loop {
        path.push(tokens[cursor].text.as_str());
        if cursor < 2 || tokens[cursor - 1].text != "::" {
            break;
        }
        cursor -= 2;
    }
    path.reverse();
    path
}

fn checker_path_at(tokens: &[Token], index: usize) -> bool {
    tokens.get(index).is_some_and(|token| token.text == "check")
        && index >= 2
        && sequence_at(tokens, index - 2, &["crate", "::", "check"])
}

fn method_call_at(tokens: &[Token], index: usize, method: &str) -> bool {
    tokens.get(index).is_some_and(|token| token.text == method)
        && index != 0
        && tokens[index - 1].text == "."
        && tokens.get(index + 1).is_some_and(|token| token.text == "(")
}

fn associated_call_at(tokens: &[Token], index: usize, ty: &str, method: &str) -> bool {
    sequence_at(tokens, index, &[ty, "::", method, "("])
}

fn function_call_at(tokens: &[Token], index: usize, function: &str) -> bool {
    tokens
        .get(index)
        .is_some_and(|token| token.text == function)
        && tokens.get(index + 1).is_some_and(|token| token.text == "(")
}

fn sequence_at(tokens: &[Token], index: usize, expected: &[&str]) -> bool {
    tokens
        .get(index..index.saturating_add(expected.len()))
        .is_some_and(|tokens| token_texts(tokens) == expected)
}

fn token_texts(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

fn normalize(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_identifier(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|first| first == '_' || first.is_alphabetic())
        && value
            .chars()
            .all(|character| character == '_' || character.is_alphanumeric())
}

fn fnv1a_128(bytes: &[u8]) -> u128 {
    const OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    const PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;
    bytes.iter().fold(OFFSET, |hash, byte| {
        (hash ^ u128::from(*byte)).wrapping_mul(PRIME)
    })
}

fn lex(source: &str) -> Vec<Token> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if bytes[index..].starts_with(b"//") {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            index = skip_block_comment(bytes, index);
            continue;
        }
        if let Some(end) = skip_literal(bytes, index) {
            index = end;
            continue;
        }
        let character = source[index..].chars().next().expect("source character");
        if character == '_' || character.is_alphabetic() {
            let start = index;
            index += character.len_utf8();
            while index < bytes.len() {
                let character = source[index..].chars().next().expect("source character");
                if character != '_' && !character.is_alphanumeric() {
                    break;
                }
                index += character.len_utf8();
            }
            tokens.push(Token {
                text: source[start..index].to_owned(),
            });
            continue;
        }
        let punctuation = [
            "::", "->", "=>", "..=", "..", "&&", "||", "==", "!=", "<=", ">=",
        ]
        .into_iter()
        .find(|punctuation| source[index..].starts_with(punctuation))
        .map(str::to_owned)
        .unwrap_or_else(|| character.to_string());
        tokens.push(Token {
            text: punctuation.clone(),
        });
        index += punctuation.len();
    }
    tokens
}

fn skip_block_comment(bytes: &[u8], mut index: usize) -> usize {
    let mut depth = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"/*") {
            depth += 1;
            index += 2;
        } else if bytes[index..].starts_with(b"*/") {
            depth -= 1;
            index += 2;
            if depth == 0 {
                break;
            }
        } else {
            index += 1;
        }
    }
    index
}

fn skip_literal(bytes: &[u8], index: usize) -> Option<usize> {
    let mut cursor = index;
    if matches!(bytes.get(cursor), Some(b'b' | b'c')) {
        cursor += 1;
    }
    if bytes.get(cursor) == Some(&b'r') {
        cursor += 1;
        let mut hashes = 0;
        while bytes.get(cursor) == Some(&b'#') {
            hashes += 1;
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'"') {
            return None;
        }
        cursor += 1;
        while cursor < bytes.len() {
            if bytes[cursor] == b'"'
                && bytes
                    .get(cursor + 1..cursor + 1 + hashes)
                    .is_some_and(|suffix| suffix.iter().all(|byte| *byte == b'#'))
            {
                return Some(cursor + 1 + hashes);
            }
            cursor += 1;
        }
        return Some(bytes.len());
    }
    let delimiter = *bytes.get(cursor)?;
    if delimiter != b'"' && delimiter != b'\'' {
        return None;
    }
    if delimiter == b'\''
        && bytes
            .get(cursor + 1)
            .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        && bytes.get(cursor + 2) != Some(&b'\'')
    {
        return None;
    }
    cursor += 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' {
            cursor += 2;
        } else if bytes[cursor] == delimiter {
            return Some(cursor + 1);
        } else {
            cursor += 1;
        }
    }
    Some(bytes.len())
}
