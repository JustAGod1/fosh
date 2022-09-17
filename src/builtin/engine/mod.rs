pub mod annotator;
pub mod contributors;
pub mod entities;
pub mod parse_tree;

use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::num::ParseIntError;
use std::ops::Deref;
use std::rc::Rc;
use crate::builtin::engine::contributors::Contributor;
use crate::builtin::engine::entities::Entity;
use crate::parser::ast::{ASTKind, Boxed, Identifier, NumberLiteral, PropertyCall, StringLiteral};
use crate::tui::settings::{ColorType, TUISettings};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    String,
    Number,
    Entity
}

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
            Value::Entity(e) => write!(f, "{}", e),
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

#[derive(Clone)]
pub struct Argument<'a> {
    pub name: String,
    pub ty: Type,
    pub contributor: &'a dyn Contributor
}

impl Into<Value<'static>> for f64 {
    fn into(self) -> Value<'static> {
        Value::Number(self)
    }
}

impl Into<Value<'static>> for String {
    fn into(self) -> Value<'static> {
        Value::String(self)
    }
}

impl <'a>Into<Value<'a>> for Entity<'a> {
    fn into(self) -> Value<'a> {
        Value::Entity(self)
    }
}




