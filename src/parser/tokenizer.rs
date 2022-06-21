use logos::{Lexer, Logos, Span};
use crate::parser::ast::{ASTKind};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Logos)]
pub enum TopLevelToken {
    #[token("&")]
    Ampersand,

    #[token("|")]
    Pipe,

    #[token(";")]
    SemiColon,

    #[token("$")]
    Dollar,

    #[regex("[^ |;&\n\t$\"}]+")]
    Literal,

    #[token("}")]
    RightBrace,

    #[token("\"")]
    DoubleQuote,

    #[regex("[ \n\t]+", logos::skip, priority = 1)]
    Whitespace,

    #[error]
    Error,

}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Logos)]
enum StringLevelToken {
    // Actually not possible as I can understand
    #[error]
    Error,

    #[token("\"")]
    DoubleQuote,

    #[regex("[^\"]+")]
    Literal,

}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Logos)]
enum FunctionLevelToken {
    #[error]
    Error,

    #[token("(")]
    LeftParen,

    #[token(")")]
    RightParen,

    #[token("{")]
    LeftBrace,

    #[token("}")]
    RightBrace,

    #[token(".")]
    Dot,

    #[token(",")]
    Comma,

    #[regex(r"[0-9]*(\.[0-9]+)?")]
    Number,

    #[token("\"")]
    DoubleQuote,

    #[regex(r#"[a-zA-Z](\w|[_0-9])*"#)]
    Identifier,

    #[token("&")]
    Ampersand,

    #[token("|")]
    Pipe,

    #[token(";")]
    SemiColon,

    #[regex("[ \n\t]+", logos::skip)]
    Whitespace,
}

impl Into<ASTKind> for StringLevelToken {
    fn into(self) -> ASTKind {
        match self {
            StringLevelToken::DoubleQuote => ASTKind::DoubleQuote,
            StringLevelToken::Literal => ASTKind::Literal,
            _ => ASTKind::Error,
        }
    }
}

impl Into<ASTKind> for FunctionLevelToken {
    fn into(self) -> ASTKind {
        match self {
            FunctionLevelToken::LeftParen => ASTKind::OpenParen,
            FunctionLevelToken::RightParen => ASTKind::CloseParen,
            FunctionLevelToken::LeftBrace => ASTKind::OpenBrace,
            FunctionLevelToken::RightBrace => ASTKind::CloseBrace,
            FunctionLevelToken::Dot => ASTKind::Dot,
            FunctionLevelToken::Comma => ASTKind::Comma,
            FunctionLevelToken::Number => ASTKind::NumberLiteral,
            FunctionLevelToken::Identifier => ASTKind::Identifier,
            FunctionLevelToken::Error => ASTKind::Error,
            FunctionLevelToken::Ampersand => ASTKind::Ampersand,
            FunctionLevelToken::Pipe => ASTKind::Pipe,
            FunctionLevelToken::SemiColon => ASTKind::SemiColon,
            FunctionLevelToken::DoubleQuote => ASTKind::DoubleQuote,
            FunctionLevelToken::Whitespace => panic!("Whitespace should not be in the function level tokenizer"),
        }
    }
}

impl Into<ASTKind> for TopLevelToken {
    fn into(self) -> ASTKind {
        match self {
            TopLevelToken::Ampersand => ASTKind::Ampersand,
            TopLevelToken::Pipe => ASTKind::Pipe,
            TopLevelToken::SemiColon => ASTKind::SemiColon,
            TopLevelToken::Dollar => ASTKind::Dollar,
            TopLevelToken::Error => ASTKind::Error,
            TopLevelToken::Literal => ASTKind::Literal,
            TopLevelToken::Whitespace => panic!("Whitespace should not be in the top level tokenizer"),
            TopLevelToken::RightBrace => ASTKind::CloseBrace,
            TopLevelToken::DoubleQuote => ASTKind::DoubleQuote,
        }
    }
}


enum TokenizerState<'a> {
    TopLevel(Lexer<'a, TopLevelToken>),
    FunctionLevel(Lexer<'a, FunctionLevelToken>),
    StringLevel(Lexer<'a, StringLevelToken>),
}

pub struct Tokenizer<'a> {
    offset: usize,
    stack: Vec<TokenizerState<'a>>,
    state: TokenizerState<'a>,
}

impl<'a> Tokenizer<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            offset: 0,
            stack: Default::default(),
            state: TokenizerState::TopLevel(TopLevelToken::lexer(s)),
        }
    }

    fn push_state(&mut self, new_state: TokenizerState<'a>) {
        let mut state = new_state;

        std::mem::swap(&mut state, &mut self.state);

        self.stack.push(state);
    }

    fn pop_state(&mut self, remainder: &'a str) {
        let state = self.stack.pop();

        if state.is_none() {
            self.state = TokenizerState::TopLevel(TopLevelToken::lexer(remainder));
        } else {
            let state = state.unwrap();

            self.state = match state {
                TokenizerState::TopLevel(_) => {
                    TokenizerState::TopLevel(TopLevelToken::lexer(remainder))
                }
                TokenizerState::FunctionLevel(_) => {
                    TokenizerState::FunctionLevel(FunctionLevelToken::lexer(remainder))
                }
                TokenizerState::StringLevel(_) => {
                    TokenizerState::StringLevel(StringLevelToken::lexer(remainder))
                }
            }
        }
    }
}

pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Spanned<ASTKind, usize, (usize, usize)>;

    fn next(&mut self) -> Option<Self::Item> {
        let (token, span) = match self.state {
            TokenizerState::TopLevel(ref mut lexer) => {
                (lexer.next().map(|token| token.into()), lexer.span())
            }
            TokenizerState::FunctionLevel(ref mut lexer) => {
                (lexer.next().map(|token| token.into()), lexer.span())
            }
            TokenizerState::StringLevel(ref mut lexer) => {
                (lexer.next().map(|token| token.into()), lexer.span())
            }
        };


        let slice = match &self.state {
            TokenizerState::TopLevel(l) => l.remainder(),
            TokenizerState::FunctionLevel(l) => l.remainder(),
            TokenizerState::StringLevel(l) => l.remainder(),
        };

        let span = span.start + self.offset..span.end + self.offset;

        if matches!(token, Some(ASTKind::Dollar)) {
            self.offset = span.end;
            self.state = TokenizerState::FunctionLevel(FunctionLevelToken::lexer(slice));
        }

        if matches!(token, Some(ASTKind::SemiColon) | Some(ASTKind::Pipe) | Some(ASTKind::Ampersand)) {
            if matches!(self.state, TokenizerState::FunctionLevel(_)) {
                self.offset = span.end;
                self.state = TokenizerState::TopLevel(TopLevelToken::lexer(slice));
            }
        }

        if matches!(token, Some(ASTKind::DoubleQuote)) {
            self.offset = span.end;
            if matches!(&self.state, TokenizerState::StringLevel(_)) {
                self.pop_state(slice)
            } else {
                self.push_state(TokenizerState::StringLevel(StringLevelToken::lexer(slice)));
            }
        }

        if matches!(token, Some(ASTKind::CloseBrace)) {
            self.offset = span.end;
            self.pop_state(slice)
        }

        if matches!(token, Some(ASTKind::OpenBrace)) {
            self.offset = span.end;
            self.push_state(TokenizerState::TopLevel(TopLevelToken::lexer(slice)))
        }

        match token {
            Some(v) => Some(Ok((span.start, v, span.end))),
            None => None
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::any::type_name;
    use std::fmt::Debug;
    pub use super::*;


    pub fn tokenize<'a, T: Logos<'a, Source=str, Extras=()>>(s: &'a str) -> Vec<ASTKind>
        where T: Logos<'a, Source=str, Extras=()>, T: Into<ASTKind>, T: Debug
    {
        let lexer = T::lexer(s);
        lexer
            .into_iter()
            .map(|t| t.into())
            .collect()
    }

    pub fn tokenize_function_level(s: &str) -> Vec<ASTKind> {
        let mut tokenizer = Tokenizer::new(s);
        tokenizer.state = TokenizerState::FunctionLevel(FunctionLevelToken::lexer(s));

        tokenizer.map(|e| e.unwrap().1).collect()
    }

    pub fn tokenize_top_level(s: &str) -> Vec<ASTKind> {
        let tokenizer = Tokenizer::new(s);
        tokenizer.map(|e| e.unwrap().1).collect()
    }

    pub fn tokenize_string_level(s: &str) -> Vec<ASTKind> {
        let mut tokenizer = Tokenizer::new(s);
        tokenizer.state = TokenizerState::StringLevel(StringLevelToken::lexer(s));

        tokenizer.map(|e| e.unwrap().1).collect()
    }

    fn token<'a, T: Logos<'a, Source=str, Extras=()>>(s: &'a str) -> ASTKind
        where T: Logos<'a, Source=str, Extras=()>, T: Into<ASTKind>, T: Debug
    {
        let tokens: Vec<ASTKind> = tokenize::<'a, T>(s);
        assert_eq!(tokens.len(), 1, "Expected one token, got {:?}", &tokens);
        let token = tokens.into_iter().next().unwrap();
        token.into()
    }

    fn expect_tokens<'a, T: Logos<'a, Source=str, Extras=()>>(s: &'a str, expected: &[ASTKind])
        where T: Logos<'a, Source=str, Extras=()>, T: Into<ASTKind>, T: Debug
    {
        let tokens: Vec<ASTKind> = tokenize::<'a, T>(s);

        assert_eq!(&tokens, expected);
    }


    macro_rules! expect_token_with_fuzz {
        ($t:ty,$s:expr, $expected:expr) => {{
            let actual = token::<$t>($s);
            assert_eq!(actual, $expected, "{}", $s);

            let v = concat!(" ", $s);
            let actual = token::<$t>(v);
            assert_eq!(actual, $expected, "{}", v);


            let v = concat!(" ", $s, " ");
            let actual = token::<$t>(v);
            assert_eq!(actual, $expected, "{}", v);
        }};
    }

    macro_rules! expect_top_level_token {
        ($s:expr, $expected:expr) => {
            expect_token_with_fuzz!(TopLevelToken, $s, $expected);
        };
    }

    macro_rules! expect_function_token {
        ($s:expr, $expected:expr) => {
            expect_token_with_fuzz!(FunctionLevelToken, $s, $expected);
        };
    }

    #[test]
    fn test_simple_tokens() {
        expect_top_level_token!("$", ASTKind::Dollar);
        expect_top_level_token!(";", ASTKind::SemiColon);
        expect_top_level_token!("&", ASTKind::Ampersand);
        expect_top_level_token!("|", ASTKind::Pipe);
        expect_top_level_token!("\"", ASTKind::DoubleQuote);
        expect_top_level_token!("}", ASTKind::CloseBrace);

        expect_function_token!("(", ASTKind::OpenParen);
        expect_function_token!(")", ASTKind::CloseParen);
        expect_function_token!("{", ASTKind::OpenBrace);
        expect_function_token!("}", ASTKind::CloseBrace);
        expect_function_token!(";", ASTKind::SemiColon);
        expect_function_token!("&", ASTKind::Ampersand);
        expect_function_token!("|", ASTKind::Pipe);
        expect_function_token!("\"", ASTKind::DoubleQuote);
        expect_function_token!("}", ASTKind::CloseBrace);
    }

    #[test]
    fn test_literal_token() {
        expect_top_level_token!("foo", ASTKind::Literal);
        expect_top_level_token!("foo.fda.fafahY8w", ASTKind::Literal);

        expect_tokens::<TopLevelToken>("fsjaf {", &[
            ASTKind::Literal,
            ASTKind::Literal
        ]);
    }


    fn expect_tokens_full(s: &str, expected: &[ASTKind]) {
        let tokenizer = Tokenizer::new(s);
        let tokens : Vec<ASTKind> = tokenizer.collect::<Result<Vec<_>, _>>()
            .unwrap()
            .into_iter()
            .map(|t| t.1)
            .collect();

        assert_eq!(expected, &tokens);
    }

    #[test]
    fn handmade() {
        expect_tokens_full("$foo()",
                           &[
                               ASTKind::Dollar,
                               ASTKind::Identifier,
                               ASTKind::OpenParen,
                               ASTKind::CloseParen
                           ],
        );
    }

    #[test]
    fn test_general_tokenizer() {
        let tokenizer = Tokenizer::new("echo 'hello world'");
        let tokens = tokenizer.collect::<Result<Vec<_>, _>>().unwrap();
        let expected = vec![
            (0, ASTKind::Literal, 4),
            (5, ASTKind::Literal, 11),
            (12, ASTKind::Literal, 18),
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_string_tokenization() {
        let tokenizer = Tokenizer::new(r#""fd d""#);
        let tokens = tokenizer.collect::<Result<Vec<_>, _>>().unwrap();

        let expected = vec![
            (0, ASTKind::DoubleQuote, 1),
            (1, ASTKind::Literal, 5),
            (5, ASTKind::DoubleQuote, 6),
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_braced_cmd() {
        let tokenizer = Tokenizer::new(r#"${kek}"#);
        let tokens = tokenizer.collect::<Result<Vec<_>, _>>().unwrap();

        let expected = vec![
            (0, ASTKind::Dollar, 1),
            (1, ASTKind::OpenBrace, 2),
            (2, ASTKind::Literal, 5),
            (5, ASTKind::CloseBrace, 6),
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_function_tokenizer() {
        let tokenizer = Tokenizer::new("$foo(1, 2, 3).lmao;");
        let tokens = tokenizer.collect::<Result<Vec<_>, _>>().unwrap();
        let expected = vec![
            (0, ASTKind::Dollar, 1),
            (1, ASTKind::Identifier, 4),
            (4, ASTKind::OpenParen, 5),
            (5, ASTKind::NumberLiteral, 6),
            (6, ASTKind::Comma, 7),
            (8, ASTKind::NumberLiteral, 9),
            (9, ASTKind::Comma, 10),
            (11, ASTKind::NumberLiteral, 12),
            (12, ASTKind::CloseParen, 13),
            (13, ASTKind::Dot, 14),
            (14, ASTKind::Identifier, 18),
            (18, ASTKind::SemiColon, 19),
        ];
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_uncompleted_str() {
        let tokenizer = Tokenizer::new(r#"$ "fdfdf"#);
        let tokens = tokenizer.collect::<Result<Vec<_>, _>>().unwrap();
        let expected = vec![
            (0, ASTKind::Dollar, 1), (2, ASTKind::DoubleQuote, 3), (3, ASTKind::Literal, 8),
        ];

        assert_eq!(tokens, expected);
    }
}