use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use downcast_rs::{Downcast, impl_downcast};
use termion::color::{Color, Cyan, Fg, Green, Magenta, Yellow};

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

pub struct ASTNode {
    pub span: Span,
    pub value: Box<dyn ASTValue>,
    pub children: Vec<ASTNode>,
}

impl ASTNode {

    pub fn new(span: Span, value: Box<dyn ASTValue>, children: Vec<ASTNode>) -> Self {
        Self { span, value, children }
    }

    pub fn walk<F>(&self, visitor: &mut F) where F : FnMut(&ASTNode) {
        visitor(self);
        for child in &self.children {
            child.walk(visitor);
        }
    }


    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ASTKind {
    Ampersand,
    FunctionName,
    FunctionCall,
    OpenParen,
    CloseParen,
    ParenInvocation,
    Literal,
    CommandName,
    Command,
    ValueLiteral,
    Comma,
    Error

}

impl ASTKind {
    // returns unique color for each kind
    pub fn color(&self, buf: &mut String)  {
        match self {
            ASTKind::Ampersand => buf.push_str(&Fg(Cyan).to_string()),
            ASTKind::FunctionName => buf.push_str(&Fg(Yellow).to_string()),
            ASTKind::OpenParen => buf.push_str(&Fg(termion::color::LightMagenta).to_string()),
            ASTKind::CloseParen => buf.push_str(&Fg(termion::color::Blue).to_string()),
            ASTKind::CommandName => buf.push_str(&Fg(Magenta).to_string()),
            ASTKind::ValueLiteral => buf.push_str(&Fg(Green).to_string()),
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
simple_token!(FunctionName, ASTKind::FunctionName);
simple_token!(FunctionCall, ASTKind::FunctionCall);
simple_token!(OpenParen, ASTKind::OpenParen);
simple_token!(CloseParen, ASTKind::CloseParen);
simple_token!(ParenInvocation, ASTKind::ParenInvocation);
simple_token!(Literal, ASTKind::Literal);
simple_token!(CommandName, ASTKind::CommandName);
simple_token!(Command, ASTKind::Command);
simple_token!(ValueLiteral, ASTKind::ValueLiteral);
simple_token!(Comma, ASTKind::Comma);

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
