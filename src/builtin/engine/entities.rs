use std::borrow::Cow;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::fmt::{Display, Formatter, write};
use std::fs::File;
use std::future::Future;
use std::io::{Error, ErrorKind, Read, stderr, stdin, stdout, Write};
use std::os::unix::io::{AsFd, OwnedFd};
use std::os::unix::prelude::{FromRawFd, RawFd};
use std::pin::Pin;
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::task::{Context, Poll};
use nix::libc::{stat};
use nix::unistd::dup;
use parse_display_derive::Display;
use pipe::{PipeReader, PipeWriter};
use fosh::error_printer::ErrorType;
use crate::builtin::contributors::FilesContributor;
use crate::builtin::engine::{Argument, Type, Value};
use crate::builtin::engine::parse_tree::{PTNode, PTNodeId};
use crate::entities;

pub type EntityRef = Rc<RefCell<Entity>>;
pub type FoshResult<A> = Result<A, EntityExecutionError>;

pub trait AwaitableFuture<T>: Future {
    fn wait(self: Pin<&mut Self>) -> T;
}

#[derive(Debug)]
pub struct ExecutionConfig {
    pub std_in: Option<OwnedFd>,
    pub std_out: Option<OwnedFd>,
    pub std_err: Option<OwnedFd>,
    pub pt: PTNodeId,
}

impl ExecutionConfig {
    pub fn try_clone(&self) -> Result<ExecutionConfig, Error> {
        let std_in = match self.std_in.as_ref() {
            None => {None}
            Some(f) => { Some(f.try_clone()?)}
        };
        let std_out = match self.std_out.as_ref() {
            None => {None}
            Some(f) => { Some(f.try_clone()?)}
        };
        let std_err = match self.std_err.as_ref() {
            None => {None}
            Some(f) => { Some(f.try_clone()?)}
        };
        Ok(ExecutionConfig {
            std_in,
            std_out,
            std_err,
            pt: self.pt,
        })
    }
}

impl ExecutionConfig {
    pub fn new_with_dup(pt: PTNodeId, std_in: &OwnedFd, std_out: &OwnedFd, std_err: &OwnedFd) -> Result<ExecutionConfig, Error> {
        Ok(ExecutionConfig {
            std_in: Some(std_in.try_clone()?),
            std_out: Some(std_out.try_clone()?),
            std_err: Some(std_err.try_clone()?),
            pt,
        })
    }
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

pub enum Execution {
    Pseudo(Box<dyn FnOnce()
        -> FoshResult<EntityRef>>),
    Process(ProcessExecution),
}

impl Execution {

    pub fn execute(mut self) -> FoshResult<EntityRef> {
        match self {
            Execution::Pseudo(f) => f(),
            Execution::Process(mut exec) => exec.execute()

        }
    }

}

impl Execution {

    fn new_pseudo(
        f: impl FnOnce()
            -> Result<EntityRef, EntityExecutionError> + 'static,
    ) -> Self {
        Self::Pseudo(Box::new(f))
    }
}

pub struct Callee {
    pub arguments: Vec<Argument>,
    pub callee: Box<dyn Fn(EntityRef, &[EntityRef], ExecutionConfig) -> Result<Execution, EntityExecutionError>>,
    pub result_prototype: Option<Box<dyn Fn(EntityRef, &[Option<EntityRef>]) -> Option<EntityRef>>>,
}

impl Callee {
    pub fn new<F>(block: F) -> Self
        where F: for<'b> Fn(EntityRef, &'b [EntityRef], ExecutionConfig) -> Result<Execution, EntityExecutionError> + 'static
    {
        Self {
            arguments: vec![],
            callee: Box::new(block),
            result_prototype: None,
        }
    }

    pub fn new_pseudo_execution<F>(block: F) -> Callee
        where F: 'static + FnOnce(PTNodeId, &[EntityRef], &mut dyn Read, &mut dyn Write, &mut dyn Write)
            -> Result<EntityRef, EntityExecutionError> + Copy
    {
        Self {
            arguments: vec![],
            callee: Box::new(move |_me, args, mut config| {
                let entities = args.iter().map(|a| a.clone()).collect::<Vec<_>>();
                let execution = Execution::new_pseudo(move || {
                    let stdin = stdin();
                    let stdout = stdout();
                    let stderr = stderr();
                    let stderr = config.std_err.as_mut().map(|a| a.as_fd().try_clone_to_owned()).unwrap_or_else(|| stderr.as_fd().try_clone_to_owned());
                    let stdout = config.std_out.as_mut().map(|a| a.as_fd().try_clone_to_owned()).unwrap_or_else(|| stdout.as_fd().try_clone_to_owned());
                    let stdin = config.std_in.as_mut().map(|a| a.as_fd().try_clone_to_owned()).unwrap_or_else(|| stdin.as_fd().try_clone_to_owned());

                    if stdin.is_err() {
                        return Err(EntityExecutionError::new_single(config.pt, ErrorType::CannotCloneFd, format!("{}", stdin.err().unwrap())));
                    }
                    let stdin = stdin.unwrap();

                    if stdout.is_err() {
                        return Err(EntityExecutionError::new_single(config.pt, ErrorType::CannotCloneFd, format!("{}", stdout.err().unwrap())));
                    }
                    let stdout = stdout.unwrap();

                    if stderr.is_err() {
                        return Err(EntityExecutionError::new_single(config.pt, ErrorType::CannotCloneFd, format!("{}", stderr.err().unwrap())));
                    }
                    let stderr = stderr.unwrap();

                    let mut stdin = File::from(stdin);
                    let mut stdout = File::from(stdout);
                    let mut stderr = File::from(stderr);

                    block(config.pt, &entities, &mut stdin, &mut stdout, &mut stderr)
                });
                Ok(execution)
            }),
            result_prototype: None,
        }
    }

    pub fn with_arguments(mut self, arguments: Vec<Argument>) -> Self {
        self.arguments = arguments;
        self
    }

    pub fn with_result_prototype<F>(mut self, prototype: F) -> Self
        where F: Fn(EntityRef, &[Option<EntityRef>]) -> Option<EntityRef> + 'static
    {
        self.result_prototype = Some(Box::new(prototype));
        self
    }
}

impl ProcessExecution {
    pub fn execute(mut self) -> FoshResult<EntityRef> {
        match self.child.wait() {
            Ok(status) => {
                if status.success() {
                    Ok(entities()
                        .make_entity("Execution result".to_string())
                        .with_property("status", Value::Number(status.code().unwrap_or(-1) as f64).into_entity())
                    )
                } else {
                    Err(EntityExecutionError::new_single(self.node_id, ErrorType::Execution, format!("Execution failed with status {}", status)))
                }
            }
            Err(e) => {
                Err(EntityExecutionError::new_single(self.node_id, ErrorType::Execution, e.to_string()))
            }
        }
    }
}



#[derive(Debug, Clone)]
pub struct ErrorData {
    pub kind: ErrorType,
    pub hints: Vec<String>,
    pub notes: Vec<String>,
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

#[derive(Debug, Clone)]
pub struct EntityExecutionError {
    pub errors: HashMap<PTNodeId, ErrorData>,
}

impl EntityExecutionError {
    pub fn new() -> Self {
        Self {
            errors: HashMap::new(),
        }
    }

    pub fn new_single<S: Into<String>>(node_id: PTNodeId, kind: ErrorType, note: S) -> Self {
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

    implicits: HashMap<Type, Box<dyn Fn(EntityRef) -> Value + 'static>>,
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
    pub fn implicits(&self) -> &HashMap<Type, Box<dyn Fn(EntityRef) -> Value + 'static>> {
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
        where F: Fn(EntityRef) -> V, F: 'static, V: Into<Value>
    {
        self.borrow_mut().implicits.insert(type_, Box::new(move |e| implicit(e).into()));
        self
    }

    fn try_as_string(&self) -> Option<String> {
        let r = self.borrow();

        if let Some(x) = r.implicits.get(&Type::String) {
            if let Value::String(x) = x(self.clone()) {
                return Some(x);
            } else {
                panic!("Implicit string is not a string");
            }
        }
        return None;
    }

    fn try_as_number(&self) -> Option<f64> {
        let r = self.borrow();

        if let Some(x) = r.implicits.get(&Type::Number) {
            if let Value::Number(x) = x(self.clone()) {
                return Some(x);
            } else {
                panic!("Implicit number is not a number");
            }
        }
        return None;
    }
}

pub trait FoshEntity {
    fn name(&self) -> &str;
    fn with_callee(self, callee: Callee) -> Self;
    fn with_property(self, name: &str, property: EntityRef) -> Self;
    fn add_property(&mut self, name: &str, property: EntityRef);
    fn with_implicit<F, V>(self, type_: Type, implicit: F) -> Self
        where F: Fn(EntityRef) -> V, F: 'static, V: Into<Value>;

    fn try_as_string(&self) -> Option<String>;
    fn try_as_number(&self) -> Option<f64>;
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
            prototype: Some(self.any.clone()),
        }))
    }

    pub fn global(&self) -> EntityRef {
        self.global.clone()
    }

    pub fn any(&self) -> EntityRef {
        self.any.clone()
    }
}
