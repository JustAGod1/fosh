pub mod annotator;
mod entities;
mod contributors;


use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use std::ops::Deref;
use std::rc::Rc;
use crate::annotator::parse_tree::PTNode;
use crate::builtin::entities::Cd;
use crate::parser::ast::{ASTKind, Boxed, CallChain, Identifier, NumberLiteral, PropertyCall, StringLiteral};
use crate::tui::settings::{ColorType, TUISettings};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    String,
    Number,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
}

impl Eq for Value {}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Number(n) => write!(f, "{}", n),
        }
    }
}

impl Value {

    fn my_type(&self) -> Type {
        match self {
            Value::String(_) => Type::String,
            Value::Number(_) => Type::Number,
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

pub trait Entity : Display {
    fn args(&self) -> &[Argument];
    fn get_properties(&self) -> &HashMap<String, Rc<dyn Entity>>;
    fn call(&self, args: &Vec<Value>) -> Rc<dyn Entity>;
}

struct GlobalEntity {
    properties: HashMap<String, Rc<dyn Entity>>,
}

impl GlobalEntity {
    pub fn new() -> Self {
        let mut properties: HashMap<String, Rc<dyn Entity>> = HashMap::new();
        properties.insert("cd".to_string(), Rc::new(Cd::new()));
        Self {
            properties
        }
    }
}


impl Display for GlobalEntity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "GlobalEntity")
    }
}

impl Entity for GlobalEntity {
    fn args(&self) -> &[Argument] {
        return &[];
    }

    fn get_properties(&self) -> &HashMap<String, Rc<dyn Entity>> {
        return &self.properties;
    }

    fn call(&self, _args: &Vec<Value>) -> Rc<(dyn Entity + 'static)> {
        panic!("Global entity cannot be called");
    }
}

pub struct EntitiesManager {
    global: Rc<dyn Entity>,
}

impl EntitiesManager {
    pub fn new() -> Self {
        Self {
            global: Rc::new(GlobalEntity::new())
        }
    }

    pub fn global(&self) -> Rc<dyn Entity> {
        self.global.clone()
    }

    pub fn infer_from_pt<'a>(&self, node: &'a PTNode<'a>) -> Result<Rc<dyn Entity>, String> {
        if !matches!(node.kind, ASTKind::CallChain | ASTKind::PropertyCall) {
            panic!("Expected call chain or property call. Got: {}", node.kind);
        }

        let left_entity = if node.kind == ASTKind::CallChain {
            self.infer_from_pt(node.value::<CallChain>().get_left_hand(node).unwrap())?
        } else {
            self.global().clone()
        };

        let call = node.find_child_with_kind_rec(ASTKind::PropertyCall).unwrap();
        let value: &PropertyCall = call.value();

        let name = value.get_property_name(call).unwrap();
        let properties = left_entity.get_properties();

        let property = properties.get(name);

        if property.is_none() {
            return Err(format!("Property {} not found on {}", name, left_entity));
        }
        let property = property.unwrap();

        let args = value.get_arguments(call);

        let mut arg_values = vec![];

        for arg in args {
            match arg.kind {
                ASTKind::StringLiteral => {
                    arg_values.push(arg.value::<StringLiteral>().get_value(arg));
                },
                ASTKind::Identifier => {
                    arg_values.push(arg.value::<Identifier>().get_value(arg));
                },
                ASTKind::NumberLiteral => {
                    let value = arg.value::<NumberLiteral>().get_value(arg);
                    if let Ok(value) = value {
                        arg_values.push(value);
                    }
                },
                _ => {}
            }
        }

        if arg_values.len() != property.args().len() {
            return Err(format!("Wrong number of arguments for property {}", name));
        }

        for i in 0..arg_values.len() {
            if arg_values.get(i).unwrap().my_type() != property.args()[i].ty {
                return Err(format!("Wrong type for argument {} on property {}", i, name));
            }
        }

        return Ok(property.call(&arg_values));


    }

}


