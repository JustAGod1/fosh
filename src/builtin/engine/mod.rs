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
use crate::builtin::engine::entities::{Entity, FoshEntity, EntityExecutionError, EntityRef};
use crate::{entities, EntitiesManager};
use crate::parser::ast::{ASTKind, Boxed, Identifier, NumberLiteral, PropertyCall, StringLiteral};
use crate::ui::settings::{ColorType, TUISettings};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    String,
    Number,
    Entity
}

pub enum Value {
    String(String),
    Number(f64),
    Entity(EntityRef)
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Number(n) => write!(f, "{}", n),
            Value::Entity(e) => write!(f, "{}", e.borrow()),
        }
    }
}

impl Value {

    pub fn my_type(&self) -> Type {
        match self {
            Value::String(_) => Type::String,
            Value::Number(_) => Type::Number,
            Value::Entity(_) => Type::Entity,
        }
    }

    pub fn into_entity(self) -> EntityRef {
        match self {
            Value::Entity(e) => e,
            Value::String(s) => entities().make_entity(s.clone()).with_implicit(Type::String, move |e| s.clone()),
            Value::Number(n) => entities().make_entity(format!("{}", n)).with_implicit(Type::Number, move |e| n),
        }
    }


}

#[derive(Clone)]
pub struct Argument {
    pub name: String,
    pub possible_types: Vec<Type>,
    pub contributor: &'static dyn Contributor
}

impl Into<Value> for f64 {
    fn into(self) -> Value {
        Value::Number(self)
    }
}

impl Into<Value> for String {
    fn into(self) -> Value {
        Value::String(self)
    }
}

impl Into<Value> for EntityRef {
    fn into(self) -> Value {
        Value::Entity(self)
    }
}





