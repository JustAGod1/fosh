
use std::fmt::{Debug};
use downcast_rs::{Downcast, impl_downcast};
use lalrpop_util::ErrorRecovery;
use lalrpop_util::lexer::Token;
use termion::color::{Cyan, Fg, Green, Magenta, Yellow};
use crate::builtin::entity::Type;
use crate::completer::parse_tree::PTNode;

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

    pub fn find_child_with_kind(&self, kind: ASTKind) -> Option<&ASTNode> {
        if self.value.kind() == kind {
            return Some(self);
        }
        for child in self.children.iter() {
            let v = child.find_child_with_kind(kind);
            if v.is_some() { return v;}
        }
        return None;
    }


}



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ASTKind {
    CommandLine,
    Piped,
    Sequenced,

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
simple_token!(Command, ASTKind::Command);
simple_token!(CommandName, ASTKind::CommandName);
simple_token!(CommandArguments, ASTKind::CommandArguments);
simple_token!(Function, ASTKind::Function);
simple_token!(CommandLine, ASTKind::CommandLine);
simple_token!(Piped, ASTKind::Piped);
simple_token!(Sequenced, ASTKind::Sequenced);

#[derive(Debug)]
pub struct CallChain {

}

impl CallChain {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_left_hand<'a>(&self, node: &'a PTNode<'a>) -> Option<&'a PTNode<'a>> {
        return node
            .children()
            .get(0)
            .filter(|x| matches!(x.kind, ASTKind::CallChain | ASTKind::PropertyCall))
            .map(|x| *x);
    }

    pub fn get_right_hand<'a>(&self, node: &'a PTNode<'a>) -> Option<&'a PTNode<'a>> {
        return node.find_child_with_kind(ASTKind::PropertyCall)
    }
}

impl ASTValue for CallChain {
    fn kind(&self) -> ASTKind {
        ASTKind::CallChain
    }
}


#[derive(Debug)]
pub struct ASTError {
    pub expected: ASTKind,
    pub error: ErrorRecovery<usize, ASTKind, (usize, usize)>
}

impl ASTError {
    pub fn new(expected: ASTKind, error: ErrorRecovery<usize, ASTKind, (usize, usize)>) -> Self {
        Self { expected, error }
    }
}

impl ASTValue for ASTError {
    fn kind(&self) -> ASTKind {
        ASTKind::Error
    }
}
