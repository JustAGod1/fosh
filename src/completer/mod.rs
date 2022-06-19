use std::collections::HashMap;
use std::future::Future;
use crate::completer::parse_tree::PTNode;
use crate::parser::ast::ASTKind;

pub mod parse_tree;
mod path_completer;

pub(crate) struct CompleterManager {
    completer: HashMap<ASTKind, Box<dyn Completer>>
}

impl CompleterManager {
    pub fn new() -> Self {
        let mut map: HashMap<ASTKind, Box<dyn Completer>> = HashMap::new();
        //map.insert(ASTKind::CommandName, Box::new(path_completer::PathCompleter::new()));
        Self {
            completer: map
        }
    }

    pub fn complete<'a>(&self, node: &'a PTNode<'a>) -> Vec<String> {
        let kind = node.kind;
        let completer = self.completer.get(&kind);
        match completer {
            Some(c) => c.complete(node),
            None => Vec::new()
        }
    }

}

trait Completer {

    fn complete<'a>(&self, node: &'a PTNode<'a>) -> Vec<String>;
}

