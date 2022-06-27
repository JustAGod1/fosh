pub mod annotator;
mod entities;
mod contributors;


use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::num::ParseIntError;
use std::ops::Deref;
use std::rc::Rc;
use crate::annotator::parse_tree::PTNode;
use crate::builtin::entities::Cd;
use crate::parser::ast::{ASTKind, Boxed, Identifier, NumberLiteral, PropertyCall, StringLiteral};
use crate::tui::settings::{ColorType, TUISettings};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    String,
    Number,
    Entity
}

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Number(f64),
    Entity(Rc<dyn Entity>)
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Number(n) => write!(f, "{}", n),
            Value::Entity(e) => write!(f, "{}", e),
        }
    }
}

impl Value {

    fn my_type(&self) -> Type {
        match self {
            Value::String(_) => Type::String,
            Value::Number(_) => Type::Number,
            Value::Entity(_) => Type::Entity,
        }
    }

}

pub struct Argument {
    name: String,
    ty: Type,
    contributor: Box<dyn Contributor>
}

pub trait Contributor {
    fn contribute(&self, value: Value) -> Vec<Value>;
}

pub trait EntityData {
    
}