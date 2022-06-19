
use std::fmt::{Debug};
use downcast_rs::{Downcast, impl_downcast};
use termion::color::{Cyan, Fg, Green, Magenta, Yellow};
use crate::builtin::entity::Type;

#[derive(Debug, Eq, PartialEq)]
pub struct Span {
    start: usize,
    end: usize
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn as_range(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }

    pub fn slice<'a>(&self, text: &'a str) -> &'a str {
        &text[self.start..self.end]
    }
}

#[derive(Debug)]
pub struct ASTNode {
    pub span: Span,
    pub value: Box<dyn ASTValue>,
    pub children: Vec<ASTNode>,
}

impl PartialEq for ASTNode {
    fn eq(&self, other: &Self) -> bool {
        if self.span != other.span { return false; }
        if self.value.kind() != other.value.kind() { return false; }
        if self.children != other.children { return false; }

        true
    }
}

impl Eq for ASTNode {}

impl ASTNode {

    pub fn new_simple<T: ASTValue>(l: usize, r: usize, value: T, children: Vec<ASTNode>) -> Self {
        return Self::new(Span::new(l, r), Box::new(value), children);
    }
    pub fn new(span: Span, value: Box<dyn ASTValue>, children: Vec<ASTNode>) -> Self {
        Self { span, value, children }
    }



}



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ASTKind {
    CommandLine,

    // General mode tokens
    Ampersand,
    Pipe,
    SemiColon,
    Dollar,

    // Function mode tokens
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    StringLiteral,
    NumberLiteral,
    Dot,
    Comma,
    Identifier,
    Literal,

    // Function mode non-terminals
    Function,
    ParenthesizedArgumentsList,
    PropertyCall,
    PropertyName,
    CallChain,

    // Command mode non-terminals
    Command,
    CommandName,
    CommandArguments,

    Error

}

impl ASTKind {
    // returns unique color for each kind
    pub fn color(&self, buf: &mut String)  {
        match self {
            ASTKind::Dollar => buf.push_str(&Fg(Cyan).to_string()),
            ASTKind::OpenParen => buf.push_str(&Fg(termion::color::LightMagenta).to_string()),
            ASTKind::CloseParen => buf.push_str(&Fg(termion::color::Blue).to_string()),
            _ => {}
        }
    }

    pub fn color_string(&self) -> String {
        let mut result = String::new();
        self.color(&mut result);

        result
    }

}


pub trait Boxed {
    fn boxed(self) -> Box<Self>;
}

impl<T> Boxed for T {
    fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}

pub trait ASTValue : Downcast + Debug{
    fn kind(&self) -> ASTKind;
}

impl_downcast!(ASTValue);

macro_rules! simple_token {
    ($name: ident, $kind: expr) => {
        #[derive(Debug)]
        pub struct $name {
        }

        impl ASTValue for $name {
            fn kind(&self) -> ASTKind {
                $kind
            }
        }

        impl $name {
            pub fn new() -> Self {
                Self {}
            }
        }
    };
}



simple_token!(Ampersand, ASTKind::Ampersand);
simple_token!(OpenParen, ASTKind::OpenParen);
simple_token!(CloseParen, ASTKind::CloseParen);
simple_token!(Literal, ASTKind::Literal);
simple_token!(OpenBrace, ASTKind::OpenBrace);
simple_token!(CloseBrace, ASTKind::CloseBrace);
simple_token!(Dot, ASTKind::Dot);
simple_token!(Comma, ASTKind::Comma);
simple_token!(SemiColon, ASTKind::SemiColon);
simple_token!(Dollar, ASTKind::Dollar);
simple_token!(Pipe, ASTKind::Pipe);
simple_token!(StringLiteral, ASTKind::StringLiteral);
simple_token!(NumberLiteral, ASTKind::NumberLiteral);
simple_token!(Identifier, ASTKind::Identifier);
simple_token!(ParenthesizedArgumentsList, ASTKind::ParenthesizedArgumentsList);
simple_token!(PropertyCall, ASTKind::PropertyCall);
simple_token!(PropertyName, ASTKind::PropertyName);
simple_token!(CallChain, ASTKind::CallChain);
simple_token!(Command, ASTKind::Command);
simple_token!(CommandName, ASTKind::CommandName);
simple_token!(CommandArguments, ASTKind::CommandArguments);
simple_token!(Function, ASTKind::Function);
simple_token!(CommandLine, ASTKind::CommandLine);


#[derive(Debug)]
pub struct ErroredASTValue {
    pub expected: ASTKind,
}

impl ErroredASTValue {
    pub fn new(expected: ASTKind) -> Self {
        Self { expected }
    }
}

impl ASTValue for ErroredASTValue {
    fn kind(&self) -> ASTKind {
        ASTKind::Error
    }
}
