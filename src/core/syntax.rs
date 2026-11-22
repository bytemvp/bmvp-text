use crate::ui::theme::Theme;
use gpui::Hsla;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxToken {
    pub text: String,
    pub token_type: TokenType,
}

const MAX_HIGHLIGHT_CACHE_ENTRIES: usize = 1024;

/// 按行内容 + 语言缓存词法分析结果，避免每帧重复扫描可见行
#[derive(Default)]
pub struct HighlightCache {
    entries: HashMap<(u64, &'static str), (String, Vec<SyntaxToken>)>,
}

impl HighlightCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn hash_line(line: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        line.hash(&mut hasher);
        hasher.finish()
    }

    pub fn get_or_highlight(&mut self, line: &str, lang: &'static str) -> &[SyntaxToken] {
        let key = (Self::hash_line(line), lang);
        let hit = self
            .entries
            .get(&key)
            .is_some_and(|(cached_line, _)| cached_line == line);
        if !hit {
            if self.entries.len() >= MAX_HIGHLIGHT_CACHE_ENTRIES {
                self.entries.clear();
            }
            let tokens = highlight_line(line, lang);
            self.entries.insert(key, (line.to_string(), tokens));
        }
        &self.entries[&key].1
    }
}

const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while",
];

const GENERAL_KEYWORDS: &[&str] = &[
    "function",
    "class",
    "var",
    "let",
    "const",
    "def",
    "import",
    "from",
    "export",
    "default",
    "if",
    "else",
    "for",
    "while",
    "return",
    "null",
    "true",
    "false",
    "this",
    "new",
    "try",
    "catch",
    "throw",
    "switch",
    "case",
    "break",
    "continue",
    "typeof",
    "package",
    "public",
    "private",
    "protected",
    "static",
    "interface",
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
            while idx < len
                && (chars[idx].is_ascii_alphanumeric() || (chars[idx] == '.' && !has_dot))
            {
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
        if "+-*/%=<>!&|^~?:;.,()[]{}#".contains(ch) {
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
            if c.is_alphabetic()
                || c.is_ascii_digit()
                || c == '_'
                || c == '"'
                || c == '\''
                || c == '/'
                || c == '#'
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
        } else if idx < len {
            // 兜底消费 1 字符，保证游标严格递增，防止死循环
            tokens.push(SyntaxToken {
                text: chars[idx].to_string(),
                token_type: TokenType::Default,
            });
            idx += 1;
        }
    }

    tokens
}

pub fn token_color(token_type: TokenType, theme: &Theme) -> Hsla {
    match token_type {
        TokenType::Keyword => theme.syntax.keyword,
        TokenType::Type => theme.syntax.type_name,
        TokenType::Function => theme.syntax.function,
        TokenType::StringLiteral => theme.syntax.string,
        TokenType::NumberLiteral => theme.syntax.number,
        TokenType::Comment => theme.syntax.comment,
        TokenType::Operator => theme.syntax.operator,
        TokenType::Default => theme.syntax.default,
    }
}

#[cfg(test)]
mod tests {
    use super::{HighlightCache, SyntaxToken, TokenType, highlight_line};

    fn joined_text(tokens: &[SyntaxToken]) -> String {
        tokens.iter().map(|t| t.text.as_str()).collect()
    }

    #[test]
    fn test_highlight_rust_hash_no_infinite_loop() {
        for line in ["#[derive(Debug, Clone)]", "#![allow(dead_code)]"] {
            let tokens = highlight_line(line, "Rust");
            assert!(!tokens.is_empty());
            assert_eq!(joined_text(&tokens), line);
            assert_eq!(tokens[0].text, "#");
            assert_eq!(tokens[0].token_type, TokenType::Operator);
        }
    }

    #[test]
    fn test_highlight_c_and_markdown_hash_no_infinite_loop() {
        let c_line = "#include <stdio.h>";
        let c_tokens = highlight_line(c_line, "C");
        assert_eq!(joined_text(&c_tokens), c_line);
        assert_eq!(c_tokens[0].token_type, TokenType::Operator);

        let md_line = "# Heading";
        let md_tokens = highlight_line(md_line, "Markdown");
        assert_eq!(joined_text(&md_tokens), md_line);
        assert_eq!(md_tokens[0].token_type, TokenType::Operator);
    }

    #[test]
    fn test_highlight_hash_comment_languages() {
        for lang in ["Python", "TOML", "Shell"] {
            let line = "x = 1 # comment";
            let tokens = highlight_line(line, lang);
            assert_eq!(joined_text(&tokens), line);
            assert_eq!(
                tokens.last().map(|t| (t.text.as_str(), t.token_type)),
                Some(("# comment", TokenType::Comment))
            );
        }
    }

    #[test]
    fn test_highlight_unicode_non_ascii_digits_no_infinite_loop() {
        let line = "let x = x² + ０ + ½; // 中文";
        let tokens = highlight_line(line, "Rust");
        assert_eq!(joined_text(&tokens), line);
        assert_eq!(
            tokens.last().map(|t| (t.text.as_str(), t.token_type)),
            Some(("// 中文", TokenType::Comment))
        );
    }

    #[test]
    fn test_highlight_cache_hit_and_clear() {
        let mut cache = HighlightCache::new();
        assert!(cache.is_empty());

        let t1 = cache.get_or_highlight("fn main() {}", "Rust").to_vec();
        assert_eq!(cache.len(), 1);
        let t2 = cache.get_or_highlight("fn main() {}", "Rust").to_vec();
        assert_eq!(t1, t2);
        assert_eq!(cache.len(), 1);

        let _ = cache.get_or_highlight("def main(): pass", "Python");
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_highlight_escaped_strings_chars_and_multilang() {
        let escaped = r#"let s = "a\"b\\c"; let ch = 'x';"#;
        let rust_tokens = highlight_line(escaped, "Rust");
        assert_eq!(joined_text(&rust_tokens), escaped);
        assert!(
            rust_tokens
                .iter()
                .any(|t| t.text == r#""a\"b\\c""# && t.token_type == TokenType::StringLiteral)
        );
        assert!(
            rust_tokens
                .iter()
                .any(|t| t.text == "'x'" && t.token_type == TokenType::StringLiteral)
        );

        let call_line = "let item: MyType = compute_value(123, 0xFF) + 1;";
        let call_tokens = highlight_line(call_line, "Rust");
        assert_eq!(joined_text(&call_tokens), call_line);
        assert!(
            call_tokens
                .iter()
                .any(|t| t.text == "MyType" && t.token_type == TokenType::Type)
        );
        assert!(
            call_tokens
                .iter()
                .any(|t| t.text == "compute_value" && t.token_type == TokenType::Function)
        );
        assert!(
            call_tokens
                .iter()
                .any(|t| t.text == "123" && t.token_type == TokenType::NumberLiteral)
        );

        for (lang, line, expected_comment) in [
            ("JavaScript", "const f = (a) => a + 1; // js", "// js"),
            ("C++", "auto v = std::min(1, 2); // cpp", "// cpp"),
            ("TOML", "name = \"bmvp\" # pkg", "# pkg"),
            ("JSON", "{\"key\": 42}", ""),
            ("Shell", "echo \"hi\" # sh", "# sh"),
        ] {
            let tokens = highlight_line(line, lang);
            assert_eq!(joined_text(&tokens), line);
            if !expected_comment.is_empty() {
                assert_eq!(
                    tokens.last().map(|t| (t.text.as_str(), t.token_type)),
                    Some((expected_comment, TokenType::Comment))
                );
            }
        }
    }
}
