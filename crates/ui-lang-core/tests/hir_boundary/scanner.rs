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
            let Some(mut index) = item.iter().position(|token| token.text == "pub") else {
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
        for index in 0..tokens.len() {
            let text = tokens[index].text.as_str();
            if ast_import_at(&tokens, index) {
                record(
                    &mut by_category,
                    "source AST import",
                    file,
                    &tokens,
                    &items,
                    index,
                );
            }
            if ast_types.contains(text) && !qualified_by_non_ast_path(&tokens, index) {
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
                if text == token && !qualified_by_non_ast_path(&tokens, index) {
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
            let Some(use_index) = item.iter().position(|token| token.text == "use") else {
                continue;
            };
            let Some(check) = item
                .windows(3)
                .position(|tokens| token_texts(tokens) == ["crate", "::", "check"])
            else {
                continue;
            };
            if use_index > check
                || item[..use_index]
                    .iter()
                    .any(|token| matches!(token.text.as_str(), "{" | ";"))
            {
                continue;
            }
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

fn ast_import_at(tokens: &[Token], index: usize) -> bool {
    sequence_at(tokens, index.saturating_sub(2), &["crate", "::", "ast"])
        && tokens.get(index).is_some_and(|token| token.text == "ast")
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

fn qualified_by_non_ast_path(tokens: &[Token], index: usize) -> bool {
    if index < 2 || tokens[index - 1].text != "::" {
        return false;
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
    if path.contains(&"ast") {
        return false;
    }
    if path.iter().any(|part| matches!(*part, "hir" | "lower")) {
        return true;
    }
    !path
        .last()
        .is_some_and(|root| matches!(*root, "crate" | "self" | "super"))
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
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && value
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
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
