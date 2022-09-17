use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::{Read, Write};
use std::rc::Rc;
use pipe::{PipeReader, PipeWriter};
use crate::builtin::contributors::FilesContributor;
use crate::builtin::engine::{Argument, Type, Value};
use crate::builtin::engine::parse_tree::PTNodeId;

pub trait Execution {
    fn std_input(&mut self) -> &mut dyn Write;
    fn std_out(&mut self) -> &mut dyn Read;
    fn std_err(&mut self) -> &mut dyn Read;
}

pub struct PseudoExecution {
    std_out_in: PipeWriter,
    std_out_out: PipeReader,
    std_err_in: PipeWriter,
    std_err_out: PipeReader,
    std_in_in: PipeWriter,
    std_in_out: PipeReader,

}

impl PseudoExecution {
    pub fn new() -> Self {
        let (std_out_out, std_out_in) = pipe::pipe();
        let (std_err_out, std_err_in) = pipe::pipe();
        let (std_in_out, std_in_in) = pipe::pipe();
        Self {
            std_out_in,
            std_out_out,
            std_err_in,
            std_err_out,
            std_in_in,
            std_in_out,
        }
    }
}

impl Execution for PseudoExecution {
    fn std_input(&mut self) -> &mut dyn Write {
        return &mut self.std_in_in;
    }

    fn std_out(&mut self) -> &mut dyn Read {
        &mut self.std_out_out
    }

    fn std_err(&mut self) -> &mut dyn Read {
        &mut self.std_err_out
    }
}


pub struct EntityExecutionError {
    general: Option<String>,
    errors: HashMap<PTNodeId, Vec<String>>,
}

impl EntityExecutionError {
    pub fn new() -> Self {
        Self {
            general: None,
            errors: HashMap::new(),
        }
    }

    pub fn with_general_error<S: Into<String>>(mut self, error: S) -> Self {
        self.general = Some(error.into());
        self
    }

    pub fn with_error<S: Into<String>>(mut self, node_id: PTNodeId, error: S) -> Self {
        self.errors.entry(node_id).or_insert(Vec::new()).push(error.into());
        self
    }
}

pub struct Entity<'a> {
    name: String,

    arguments: Vec<Argument<'a>>,
    implicits: HashMap<Type, Box<dyn Fn(&Entity) -> Value<'static>>>,
    callee: Option<Box<dyn Fn(&mut Entity, &Vec<Entity>) -> Result<Entity<'a>, EntityExecutionError> + 'a>>,
    execution_not_piped: Option<Box<dyn Fn(&mut Entity) -> Result<Box<dyn Execution>, EntityExecutionError>>>,

    properties: HashMap<String, Entity<'a>>,

    prototype: Option<&'a RefCell<Entity<'a>>>,
}

impl Display for Entity<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub struct Comms<'a> {
    pub std_in: &'a mut dyn Read,
    pub std_out: &'a mut dyn Write,
    pub std_err: &'a mut dyn Write,
}

impl <'a>Entity<'a> {
    pub fn with_pseudo_execution<F>(mut self, block: F) -> Self
        where F: Fn(&mut Entity, &mut Comms) -> Result<(), EntityExecutionError> + 'static {
        self.execution_not_piped = Some(Box::new(move |entity| {
            let mut execution = PseudoExecution::new();
            let mut comms = Comms {
                std_in: &mut execution.std_in_out,
                std_out: &mut execution.std_out_in,
                std_err: &mut execution.std_err_in,
            };
            block(entity, &mut comms)?;
            Ok(Box::new(execution))
        }));

        self
    }

    pub fn with_property(mut self, name: &str, property: Entity<'a>) -> Self {
        self.properties.insert(name.to_string(), property);
        self
    }

    pub fn add_property(&mut self, name: &str, property: Entity<'a>) {
        self.properties.insert(name.to_string(), property);
    }

    pub fn with_implicit(mut self, type_: Type, implicit: Box<dyn Fn(&Entity) -> Value<'static>>) -> Self {
        self.implicits.insert(type_, implicit);
        self
    }

    pub fn with_arguments(mut self, arguments: Vec<Argument<'a>>) -> Self {
        self.arguments = arguments;
        self
    }

    pub fn with_callee<F>(mut self, block: F) -> Self
        where F: Fn(&mut Entity, &Vec<Entity>) -> Result<Entity<'a>, EntityExecutionError> + 'a {
        self.callee = Some(Box::new(block));
        self
    }
}

pub fn implicit_type(typ: Type, entity: Option<&Entity>) -> Option<Value<'static>> {
    if entity.is_none() { return None; }
    let entity = entity.unwrap();

    let implicit = entity.implicits.get(&typ);
    if implicit.is_none() { return None; }
    let implicit = implicit.unwrap();
    Some(implicit(entity))
}


pub struct EntitiesManager<'a> {
    pub files_contributor: FilesContributor,
    any: RefCell<Entity<'a>>,
    global: RefCell<Entity<'a>>,
}

impl <'a>EntitiesManager<'a> {
    pub fn new() -> EntitiesManager<'a> {
        EntitiesManager {
            files_contributor: FilesContributor {},
            any: RefCell::new(Entity {
                name: "Any".to_string(),
                arguments: vec![],
                implicits: HashMap::new(),
                callee: None,
                execution_not_piped: None,
                properties: HashMap::new(),
                prototype: None,
            }),
            global: RefCell::new(Entity {
                name: "Global".to_string(),
                arguments: vec![],
                implicits: HashMap::from([]),
                callee: None,
                execution_not_piped: None,
                properties: HashMap::from([]),
                prototype: None
            }),
        }
    }

    pub fn make_entity(&'a self, name: String) -> Entity<'a> {
        Entity {
            name,
            arguments: vec![],
            implicits: HashMap::new(),
            callee: None,
            execution_not_piped: None,
            properties: HashMap::new(),
            prototype: Some(&self.any),
        }
    }

    pub fn global(&'a self) -> RefMut<Entity<'a>> {
        self.global.borrow_mut()
    }
}

