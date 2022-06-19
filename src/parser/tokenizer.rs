use logos::{Lexer, Logos, Span};
use crate::parser::ast::{ASTKind};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Logos)]
enum TopLevelToken {
    #[token("&")]
    Ampersand,

    #[token("|")]
    Pipe,

    #[token(";")]
    SemiColon,

    #[token("$")]
    Dollar,

    #[regex("[^ |;&\n\t$]+")]
    Literal,

    #[regex("[ \n\t]+", logos::skip, priority = 1)]
    Whitespace,

    #[error]
    Error,

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

    #[regex(r#""[^"]*""#)]
    String,

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
            FunctionLevelToken::String => ASTKind::StringLiteral,
            FunctionLevelToken::Identifier => ASTKind::Identifier,
            FunctionLevelToken::Error => ASTKind::Error,
            FunctionLevelToken::Ampersand => ASTKind::Ampersand,
            FunctionLevelToken::Pipe => ASTKind::Pipe,
            FunctionLevelToken::SemiColon => ASTKind::SemiColon,
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
        }
    }
}


enum TokenizerState<'a> {
    TopLevel(Lexer<'a, TopLevelToken>),
    FunctionLevel(Lexer<'a, FunctionLevelToken>),
}

pub struct Tokenizer<'a> {
    offset: usize,
    state: TokenizerState<'a>,
}

impl<'a> Tokenizer<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            offset: 0,
            state: TokenizerState::TopLevel(TopLevelToken::lexer(s))
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
        };


        if matches!(token, Some(ASTKind::Ampersand)) {
            return Some(Err((span.start, span.end)));
        }
        let slice = match &self.state {
            TokenizerState::TopLevel(l) => l.remainder(),
            TokenizerState::FunctionLevel(l) => l.remainder()
        };

        let span =  span.start + self.offset..span.end + self.offset;

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

        match token {
            Some(v) => Some(Ok((span.start, v, span.end))),
            None => None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}