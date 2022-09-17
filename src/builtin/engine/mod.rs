pub mod annotator;
pub mod contributors;
pub mod entities;
pub mod parse_tree;

use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::num::ParseIntError;
use std::ops::Deref;
use std::rc::Rc;
use crate::builtin::engine::entities::Entity;
use crate::parser::ast::{ASTKind, Boxed, Identifier, NumberLiteral, PropertyCall, StringLiteral};
use crate::tui::settings::{ColorType, TUISettings};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    String,
    Number,
    Entity
}

#[derive(Debug, Clone)]
pub enum Value<'a> {
    String(String),
    Number(f64),
    Entity(Entity<'a>)
}

impl Display for Value<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Number(n) => write!(f, "{}", n),
            Value::Entity(e) => write!(f, "{:?}", e),
        }
    }
}

impl Value<'_> {

    fn my_type(&self) -> Type {
        match self {
            Value::String(_) => Type::String,
            Value::Number(_) => Type::Number,
            Value::Entity(_) => Type::Entity,
        }
    }

}

pub struct Argument {
    pub name: String,
    pub ty: Type,
    pub contributor: Box<dyn Contributor>
}

impl Into<Value<'_>> for f64 {
    fn into(self) -> Value {
        Value::Number(self)
    }
}

impl Into<Value<'_>> for String {
    fn into(self) -> Value {
        Value::String(self)
    }
}

impl <'a>Into<Value<'a>> for Entity<'a> {
    fn into(self) -> Value {
        Value::Entity(self)
    }
}



pub trait Contributor {
    fn contribute(&self, value: Value) -> Vec<Value>;
}

