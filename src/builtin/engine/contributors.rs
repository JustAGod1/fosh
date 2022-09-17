use crate::builtin::engine::Value;

pub trait Contributor {
    fn contribute(&self, value: Value) -> Vec<Value>;
}