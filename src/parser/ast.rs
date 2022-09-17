use std::fmt::{Debug, Display};
use std::rc::Rc;
use downcast_rs::{Downcast, impl_downcast};
use lalrpop_util::ErrorRecovery;
use lalrpop_util::lexer::Token;
use termion::color::{Bg, Cyan, Fg, Green, LightGreen, LightMagenta, LightYellow, Magenta, Red, Yellow};
use crate::builtin::engine::entities::EntitiesManager;
use crate::builtin::engine::parse_tree::PTNode;
use crate::builtin::engine::Value;

#[derive(Debug, Eq, PartialEq)]
pub struct Span {
    start: usize,
    end: usize,
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
            if v.is_some() { return v; }
        }
        return None;
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ASTKind {
    Piped,
    Sequenced,
    Delimited,

    // General mode tokens
    Ampersand,
    Pipe,
    SemiColon,
    Dollar,

    // Special mode tokens
    DoubleQuote,
    Literal,

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
    Equals,
    VariableName,

    // Function mode non-terminals
    Function,
    ParenthesizedArgumentsList,
    PropertyCall,
    PropertyName,
    Assignation,
    BracedCommand,

    // Command mode non-terminals
    Command,
    CommandName,
    CommandArguments,

    Error,
}

impl Display for ASTKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}


impl ASTKind {
    // returns unique color for each kind
    pub fn color(&self, buf: &mut String) {
        match self {
            ASTKind::Dollar => buf.push_str(&Fg(Yellow).to_string()),
            ASTKind::Pipe => buf.push_str(&Fg(Cyan).to_string()),
            ASTKind::Ampersand => buf.push_str(&Fg(Cyan).to_string()),
            ASTKind::SemiColon => buf.push_str(&Fg(Cyan).to_string()),
            ASTKind::PropertyName => buf.push_str(&Fg(LightYellow).to_string()),
            ASTKind::CommandName => buf.push_str(&Fg(LightGreen).to_string()),
            ASTKind::StringLiteral => buf.push_str(&Fg(Green).to_string()),
            ASTKind::VariableName => buf.push_str(&Fg(Magenta).to_string()),
            ASTKind::Error => buf.push_str(&Bg(Red).to_string()),
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

pub trait ASTValue: Downcast + Debug {
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
simple_token!(Piped, ASTKind::Piped);
simple_token!(Sequenced, ASTKind::Sequenced);
simple_token!(Delimited, ASTKind::Delimited);
simple_token!(BracedCommand, ASTKind::BracedCommand);
simple_token!(Assignation, ASTKind::Assignation);
simple_token!(Equals, ASTKind::Equals);
simple_token!(VariableName, ASTKind::VariableName);

pub trait Typed {
    fn infer_value<'a>(&self, pt: &'a PTNode<'a>, entities: &EntitiesManager) -> Option<Result<Value, String>>;
}

impl PropertyCall {
    pub fn get_property_name<'a>(&self, node: &'a PTNode<'a>) -> Option<&'a str> {
        return node.find_child_with_kind(ASTKind::PropertyName).map(|x| x.data);
    }

    pub fn get_arguments<'a>(&self, node: &'a PTNode<'a>) -> Vec<&'a PTNode<'a>> {
        let node = node
            .find_child_with_kind(ASTKind::ParenthesizedArgumentsList);

        if node.is_none() { return vec![]; }
        let node = node.unwrap();


        let result = node
            .children()
            .iter()
            .filter(|x| { matches!(x.kind, ASTKind::StringLiteral | ASTKind::NumberLiteral | ASTKind::Identifier) })
            .map(|x| *x)
            .collect();

        result
    }

    pub fn left_hand<'a>(&self, pt: &'a PTNode<'a>) -> Option<&'a PTNode<'a>> {
        let first = pt.children().get(0).unwrap().clone();
        if first.kind != ASTKind::PropertyName {
            return Some(first);
        }
        None
    }

}

impl Identifier {
    pub fn get_value<'a>(&self, node: &'a PTNode<'a>) -> Value {
        return Value::String(node.data.to_string());
    }
}

#[derive(Debug)]
pub struct ASTError {
    pub expected: Box<dyn ASTValue>,
    pub error: Option<ErrorRecovery<usize, ASTKind, (usize, usize)>>,
}

impl ASTError {
    pub fn new<T : ASTValue>(expected: T, error: ErrorRecovery<usize, ASTKind, (usize, usize)>) -> Self {
        Self { expected: Box::new(expected), error: Some(error) }
    }
    pub fn new_artificial<T : ASTValue>(expected: T) -> Self {
        Self { expected: Box::new(expected), error: None }
    }
}

impl ASTValue for ASTError {
    fn kind(&self) -> ASTKind {
        ASTKind::Error
    }
}

pub fn downcast_to_typed<'a>(pt: &'a PTNode<'a>) -> Option<&'a dyn Typed> {
    match pt.kind {
        ASTKind::StringLiteral => Some(pt.value::<StringLiteral>()),
        ASTKind::NumberLiteral => Some(pt.value::<NumberLiteral>()),
        ASTKind::Function => Some(pt.value::<Function>()),
        ASTKind::PropertyCall => Some(pt.value::<PropertyCall>()),
        ASTKind::Delimited => Some(pt.value::<Delimited>()),
        ASTKind::Piped => Some(pt.value::<Piped>()),
        ASTKind::Sequenced => Some(pt.value::<Sequenced>()),
        ASTKind::BracedCommand => Some(pt.value::<BracedCommand>()),
        _ => None,
    }
}

impl Typed for StringLiteral {
    fn infer_value<'a>(&self, pt: &'a PTNode<'a>, _entities: &EntitiesManager) -> Option<Result<Value, String>> {
        let result = if pt.data.ends_with("\"") && pt.data.len() > 1 {
            Value::String((&pt.data[1..pt.data.len() - 1]).to_string())
        } else {
            Value::String((&pt.data[1..]).to_string())
        };

        return Some(Ok(result));
    }
}
impl Typed for NumberLiteral {
    fn infer_value<'a>(&self, pt: &'a PTNode<'a>, _entities: &EntitiesManager) -> Option<Result<Value, String>> {
        Some(pt.data.parse::<f64>()
            .map_err(|e| e.to_string())
            .map(|x| Value::Number(x)))
    }
}
impl Typed for Function {
    fn infer_value<'a>(&self, pt: &'a PTNode<'a>, entities: &EntitiesManager) -> Option<Result<Value, String>> {
        let v = downcast_to_typed(pt.children()[1])
            .map(|x| x.infer_value(pt.children()[1], entities));

        if v.is_none() { return None;}
        return v.unwrap();
    }
}
impl Typed for PropertyCall {
    fn infer_value<'a>(&self, pt: &'a PTNode<'a>, entities: &EntitiesManager) -> Option<Result<Value, String>> {
        let v = pt.value::<PropertyCall>();

        match v.left_hand(pt) {
            None => entities.global().clone(),
            Some(left_hand) => {
                let typed = downcast_to_typed(left_hand);
                if typed.is_none() { return None; }
                let typed = typed.unwrap();
                
                let left_value = typed.infer_value(left_hand, entities);
                if left_value.is_none() { return None; }
                let left_value = left_value.unwrap();
                
                
            }
        }
        
        return None;
        
    }
}
impl Typed for BracedCommand {}
impl Typed for Delimited {}
impl Typed for Sequenced {}
impl Typed for Piped {}
