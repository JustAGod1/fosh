use std::collections::HashMap;
use std::rc::Rc;
use crate::builtin::{Argument, Entity, Type, Value};
use crate::builtin::contributors::FilesContributor;

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

impl Entity for Cd {
    fn name(&self) -> &str {
        "cd"
    }

    fn args(&self) -> &[Argument] {
        return &self.args;
    }

    fn get_properties(&self) -> &HashMap<String, Rc<dyn Entity>> {
        return &self.properties;
    }

    fn call(&self, args: &Vec<Value>) -> Rc<dyn Entity> {
        todo!()
    }
}