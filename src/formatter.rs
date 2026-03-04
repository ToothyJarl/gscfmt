use crate::lexer::{Token, TokenKind, is_control_kw, is_keyword, is_space_after_kw};

#[derive(Debug, Clone)]
pub struct FormatConfig {
    pub indent: String,
    pub max_blank_lines: usize,
}

impl Default for FormatConfig {
    fn default() -> Self {
        FormatConfig {
            indent: "    ".to_string(),
            max_blank_lines: 1,
        }
    }
}

pub struct Formatter {
    cfg: FormatConfig,
}

impl Formatter {
    pub fn new(cfg: FormatConfig) -> Self {
        Formatter { cfg }
    }

    pub fn format(&self, src: &str) -> String {
        let tokens = crate::lexer::tokenize(src);
        self.emit(&tokens)
    }

    fn emit(&self, tokens: &[Token]) -> String {
        let n = tokens.len();
        let mut out = String::with_capacity(n * 8);

        let mut depth: usize = 0;
        let mut paren_depth: usize = 0;
        let mut control_stack: Vec<usize> = Vec::new();
        let mut awaiting_body: bool = false;
        let mut braceless_saved_depth: Option<usize> = None;
        let mut prev_real: Option<&Token> = None;
        let mut last_real: Option<&Token> = None;
        let mut prev_was_rbrace_toplevel = false;
        let mut prev_was_preprocessor = false;

        let mut in_case_label: bool = false;
        let mut case_body_depth: Option<usize> = None;
        let mut case_open_braces: usize = 0;

        fn at_line_start(out: &str) -> bool {
            out.is_empty() || out.ends_with('\n')
        }

        fn strip_trailing_space(out: &mut String) {
            while out.ends_with(' ') || out.ends_with('\t') {
                out.pop();
            }
        }

        fn newline(out: &mut String) {
            strip_trailing_space(out);
            out.push('\n');
        }

        let next_real = |from: usize| -> Option<&Token> {
            let mut j = from + 1;
            while j < n {
                match tokens[j].kind {
                    TokenKind::LineComment | TokenKind::BlockComment => j += 1,
                    TokenKind::Eof => return None,
                    _ => return Some(&tokens[j]),
                }
            }
            None
        };

        fn needs_space(pprev: Option<&Token>, prev: &Token, cur: &Token) -> bool {
            let pk = &prev.kind;
            let pv = prev.text.as_str();
            let ck = &cur.kind;
            let cv = cur.text.as_str();

            if matches!(pk, TokenKind::Dot | TokenKind::DoubleColon | TokenKind::At) {
                return false;
            }

            if matches!(
                ck,
                TokenKind::Dot | TokenKind::DoubleColon | TokenKind::LBracket
            ) {
                if pk.is_binary_op() || *pk == TokenKind::Comma {
                    return true;
                }
                return false;
            }

            if matches!(ck, TokenKind::Comma | TokenKind::Semicolon) {
                return false;
            }

            if *ck == TokenKind::RParen {
                return false;
            }

            if *ck == TokenKind::LParen {
                if pk.is_binary_op() || *pk == TokenKind::Comma {
                    return true;
                }
                return *pk == TokenKind::Ident && is_control_kw(pv);
            }

            if matches!(ck, TokenKind::PlusPlus | TokenKind::MinusMinus)
                || matches!(pk, TokenKind::PlusPlus | TokenKind::MinusMinus)
            {
                return false;
            }

            if matches!(ck, TokenKind::Bang | TokenKind::Tilde) {
                return pk.is_binary_op() || *pk == TokenKind::Comma;
            }

            let is_unary = matches!(pk, TokenKind::Minus | TokenKind::Plus)
                && pprev.map_or(true, |pp| {
                    matches!(
                        pp.kind,
                        TokenKind::LParen
                            | TokenKind::LBracket
                            | TokenKind::Comma
                            | TokenKind::Semicolon
                            | TokenKind::Colon
                            | TokenKind::Assign
                            | TokenKind::Plus
                            | TokenKind::Minus
                            | TokenKind::Star
                            | TokenKind::Slash
                            | TokenKind::Percent
                    ) || (pp.kind == TokenKind::Ident
                        && (pp.text == "return"
                            || is_control_kw(&pp.text)
                            || is_space_after_kw(&pp.text)))
                });

            if is_unary && matches!(ck, TokenKind::Number | TokenKind::Ident | TokenKind::LParen) {
                return false;
            }

            if pk.is_binary_op() || ck.is_binary_op() {
                return true;
            }

            if *pk == TokenKind::Comma {
                return true;
            }

            if *pk == TokenKind::Ident && is_keyword(pv) {
                if !matches!(cv, ";" | "," | ")" | "]" | "(" | "{" | "}") {
                    return true;
                }
            }

            if *ck == TokenKind::Ident && is_keyword(cv) {
                if !matches!(pv, "(" | ")" | "[" | "]" | "{" | "." | "::") {
                    return true;
                }
            }

            if *pk == TokenKind::RBracket && *ck == TokenKind::Ident {
                return true;
            }

            let is_value = |k: &TokenKind| {
                matches!(
                    k,
                    TokenKind::Ident
                        | TokenKind::Number
                        | TokenKind::StringLit
                        | TokenKind::IString
                        | TokenKind::AnimRef
                )
            };
            if is_value(pk) && is_value(ck) {
                return true;
            }

            if pk.is_preprocessor() {
                return true;
            }

            false
        }

        let mut i = 0;
        while i < n {
            let tk = &tokens[i];

            if tk.kind == TokenKind::Eof {
                break;
            }

            let need_blank = {
                let tl_fn = depth == 0
                    && prev_was_rbrace_toplevel
                    && tk.kind == TokenKind::Ident
                    && next_real(i).map_or(false, |t| t.kind == TokenKind::LParen);

                let author_blank = tk.preceded_by_blank
                    && depth == 0
                    && !prev_was_rbrace_toplevel
                    && !prev_was_preprocessor;
                let inner_blank = tk.preceded_by_blank
                    && depth > 0
                    && at_line_start(&out)
                    && !prev_was_preprocessor;
                tl_fn || author_blank || inner_blank
            };

            if matches!(tk.kind, TokenKind::LineComment | TokenKind::BlockComment) {
                if at_line_start(&out) {
                    if need_blank {
                        out.push('\n');
                    }
                    out.push_str(&self.cfg.indent.repeat(depth));
                } else {
                    strip_trailing_space(&mut out);
                    out.push_str("  ");
                }
                out.push_str(&tk.text);
                newline(&mut out);
                i += 1;
                continue;
            }

            if tk.kind == TokenKind::RBrace {
                if case_body_depth.is_some() {
                    if case_open_braces > 0 {
                        case_open_braces -= 1;
                    } else {
                        if let Some(d) = case_body_depth.take() {
                            depth = d;
                        }
                        in_case_label = false;
                    }
                }

                if !at_line_start(&out) {
                    newline(&mut out);
                }
                if depth > 0 {
                    depth -= 1;
                }
                out.push_str(&self.cfg.indent.repeat(depth));
                out.push('}');
                prev_was_rbrace_toplevel = depth == 0;
                prev_was_preprocessor = false;
                newline(&mut out);
                prev_real = last_real;
                last_real = Some(tk);
                awaiting_body = false;

                i += 1;
                continue;
            }

            if tk.kind == TokenKind::LBrace {
                awaiting_body = false;
                braceless_saved_depth = None;

                if case_body_depth.is_some() {
                    case_open_braces += 1;
                }

                if !at_line_start(&out) {
                    newline(&mut out);
                }
                out.push_str(&self.cfg.indent.repeat(depth));
                out.push('{');
                depth += 1;
                prev_was_rbrace_toplevel = false;
                prev_was_preprocessor = false;
                newline(&mut out);
                prev_real = last_real;
                last_real = Some(tk);
                i += 1;
                continue;
            }

            if tk.kind.is_preprocessor() {
                if !at_line_start(&out) {
                    newline(&mut out);
                }
                if need_blank || prev_was_rbrace_toplevel {
                    out.push('\n');
                }
                out.push_str(&self.cfg.indent.repeat(depth));
                out.push_str(&tk.text);
                prev_real = last_real;
                last_real = Some(tk);
                prev_was_rbrace_toplevel = false;
                prev_was_preprocessor = true;
                i += 1;
                continue;
            }

            if tk.kind == TokenKind::Semicolon {
                strip_trailing_space(&mut out);
                out.push(';');
                if paren_depth == 0 {
                    newline(&mut out);
                    if let Some(saved) = braceless_saved_depth.take() {
                        depth = saved;
                        awaiting_body = false;
                    }
                } else {
                    let next_needs_space = next_real(i).map_or(false, |t| {
                        !matches!(t.kind, TokenKind::Semicolon | TokenKind::RParen)
                    });
                    if next_needs_space {
                        out.push(' ');
                    }
                }
                prev_real = last_real;
                last_real = Some(tk);
                prev_was_preprocessor = false;
                i += 1;
                continue;
            }

            if tk.kind == TokenKind::Colon {
                out.push(':');
                if in_case_label {
                    case_body_depth = Some(depth);
                    case_open_braces = 0;
                    depth += 1;
                    in_case_label = false;
                }
                newline(&mut out);
                prev_real = last_real;
                last_real = Some(tk);
                prev_was_preprocessor = false;
                i += 1;
                continue;
            }

            if tk.kind == TokenKind::LParen {
                paren_depth += 1;
            } else if tk.kind == TokenKind::RParen {
                strip_trailing_space(&mut out);

                let closes_control = control_stack.last().copied() == Some(paren_depth - 1);

                paren_depth -= 1;

                if closes_control {
                    control_stack.pop();
                    awaiting_body = true;
                }

                let space_before_rparen = last_real.map_or(false, |prev| {
                    !matches!(prev.kind, TokenKind::LParen | TokenKind::Semicolon)
                });

                if space_before_rparen {
                    out.push(' ');
                }
                out.push(')');
                prev_was_rbrace_toplevel = false;
                prev_was_preprocessor = false;
                prev_real = last_real;
                last_real = Some(tk);
                i += 1;
                continue;
            }

            if tk.is_ident("else") {
                if !at_line_start(&out) {
                    newline(&mut out);
                }
                out.push_str(&self.cfg.indent.repeat(depth));
                out.push_str("else");

                let nxt = next_real(i);
                let next_is_if = nxt.map_or(false, |t| t.is_ident("if"));
                let next_is_brace = nxt.map_or(false, |t| t.kind == TokenKind::LBrace);

                if next_is_if {
                } else if next_is_brace {
                    newline(&mut out);
                } else {
                    awaiting_body = true;
                    newline(&mut out);
                }

                prev_real = last_real;
                last_real = Some(tk);
                prev_was_rbrace_toplevel = false;
                prev_was_preprocessor = false;
                i += 1;
                continue;
            }

            if awaiting_body && !matches!(tk.kind, TokenKind::LineComment | TokenKind::BlockComment)
            {
                awaiting_body = false;
                braceless_saved_depth = Some(depth);
                depth += 1;

                if !at_line_start(&out) {
                    newline(&mut out);
                }
            }

            if tk.kind == TokenKind::Ident && (tk.text == "case" || tk.text == "default") {
                if let Some(d) = case_body_depth.take() {
                    depth = d;
                    case_open_braces = 0;
                }
                in_case_label = true;
            }

            if tk.kind == TokenKind::Ident && is_control_kw(&tk.text) {
                control_stack.push(paren_depth);
            }

            if at_line_start(&out) {
                if need_blank {
                    out.push('\n');
                }
                out.push_str(&self.cfg.indent.repeat(depth));
                out.push_str(&tk.text);
            } else {
                let space_after_lparen = last_real.map_or(false, |prev| {
                    prev.kind == TokenKind::LParen
                        && !matches!(tk.kind, TokenKind::RParen | TokenKind::Semicolon)
                });

                if space_after_lparen {
                    out.push(' ');
                } else if let Some(prev) = last_real {
                    if needs_space(prev_real, prev, tk) {
                        out.push(' ');
                    }
                }
                out.push_str(&tk.text);
            }

            prev_real = last_real;
            last_real = Some(tk);
            prev_was_rbrace_toplevel = false;
            prev_was_preprocessor = false;
            i += 1;
        }

        finalize(out, self.cfg.max_blank_lines)
    }
}

fn finalize(src: String, max_blank_lines: usize) -> String {
    let lines: Vec<&str> = src.split('\n').collect();
    let mut result: Vec<String> = Vec::with_capacity(lines.len());
    let mut consecutive_blank: usize = 0;

    for line in &lines {
        let trimmed = line.trim_end().to_string();
        if trimmed.is_empty() {
            consecutive_blank += 1;
            if consecutive_blank <= max_blank_lines {
                result.push(trimmed);
            }
        } else {
            consecutive_blank = 0;
            result.push(trimmed);
        }
    }

    while result.last().map_or(false, |l: &String| l.is_empty()) {
        result.pop();
    }

    let mut out = result.join("\n");
    out.push('\n');
    out
}
