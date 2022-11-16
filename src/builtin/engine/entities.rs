use std::borrow::Cow;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::fmt::{Display, Formatter, write};
use std::fs::File;
use std::io::{ErrorKind, Read, stderr, stdin, stdout, Write};
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use nix::libc::stat;
use parse_display_derive::Display;
use pipe::{PipeReader, PipeWriter};
use fosh::error_printer::ErrorType;
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

pub struct ProcessExecution {
    child: Child,
    node_id: PTNodeId,
}

impl ProcessExecution {
    pub fn new(child: Child, node_id: PTNodeId) -> Self {
        Self {
            child,
            node_id,
        }
    }
}

pub struct Callee {
    pub arguments: Vec<Argument>,
    pub callee: Box<dyn for<'b> Fn(EntityRef, &'b [EntityRef], ExecutionConfig) -> Result<Box<dyn Execution<'b> + 'b>, EntityExecutionError>>,
    pub result_prototype: Option<Box<dyn Fn(EntityRef, &[Option<EntityRef>]) -> Option<EntityRef>>>,
}

impl Callee {
    pub fn new<F>(block: F) -> Self
        where F: for<'b> Fn(EntityRef, &'b [EntityRef], ExecutionConfig) -> Result<Box<(dyn Execution<'b> + 'b)>, EntityExecutionError> + 'static
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
            callee: Box::new(move |_me, args, mut config| {
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

    pub fn with_result_prototype<F>(mut self, prototype: F) -> Self
    where F : Fn(EntityRef, &[Option<EntityRef>]) -> Option<EntityRef> + 'static
    {
        self.result_prototype = Some(Box::new(prototype));
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


impl<'b> Execution<'b> for ProcessExecution {
    fn execute(&mut self) -> Result<EntityRef, EntityExecutionError> {
        let status = self.child.wait().map_err(|x| EntityExecutionError::new_single(self.node_id, ErrorType::Execution, format!("{:?}", x)))?;
        Ok(
            entities()
                .make_entity("Execution result".to_string())
                .with_property("status", Value::Number(status.code().unwrap_or(-1) as f64).into_entity())
        )
    }
}

#[derive(Debug)]
pub struct ErrorData {
    pub kind: ErrorType,
    pub hints: Vec<String>,
    pub notes: Vec<String>
}

impl ErrorData {
    pub fn new(kind: ErrorType) -> Self {
        Self {
            kind,
            hints: vec![],
            notes: vec![],
        }
    }

    pub fn with_hints(&mut self, hints: Vec<String>) -> &mut Self {
        self.hints = hints;
        self
    }

    pub fn with_notes(&mut self, notes: Vec<String>) -> &mut Self {
        self.notes = notes;
        self
    }
}

#[derive(Debug)]
pub struct EntityExecutionError {
    pub errors: HashMap<PTNodeId, ErrorData>,
}

impl EntityExecutionError {
    pub fn new() -> Self {
        Self {
            errors: HashMap::new(),
        }
    }

    pub fn new_single<S : Into<String>>(node_id: PTNodeId, kind: ErrorType, note: S) -> Self {
        let mut r = Self::new();
        r.with_error(node_id, kind).with_notes(vec![note.into()]);
        r
    }

    pub fn with_error(&mut self, node_id: PTNodeId, kind: ErrorType) -> &mut ErrorData {
        self.errors.insert(node_id, ErrorData::new(kind));
        return self.errors.get_mut(&node_id).unwrap();
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
