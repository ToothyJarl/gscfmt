#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Semicolon,
    Comma,
    Dot,
    Colon,
    DoubleColon,
    At,

    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    Tilde,
    Amp,
    Pipe,
    Caret,
    Lt,
    Gt,
    Question,

    Eq,
    NotEq,
    LtEq,
    GtEq,
    And,
    Or,
    PlusPlus,
    MinusMinus,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    AmpEq,
    PipeEq,
    CaretEq,
    LShift,
    RShift,

    Ident,
    Number,
    StringLit, // "..."
    IString,   // &"..."
    AnimRef,   // %ident

    LineComment,  // // ...
    BlockComment, // /* ... */

    HashInclude,
    HashUsingAnimtree,
    HashAnimtree,
    HashDefine,
    HashOther,

    Eof,
}

impl TokenKind {
    pub fn is_binary_op(&self) -> bool {
        matches!(
            self,
            TokenKind::Assign
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::Amp
                | TokenKind::Pipe
                | TokenKind::Caret
                | TokenKind::Lt
                | TokenKind::Gt
                | TokenKind::Question
                | TokenKind::Eq
                | TokenKind::NotEq
                | TokenKind::LtEq
                | TokenKind::GtEq
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::PlusEq
                | TokenKind::MinusEq
                | TokenKind::StarEq
                | TokenKind::SlashEq
                | TokenKind::PercentEq
                | TokenKind::AmpEq
                | TokenKind::PipeEq
                | TokenKind::CaretEq
                | TokenKind::LShift
                | TokenKind::RShift
        )
    }

    pub fn is_preprocessor(&self) -> bool {
        matches!(
            self,
            TokenKind::HashInclude
                | TokenKind::HashUsingAnimtree
                | TokenKind::HashAnimtree
                | TokenKind::HashDefine
                | TokenKind::HashOther
        )
    }

    pub fn is_unary_prefix_only(&self) -> bool {
        matches!(self, TokenKind::Bang | TokenKind::Tilde)
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub line: usize,
    pub preceded_by_blank: bool,
}

impl Token {
    pub fn is_ident(&self, word: &str) -> bool {
        self.kind == TokenKind::Ident && self.text == word
    }
}

pub const KEYWORDS: &[&str] = &[
    "if",
    "else",
    "for",
    "while",
    "foreach",
    "do",
    "switch",
    "case",
    "default",
    "break",
    "continue",
    "return",
    "wait",
    "waittill",
    "waittillmatch",
    "waittillframeend",
    "waittilltimeout",
    "endon",
    "notify",
    "thread",
    "childthread",
    "in",
    "undefined",
    "true",
    "false",
    "game",
    "self",
    "anim",
    "level",
    "new",
    "delete",
    "call",
    "prof_begin",
    "prof_end",
    "breakpoint",
];

pub const CONTROL_KW: &[&str] = &["if", "for", "while", "foreach", "switch"];

pub const SPACE_AFTER_KW: &[&str] = &[
    "return",
    "wait",
    "waittill",
    "waittillmatch",
    "waittilltimeout",
    "endon",
    "notify",
    "waittillframeend",
    "thread",
    "childthread",
    "case",
    "in",
    "new",
    "delete",
];

pub fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

pub fn is_control_kw(s: &str) -> bool {
    CONTROL_KW.contains(&s)
}

pub fn is_space_after_kw(s: &str) -> bool {
    SPACE_AFTER_KW.contains(&s)
}

pub fn tokenize(src: &str) -> Vec<Token> {
    let src = src.replace("\r\n", "\n").replace('\r', "\n");
    let chars: Vec<char> = src.chars().collect();
    let n = chars.len();
    let mut i = 0;
    let mut line = 1usize;
    let mut tokens: Vec<Token> = Vec::new();
    let mut pending_blank = false;

    macro_rules! peek {
        ($off:expr) => {
            chars.get(i + $off).copied().unwrap_or('\0')
        };
    }

    while i < n {
        let ch = chars[i];

        if ch == ' ' || ch == '\t' {
            i += 1;
            continue;
        }

        if ch == '\n' {
            let mut nl = 0usize;
            while i < n && chars[i] == '\n' {
                line += 1;
                i += 1;
                nl += 1;
            }
            if nl > 1 {
                pending_blank = true;
            }
            continue;
        }

        let token_line = line;
        let pb = pending_blank;
        pending_blank = false;

        if ch == '/' && peek!(1) == '/' {
            let start = i;
            i += 2;
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::LineComment,
                text: chars[start..i].iter().collect(),
                line: token_line,
                preceded_by_blank: pb,
            });
            continue;
        }

        if ch == '/' && peek!(1) == '*' {
            let start = i;
            i += 2;
            while i < n && !(chars[i] == '*' && peek!(1) == '/') {
                if chars[i] == '\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 2; // consume */
            tokens.push(Token {
                kind: TokenKind::BlockComment,
                text: chars[start..i].iter().collect(),
                line: token_line,
                preceded_by_blank: pb,
            });
            continue;
        }

        if ch == '#' {
            let start = i;
            i += 1;
            while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let kind = match text.as_str() {
                "#include" => TokenKind::HashInclude,
                "#using_animtree" => TokenKind::HashUsingAnimtree,
                "#animtree" => TokenKind::HashAnimtree,
                "#define" => TokenKind::HashDefine,
                _ => TokenKind::HashOther,
            };
            tokens.push(Token {
                kind,
                text,
                line: token_line,
                preceded_by_blank: pb,
            });
            continue;
        }

        if ch == '&' && peek!(1) == '"' {
            let start = i;
            i += 2;
            while i < n {
                match chars[i] {
                    '\\' => i += 2,
                    '"' => {
                        i += 1;
                        break;
                    }
                    _ => i += 1,
                }
            }
            tokens.push(Token {
                kind: TokenKind::IString,
                text: chars[start..i].iter().collect(),
                line: token_line,
                preceded_by_blank: pb,
            });
            continue;
        }

        if ch == '"' {
            let start = i;
            i += 1;
            while i < n {
                match chars[i] {
                    '\\' => i += 2,
                    '"' => {
                        i += 1;
                        break;
                    }
                    _ => i += 1,
                }
            }
            tokens.push(Token {
                kind: TokenKind::StringLit,
                text: chars[start..i].iter().collect(),
                line: token_line,
                preceded_by_blank: pb,
            });
            continue;
        }

        if ch == '%' && (peek!(1).is_alphabetic() || peek!(1) == '_') {
            let start = i;
            i += 1;
            while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::AnimRef,
                text: chars[start..i].iter().collect(),
                line: token_line,
                preceded_by_blank: pb,
            });
            continue;
        }

        if ch.is_ascii_digit() || (ch == '.' && peek!(1).is_ascii_digit()) {
            let start = i;
            if ch == '0' && (peek!(1) == 'x' || peek!(1) == 'X') {
                i += 2;
                while i < n && (chars[i].is_ascii_hexdigit() || chars[i] == '_') {
                    i += 1;
                }
            } else {
                while i < n
                    && (chars[i].is_ascii_digit()
                        || chars[i] == '.'
                        || chars[i] == '_'
                        || chars[i] == 'e'
                        || chars[i] == 'E')
                {
                    i += 1;
                }
            }
            tokens.push(Token {
                kind: TokenKind::Number,
                text: chars[start..i].iter().collect(),
                line: token_line,
                preceded_by_blank: pb,
            });
            continue;
        }

        if ch.is_alphabetic() || ch == '_' {
            let start = i;
            while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Ident,
                text: chars[start..i].iter().collect(),
                line: token_line,
                preceded_by_blank: pb,
            });
            continue;
        }

        let two: String = chars[i..n.min(i + 2)].iter().collect();
        if let Some((kind, len)) = match two.as_str() {
            "::" => Some((TokenKind::DoubleColon, 2)),
            "==" => Some((TokenKind::Eq, 2)),
            "!=" => Some((TokenKind::NotEq, 2)),
            "<=" => Some((TokenKind::LtEq, 2)),
            ">=" => Some((TokenKind::GtEq, 2)),
            "&&" => Some((TokenKind::And, 2)),
            "||" => Some((TokenKind::Or, 2)),
            "++" => Some((TokenKind::PlusPlus, 2)),
            "--" => Some((TokenKind::MinusMinus, 2)),
            "+=" => Some((TokenKind::PlusEq, 2)),
            "-=" => Some((TokenKind::MinusEq, 2)),
            "*=" => Some((TokenKind::StarEq, 2)),
            "/=" => Some((TokenKind::SlashEq, 2)),
            "%=" => Some((TokenKind::PercentEq, 2)),
            "&=" => Some((TokenKind::AmpEq, 2)),
            "|=" => Some((TokenKind::PipeEq, 2)),
            "^=" => Some((TokenKind::CaretEq, 2)),
            "<<" => Some((TokenKind::LShift, 2)),
            ">>" => Some((TokenKind::RShift, 2)),
            _ => None,
        } {
            tokens.push(Token {
                kind,
                text: two[..len].to_string(),
                line: token_line,
                preceded_by_blank: pb,
            });
            i += len;
            continue;
        }

        let kind = match ch {
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ';' => TokenKind::Semicolon,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            ':' => TokenKind::Colon,
            '\\' => TokenKind::At,
            '@' => TokenKind::At,
            '=' => TokenKind::Assign,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '!' => TokenKind::Bang,
            '~' => TokenKind::Tilde,
            '&' => TokenKind::Amp,
            '|' => TokenKind::Pipe,
            '^' => TokenKind::Caret,
            '<' => TokenKind::Lt,
            '>' => TokenKind::Gt,
            '?' => TokenKind::Question,
            _ => TokenKind::Ident,
        };
        tokens.push(Token {
            kind,
            text: ch.to_string(),
            line: token_line,
            preceded_by_blank: pb,
        });
        i += 1;
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        text: String::new(),
        line,
        preceded_by_blank: false,
    });

    tokens
}
