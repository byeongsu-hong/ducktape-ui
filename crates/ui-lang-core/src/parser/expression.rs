use super::*;

pub(in crate::parser) fn parse_type(source: &str, line: &Line) -> Result<Type, Error> {
    let source = source.trim();
    if let Some(inner) = source.strip_suffix('?') {
        return Ok(Type::Option(Box::new(parse_type(inner, line)?)));
    }
    if let Some(inner) = source
        .strip_prefix("result[")
        .and_then(|source| source.strip_suffix(']'))
    {
        let parts = split_top(inner, ',');
        if parts.len() != 2 {
            return Err(error(
                "E023",
                line,
                "result type uses `result[Output,Error]`",
            ));
        }
        return Ok(Type::Result(
            Box::new(parse_type(parts[0].trim(), line)?),
            Box::new(parse_type(parts[1].trim(), line)?),
        ));
    }
    if let Some(inner) = source
        .strip_prefix("combo[")
        .and_then(|source| source.strip_suffix(']'))
    {
        return Ok(Type::Combo(Box::new(parse_type(inner, line)?)));
    }
    if let Some(inner) = source
        .strip_prefix("animation[")
        .and_then(|source| source.strip_suffix(']'))
    {
        return Ok(Type::Animation(Box::new(parse_type(inner, line)?)));
    }
    if let Some(contract) = source
        .strip_prefix("palette[")
        .and_then(|source| source.strip_suffix(']'))
    {
        return Ok(Type::Palette(qualified_identifier(contract.trim(), line)?));
    }
    if source.starts_with('[') && source.ends_with(']') {
        return Ok(Type::List(Box::new(parse_type(
            &source[1..source.len() - 1],
            line,
        )?)));
    }
    Ok(match source {
        "bool" => Type::Bool,
        "i64" => Type::I64,
        "f64" => Type::F64,
        "str" => Type::Str,
        "bytes" => Type::Bytes,
        "image" => Type::Image,
        "image-allocation" => Type::ImageAllocation,
        "image-memory" => Type::ImageMemory,
        "image-error" => Type::ImageError,
        "debug-span" => Type::DebugSpan,
        "markdown" => Type::Markdown,
        "editor" => Type::Editor,
        "event" => Type::Event,
        "event-status" => Type::EventStatus,
        "key" => Type::Key,
        "physical-key" => Type::PhysicalKey,
        "key-location" => Type::KeyLocation,
        "key-modifiers" => Type::KeyModifiers,
        "pixels" => Type::Pixels,
        "padding" => Type::Padding,
        "degrees" => Type::Degrees,
        "radians" => Type::Radians,
        "rotation" => Type::Rotation,
        "content-fit" => Type::ContentFit,
        "color" => Type::Color,
        "background" => Type::Background,
        "gradient" => Type::Gradient,
        "linear-gradient" => Type::LinearGradient,
        "color-stop" => Type::ColorStop,
        "font" => Type::Font,
        "font-family" => Type::FontFamily,
        "font-weight" => Type::FontWeight,
        "font-stretch" => Type::FontStretch,
        "font-style" => Type::FontStyle,
        "theme-mode" => Type::ThemeMode,
        "text-alignment" => Type::TextAlignment,
        "text-shaping" => Type::TextShaping,
        "text-wrapping" => Type::TextWrapping,
        "text-line-height" => Type::TextLineHeight,
        "length" => Type::Length,
        "alignment" => Type::Alignment,
        "horizontal-alignment" => Type::HorizontalAlignment,
        "vertical-alignment" => Type::VerticalAlignment,
        "border" => Type::Border,
        "radius" => Type::Radius,
        "shadow" => Type::Shadow,
        "point" => Type::Point,
        "point-u32" => Type::PointU32,
        "vector" => Type::Vector,
        "size" => Type::Size,
        "size-u32" => Type::SizeU32,
        "rectangle" => Type::Rectangle,
        "rectangle-u32" => Type::RectangleU32,
        "transformation" => Type::Transformation,
        "mouse-interaction" => Type::MouseInteraction,
        "scroll-delta" => Type::ScrollDelta,
        "mouse-button" => Type::MouseButton,
        "mouse-cursor" => Type::MouseCursor,
        "mouse-click" => Type::MouseClick,
        "touch-finger" => Type::TouchFinger,
        "instant" => Type::Instant,
        "window-id" => Type::WindowId,
        "window-screenshot" => Type::WindowScreenshot,
        "window-position" => Type::WindowPosition,
        "redraw-request" => Type::RedrawRequest,
        "window-direction" => Type::WindowDirection,
        "window-level" => Type::WindowLevel,
        "window-mode" => Type::WindowMode,
        "window-attention" => Type::WindowAttention,
        "widget-id" => Type::WidgetId,
        "widget-target" => Type::WidgetTarget,
        "task-handle" => Type::TaskHandle,
        "unit" => Type::Unit,
        value
            if value
                .rsplit("::")
                .next()
                .and_then(|name| name.chars().next())
                .is_some_and(char::is_uppercase) =>
        {
            Type::Named(line.qualify(&qualified_identifier(value, line)?))
        }
        _ => return Err(error("E023", line, format!("unknown type `{source}`"))),
    })
}

pub(in crate::parser) fn parse_expr(source: &str, line: &Line) -> Result<Expr, Error> {
    let line = line.origin_for(source);
    ExprParser::new(source, line)?.parse()
}

pub(in crate::parser) fn parse_hex_bytes(
    source: &str,
    line: &Line,
    code: &'static str,
) -> Result<Vec<u8>, Error> {
    source
        .split_whitespace()
        .map(|byte| {
            (byte.len() == 2)
                .then(|| u8::from_str_radix(byte, 16).ok())
                .flatten()
                .ok_or_else(|| error(code, line, "bytes use two hex digits per byte"))
        })
        .collect()
}

pub(in crate::parser) fn parse_expr_list(source: &str, line: &Line) -> Result<Vec<Expr>, Error> {
    if source.trim().is_empty() {
        return Ok(Vec::new());
    }
    split_top(source, ',')
        .into_iter()
        .map(|part| parse_expr(part.trim(), line))
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Ident(String),
    Str(String),
    Integer(String),
    F64(f64),
    Bytes(Vec<u8>),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Namespace,
    Not,
    Neg,
    Plus,
    Star,
    Slash,
    Percent,
    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

struct ExprParser<'a> {
    tokens: Vec<Token>,
    index: usize,
    line: &'a Line,
}

impl<'a> ExprParser<'a> {
    fn new(source: &str, line: &'a Line) -> Result<Self, Error> {
        Ok(Self {
            tokens: lex_expr(source, line)?,
            index: 0,
            line,
        })
    }

    fn parse(mut self) -> Result<Expr, Error> {
        let expr = self.binary(0)?;
        if self.index != self.tokens.len() {
            return Err(error("E070", self.line, "unexpected token in expression"));
        }
        Ok(expr)
    }

    fn binary(&mut self, min_precedence: u8) -> Result<Expr, Error> {
        let mut left = self.unary()?;
        while let Some((op, precedence)) = self.binary_op() {
            if precedence < min_precedence {
                break;
            }
            self.index += 1;
            let right = self.binary(precedence + 1)?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, Error> {
        if self.peek() == Some(&Token::Not) {
            self.index += 1;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                value: Box::new(self.unary()?),
            });
        }
        if self.peek() == Some(&Token::Neg) {
            self.index += 1;
            if matches!(
                self.peek(),
                Some(Token::Integer(value)) if value == "9223372036854775808"
            ) {
                self.index += 1;
                return Ok(Expr::I64(i64::MIN));
            }
            let value = self.unary()?;
            if matches!(value, Expr::I64(i64::MIN)) {
                return Err(error("E070", self.line, "integer is out of range"));
            }
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                value: Box::new(value),
            });
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, Error> {
        let token = self
            .next()
            .ok_or_else(|| error("E070", self.line, "expected expression"))?;
        match token {
            Token::Str(value) => Ok(Expr::Str(value)),
            Token::Integer(value) => {
                Ok(Expr::I64(value.parse().map_err(|_| {
                    error("E070", self.line, "invalid integer")
                })?))
            }
            Token::F64(value) => Ok(Expr::F64(value)),
            Token::Bytes(value) => Ok(Expr::Bytes(value)),
            Token::LBracket => {
                if self.peek() == Some(&Token::RBracket) {
                    self.index += 1;
                    return Ok(Expr::EmptyList);
                }
                let mut values = Vec::new();
                loop {
                    values.push(self.binary(0)?);
                    if self.peek() == Some(&Token::Comma) {
                        self.index += 1;
                    } else {
                        break;
                    }
                }
                if self.next() != Some(Token::RBracket) {
                    return Err(error("E070", self.line, "missing closing `]`"));
                }
                Ok(Expr::List(values))
            }
            Token::LParen => {
                let value = self.binary(0)?;
                if self.next() != Some(Token::RParen) {
                    return Err(error("E070", self.line, "missing closing `)`"));
                }
                Ok(value)
            }
            Token::Ident(name) if name == "true" => Ok(Expr::Bool(true)),
            Token::Ident(name) if name == "false" => Ok(Expr::Bool(false)),
            Token::Ident(name) if name == "none" => Ok(Expr::None),
            Token::Ident(name) => {
                let mut qualified = name;
                while self.peek() == Some(&Token::Namespace) {
                    self.index += 1;
                    match self.next() {
                        Some(Token::Ident(segment)) => {
                            qualified.push_str("::");
                            qualified.push_str(&segment);
                        }
                        _ => return Err(error("E070", self.line, "expected name after `::`")),
                    }
                }
                let mut path = vec![qualified];
                while self.peek() == Some(&Token::Dot) {
                    self.index += 1;
                    match self.next() {
                        Some(Token::Ident(field)) => path.push(field),
                        _ => return Err(error("E070", self.line, "expected name after `.`")),
                    }
                }
                if self.peek() == Some(&Token::LParen) {
                    self.index += 1;
                    let mut args = Vec::new();
                    if self.peek() != Some(&Token::RParen) {
                        loop {
                            args.push(self.binary(0)?);
                            if self.peek() == Some(&Token::Comma) {
                                self.index += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    if self.next() != Some(Token::RParen) {
                        return Err(error("E070", self.line, "missing closing `)`"));
                    }
                    return Ok(Expr::Call {
                        name: self.line.qualify(&path.join(".")),
                        args,
                    });
                }
                if path[0]
                    .rsplit("::")
                    .next()
                    .and_then(|name| name.chars().next())
                    .is_some_and(char::is_uppercase)
                {
                    path[0] = self.line.qualify(&path[0]);
                }
                Ok(Expr::Path(path))
            }
            _ => Err(error("E070", self.line, "invalid expression")),
        }
    }

    fn binary_op(&self) -> Option<(BinaryOp, u8)> {
        Some(match self.peek()? {
            Token::Or => (BinaryOp::Or, 1),
            Token::And => (BinaryOp::And, 2),
            Token::EqEq => (BinaryOp::Eq, 3),
            Token::NotEq => (BinaryOp::NotEq, 3),
            Token::Lt => (BinaryOp::Lt, 4),
            Token::LtEq => (BinaryOp::LtEq, 4),
            Token::Gt => (BinaryOp::Gt, 4),
            Token::GtEq => (BinaryOp::GtEq, 4),
            Token::Plus => (BinaryOp::Add, 5),
            Token::Neg => (BinaryOp::Sub, 5),
            Token::Star => (BinaryOp::Mul, 6),
            Token::Slash => (BinaryOp::Div, 6),
            Token::Percent => (BinaryOp::Rem, 6),
            _ => return None,
        })
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn next(&mut self) -> Option<Token> {
        let value = self.tokens.get(self.index).cloned();
        self.index += usize::from(value.is_some());
        value
    }
}

fn lex_expr(source: &str, line: &Line) -> Result<Vec<Token>, Error> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }
        if ch == '"' {
            index += 1;
            let mut value = String::new();
            while index < chars.len() && chars[index] != '"' {
                if chars[index] == '\\' {
                    index += 1;
                    let escaped = *chars
                        .get(index)
                        .ok_or_else(|| error("E070", line, "unfinished string escape"))?;
                    value.push(match escaped {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '"' => '"',
                        '\\' => '\\',
                        _ => {
                            return Err(error(
                                "E070",
                                line,
                                format!("unsupported string escape `\\{escaped}`"),
                            ));
                        }
                    });
                } else {
                    value.push(chars[index]);
                }
                index += 1;
            }
            if chars.get(index) != Some(&'"') {
                return Err(error("E070", line, "unterminated string"));
            }
            index += 1;
            tokens.push(Token::Str(value));
            continue;
        }
        if chars[index..].starts_with(&['b', 'y', 't', 'e', 's', '(']) {
            let start = index + 6;
            let end = chars[start..]
                .iter()
                .position(|ch| *ch == ')')
                .map(|offset| start + offset)
                .ok_or_else(|| error("E070", line, "missing closing `)` after bytes"))?;
            let source = chars[start..end].iter().collect::<String>();
            tokens.push(Token::Bytes(parse_hex_bytes(&source, line, "E070")?));
            index = end + 1;
            continue;
        }
        if ch.is_ascii_digit() {
            let start = index;
            index += 1;
            while index < chars.len() && (chars[index].is_ascii_digit() || chars[index] == '.') {
                index += 1;
            }
            let mut float = chars[start..index].contains(&'.');
            if chars.get(index).is_some_and(|ch| matches!(ch, 'e' | 'E')) {
                float = true;
                index += 1;
                if chars.get(index).is_some_and(|ch| matches!(ch, '+' | '-')) {
                    index += 1;
                }
                while chars.get(index).is_some_and(char::is_ascii_digit) {
                    index += 1;
                }
            }
            let value: String = chars[start..index].iter().collect();
            if float {
                let value = value
                    .parse::<f64>()
                    .map_err(|_| error("E070", line, "invalid float"))?;
                if !value.is_finite() {
                    return Err(error("E070", line, "float is out of range"));
                }
                tokens.push(Token::F64(value));
            } else {
                tokens.push(Token::Integer(value));
            }
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
            {
                index += 1;
            }
            tokens.push(Token::Ident(chars[start..index].iter().collect()));
            continue;
        }
        let next = chars.get(index + 1).copied();
        let (token, width) = match (ch, next) {
            ('=', Some('=')) => (Token::EqEq, 2),
            ('!', Some('=')) => (Token::NotEq, 2),
            ('<', Some('=')) => (Token::LtEq, 2),
            ('>', Some('=')) => (Token::GtEq, 2),
            ('&', Some('&')) => (Token::And, 2),
            ('|', Some('|')) => (Token::Or, 2),
            (':', Some(':')) => (Token::Namespace, 2),
            ('(', _) => (Token::LParen, 1),
            (')', _) => (Token::RParen, 1),
            ('[', _) => (Token::LBracket, 1),
            (']', _) => (Token::RBracket, 1),
            (',', _) => (Token::Comma, 1),
            ('.', _) => (Token::Dot, 1),
            ('!', _) => (Token::Not, 1),
            ('-', _) => (Token::Neg, 1),
            ('+', _) => (Token::Plus, 1),
            ('*', _) => (Token::Star, 1),
            ('/', _) => (Token::Slash, 1),
            ('%', _) => (Token::Percent, 1),
            ('<', _) => (Token::Lt, 1),
            ('>', _) => (Token::Gt, 1),
            _ => return Err(error("E070", line, format!("unexpected character `{ch}`"))),
        };
        tokens.push(token);
        index += width;
    }
    Ok(tokens)
}
