#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    String
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    String(String)
}

impl Value {

    fn my_type(&self) -> Type {
        match self {
            Value::String(_) => Type::String,
        }
    }

}

pub struct Argument {
    name: String,
    ty: Type,
    default: Value,
    contributor: Box<dyn Contributor>
}

pub trait Contributor {
    fn contribute(&self, value: Value) -> Vec<Value>;
}


pub struct EntityProperty {
    name: String,
    args: Vec<Argument>,
}

impl EntityProperty {
    pub fn new(name: String, args: Vec<Argument>) -> Self {
        Self {
            name,
            args
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn args(&self) -> &Vec<Argument> {
        &self.args
    }
}

pub trait BuiltinEntity {
    fn get_properties(&self, args: Option<&Vec<Value>>) -> Vec<EntityProperty>;
}