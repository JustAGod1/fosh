use std::borrow::Cow;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::fmt::{Display, Formatter, write};
use std::fs::File;
use std::io::{Read, stderr, stdin, stdout, Write};
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use nix::libc::stat;
use parse_display_derive::Display;
use pipe::{PipeReader, PipeWriter};
use crate::builtin::contributors::FilesContributor;
use crate::builtin::engine::{Argument, Type, Value};
use crate::builtin::engine::parse_tree::PTNodeId;
use crate::entities;

pub type EntityRef = Rc<RefCell<Entity>>;

pub struct ExecutionConfig {
    pub std_in: Option<File>,
    pub std_out: Option<File>,
    pub std_err: Option<File>,
}

pub trait Execution<'a> {
    fn execute(&mut self) -> Result<EntityRef, EntityExecutionError>;
}

pub struct PseudoExecution<'a> {
    work: Option<Box<dyn for<'b> FnOnce() -> Result<EntityRef, EntityExecutionError> + 'a>>,
}

pub struct Callee {
    pub arguments: Vec<Argument>,
    pub callee: Box<dyn for<'b> Fn(&'b [EntityRef], ExecutionConfig) -> Result<Box<dyn Execution<'b> + 'b>, EntityExecutionError>>,
    pub result_prototype: Option<EntityRef>,
}

impl Callee {
    pub fn new<F>(block: F) -> Self
        where F: for<'b> Fn(&'b [EntityRef], ExecutionConfig) -> Result<Box<(dyn Execution<'b> + 'b)>, EntityExecutionError> + 'static
    {
        Self {
            arguments: vec![],
            callee: Box::new(block),
            result_prototype: None,
        }
    }

    pub fn new_pseudo_execution<F>(block: F) -> Callee
        where F: 'static + FnOnce(&[EntityRef], &mut dyn Read, &mut dyn Write, &mut dyn Write)
            -> Result<EntityRef, EntityExecutionError> + Copy
    {
        Self {
            arguments: vec![],
            callee: Box::new(move |args, mut config| {
                let execution = PseudoExecution::from(move || {
                    let mut stdin = stdin();
                    let mut stdout = stdout();
                    let mut stderr = stderr();
                    let mut stderr = config.std_err.as_mut().map(|a| a as &mut dyn Write).unwrap_or_else(|| &mut stderr);
                    let mut stdout = config.std_out.as_mut().map(|a| a as &mut dyn Write).unwrap_or_else(|| &mut stdout);
                    let mut stdin = config.std_in.as_mut().map(|a| a as &mut dyn Read).unwrap_or_else(|| &mut stdin);

                    block(args, &mut stdin, &mut stdout, &mut stderr)
                });
                Ok(Box::new(execution))
            }),
            result_prototype: None,
        }
    }

    pub fn with_arguments(mut self, arguments: Vec<Argument>) -> Self {
        self.arguments = arguments;
        self
    }

    pub fn with_result_prototype(mut self, prototype: EntityRef) -> Self {
        self.result_prototype = Some(prototype);
        self
    }
}

impl<'a> PseudoExecution<'a> {
    pub fn from<F>(work: F) -> Self
        where F: FnOnce() -> Result<EntityRef, EntityExecutionError> + 'a, F: 'a
    {
        Self {
            work: Some(Box::new(work))
        }
    }
}

impl<'b> Execution<'b> for PseudoExecution<'b> {
    fn execute(&mut self) -> Result<EntityRef, EntityExecutionError> {
        let mut work = None::<_>;
        std::mem::swap(&mut work, &mut self.work);
        work.expect("Cannot call execution twice")()
    }
}


impl<'b> Execution<'b> for Child {
    fn execute(&mut self) -> Result<EntityRef, EntityExecutionError> {
        let status = self.wait().map_err(|x| x.to_string().into())?;
        Ok(
            entities()
                .make_entity("Execution result".to_string())
                .with_property("status", Value::Number(status.code().unwrap_or(-1) as f64).into_entity())
        )
    }
}

#[derive(Debug)]
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

impl Into<EntityExecutionError> for String {
    fn into(self) -> EntityExecutionError {
        EntityExecutionError::new().with_general_error(self)
    }
}

impl Into<EntityExecutionError> for &str {
    fn into(self) -> EntityExecutionError {
        EntityExecutionError::new().with_general_error(self)
    }
}

pub struct Entity {
    name: String,

    implicits: HashMap<Type, Box<dyn Fn() -> Value +'static>>,
    callee: Option<Box<Callee>>,

    properties: HashMap<String, EntityRef>,

    prototype: Option<EntityRef>,
}

impl Display for Entity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut properties = String::new();
        properties.push('[');

        for (name, value) in &self.properties {
            properties.push_str(&format!("{}: {},", name, value.borrow()));
        }
        properties.push(']');

        let mut implicits = String::new();
        if !self.implicits.is_empty() {
            implicits.push_str(", implicits: [");
            for x in self.implicits.keys() {
                implicits.push_str(format!("{:?}", x).as_str());
            }
            implicits.push_str("] ");
        }

        write!(f, "{{ {}: {{ prototype: {:?}, properties: {} {}}} }}", self.name, self.prototype.clone().map(|a| a.borrow().name.clone()), properties, implicits)
    }
}

pub struct Comms<'a> {
    pub std_in: &'a mut dyn Read,
    pub std_out: &'a mut dyn Write,
    pub std_err: &'a mut dyn Write,
}

impl Entity {

    pub fn implicits(&self) -> &HashMap<Type, Box<dyn Fn() -> Value + 'static>> {
        &self.implicits
    }
    pub fn callee(&self) -> &Option<Box<Callee>> {
        &self.callee
    }
    pub fn properties(&self) -> &HashMap<String, EntityRef> {
        &self.properties
    }
    pub fn prototype(&self) -> Option<EntityRef> {
        self.prototype.clone()
    }
}

impl FoshEntity for EntityRef {
    fn name(&self) -> &str {
        let x = &self.borrow();
        unsafe { std::mem::transmute(x.name.as_str()) }
    }
    fn with_callee(self, callee: Callee) -> Self {
        self.borrow_mut().callee = Some(Box::new(callee));
        self
    }
    fn with_property(mut self, name: &str, property: EntityRef) -> Self {
        self.borrow_mut().properties.insert(name.to_string(), property);
        self
    }
    fn add_property(&mut self, name: &str, property: EntityRef) {
        self.borrow_mut().properties.insert(name.to_string(), property);
    }
    fn with_implicit<F, V>(mut self, type_: Type, implicit: F) -> Self
        where F: Fn() -> V, F: 'static, V: Into<Value>
    {
        self.borrow_mut().implicits.insert(type_, Box::new(move || implicit().into()));
        self
    }
}

pub trait FoshEntity {
    fn name(&self) -> &str;
    fn with_callee(self, callee: Callee) -> Self;
    fn with_property(self, name: &str, property: EntityRef) -> Self;
    fn add_property(&mut self, name: &str, property: EntityRef);
    fn with_implicit<F, V>(self, type_: Type, implicit: F) -> Self
        where F: Fn() -> V, F: 'static, V: Into<Value>
    ;
}

pub fn implicit_type(typ: Type, entity: Option<&Entity>) -> Option<Value> {
    if entity.is_none() { return None; }
    let entity = entity.unwrap();

    let implicit = entity.implicits.get(&typ);
    if implicit.is_none() { return None; }
    let implicit = implicit.unwrap();
    Some(implicit())
}


pub struct EntitiesManager {
    pub files_contributor: FilesContributor,
    any: EntityRef,
    global: EntityRef,
}

impl EntitiesManager {
    pub fn new() -> EntitiesManager {
        EntitiesManager {
            files_contributor: FilesContributor {},
            any: Rc::new(RefCell::new(Entity {
                name: "Any".to_string(),
                implicits: HashMap::new(),
                callee: None,
                properties: HashMap::new(),
                prototype: None,
            })),
            global: Rc::new(RefCell::new(Entity {
                name: "Global".to_string(),
                implicits: HashMap::from([]),
                callee: None,
                properties: HashMap::from([]),
                prototype: None,
            })),
        }
    }

    pub fn make_entity(&self, name: String) -> EntityRef {
        Rc::new(RefCell::new(Entity {
            name,
            implicits: HashMap::new(),
            callee: None,
            properties: HashMap::new(),
            prototype: Some(self.any.clone())
        }))
    }

    pub fn global(&self) -> EntityRef {
        self.global.clone()
    }

    pub fn any(&self) -> EntityRef {
        self.any.clone()
    }

}

