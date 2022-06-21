use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use crate::builtin::{Argument, Entity, Type, Value};
use crate::builtin::contributors::FilesContributor;

pub struct Stub {
    properties: HashMap<String, Rc<dyn Entity>>
}

impl Stub {
    pub fn new() -> Self {
        Self { properties: HashMap::new() }
    }
}

impl Display for Stub {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stub")
    }
}

impl Entity for Stub {
    fn args(&self) -> &[Argument] {
        return &[];
    }

    fn get_properties(&self) -> &HashMap<String, Rc<dyn Entity>> {
        return &self.properties;
    }

    fn call(&self, _args: &Vec<Value>) -> Rc<dyn Entity> {
        return Rc::new(Stub::new());
    }
}

pub struct Cd {
    pub args: Vec<Argument>,
    pub properties: HashMap<String, Rc<dyn Entity>>,
}

impl Cd {
    pub fn new() -> Self {
        Self {
            args: vec![
                Argument {
                    name: "path".to_string(),
                    ty: Type::String,
                    contributor: Box::new(FilesContributor::new()),
                },
            ],
            properties: Default::default()
        }
    }
}

impl Display for Cd {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cd")
    }
}

impl Entity for Cd {
    fn args(&self) -> &[Argument] {
        return &self.args;
    }

    fn get_properties(&self) -> &HashMap<String, Rc<dyn Entity>> {
        return &self.properties;
    }


    fn call(&self, _args: &Vec<Value>) -> Rc<(dyn Entity + 'static)> {
        Rc::new(Stub::new())
    }
}

