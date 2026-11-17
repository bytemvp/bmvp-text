use gpui::*;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Default,
    Keyword,
    Type,
    Function,
    StringLiteral,
    NumberLiteral,
    Comment,
    Operator,
}

pub struct SyntaxToken {
    pub text: String,
    pub token_type: TokenType,
}

const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
    "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
    "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
    "trait", "true", "type", "unsafe", "use", "where", "while",
];

const GENERAL_KEYWORDS: &[&str] = &[
    "function", "class", "var", "let", "const", "def", "import", "from", "export",
    "default", "if", "else", "for", "while", "return", "null", "true", "false", "this",
    "new", "try", "catch", "throw", "switch", "case", "break", "continue", "typeof",
    "package", "public", "private", "protected", "static", "interface",
];

pub fn highlight_line(line: &str, lang: &str) -> Vec<SyntaxToken> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut idx = 0;

    let is_hash_comment = lang == "Python" || lang == "TOML" || lang == "Shell";

    while idx < len {
        let ch = chars[idx];

        // 1. 注释检测
        if (!is_hash_comment && ch == '/' && idx + 1 < len && chars[idx + 1] == '/')
            || (is_hash_comment && ch == '#')
        {
            let comment_text: String = chars[idx..].iter().collect();
            tokens.push(SyntaxToken {
                text: comment_text,
                token_type: TokenType::Comment,
            });
            break;
        }

        // 2. 字符串检测 ("..." 或 '...')
        if ch == '"' || ch == '\'' {
            let quote = ch;
            let mut s = String::new();
            s.push(quote);
            idx += 1;
            while idx < len {
                let c = chars[idx];
                s.push(c);
                if c == '\\' && idx + 1 < len {
                    idx += 1;
                    s.push(chars[idx]);
                } else if c == quote {
                    idx += 1;
                    break;
                }
                idx += 1;
            }
            tokens.push(SyntaxToken {
                text: s,
                token_type: TokenType::StringLiteral,
            });
            continue;
        }

        // 3. 数字字面量
        if ch.is_ascii_digit() {
            let mut num_str = String::new();
            let mut has_dot = false;
            while idx < len && (chars[idx].is_ascii_alphanumeric() || (chars[idx] == '.' && !has_dot)) {
                if chars[idx] == '.' {
                    has_dot = true;
                }
                num_str.push(chars[idx]);
                idx += 1;
            }
            tokens.push(SyntaxToken {
                text: num_str,
                token_type: TokenType::NumberLiteral,
            });
            continue;
        }

        // 4. 标识符 / 关键字 / 类型
        if ch.is_alphabetic() || ch == '_' {
            let mut word = String::new();
            while idx < len && (chars[idx].is_alphanumeric() || chars[idx] == '_') {
                word.push(chars[idx]);
                idx += 1;
            }

            // 判断是否是函数调用 (后跟 '(')
            let mut lookahead = idx;
            while lookahead < len && chars[lookahead].is_whitespace() {
                lookahead += 1;
            }
            let is_call = lookahead < len && chars[lookahead] == '(';

            let is_kw = if lang == "Rust" {
                RUST_KEYWORDS.contains(&word.as_str())
            } else {
                GENERAL_KEYWORDS.contains(&word.as_str())
            };

            let token_type = if is_kw {
                TokenType::Keyword
            } else if word.starts_with(char::is_uppercase) {
                TokenType::Type
            } else if is_call {
                TokenType::Function
            } else {
                TokenType::Default
            };

            tokens.push(SyntaxToken {
                text: word,
                token_type,
            });
            continue;
        }

        // 5. 操作符与标点符号
        if "+-*/%=<>!&|^~?:;.,()[]{}".contains(ch) {
            tokens.push(SyntaxToken {
                text: ch.to_string(),
                token_type: TokenType::Operator,
            });
            idx += 1;
            continue;
        }

        // 6. 其他普通字符 / 空格
        let mut def_str = String::new();
        while idx < len {
            let c = chars[idx];
            if c.is_alphanumeric() || c == '_' || c == '"' || c == '\'' || c == '/' || c == '#'
                || "+-*/%=<>!&|^~?:;.,()[]{}".contains(c)
            {
                break;
            }
            def_str.push(c);
            idx += 1;
        }
        if !def_str.is_empty() {
            tokens.push(SyntaxToken {
                text: def_str,
                token_type: TokenType::Default,
            });
        }
    }

    tokens
}

pub fn token_color(token_type: TokenType, theme: &Theme) -> Hsla {
    match token_type {
        TokenType::Keyword => rgb(0xec7279).into(),       // Zed keyword pink/red
        TokenType::Type => rgb(0xe5c07b).into(),          // Warm gold
        TokenType::Function => rgb(0x61afef).into(),      // Soft blue
        TokenType::StringLiteral => rgb(0x98c379).into(), // Leaf green
        TokenType::NumberLiteral => rgb(0xd19a66).into(), // Warm orange
        TokenType::Comment => theme.text_muted,           // Dimmed gray
        TokenType::Operator => theme.text_secondary,      // Muted light gray
        TokenType::Default => theme.text_primary,         // Default primary text
    }
}
