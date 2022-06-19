use std::cell::{Cell, RefCell};
use std::ops::Deref;
use typed_arena::Arena;
use crate::parser::ast::{ASTKind, ASTNode, ErroredASTValue};

pub struct PTNode<'a> {
    pub data: &'a str,
    pub kind: ASTKind,
    pub origin: &'a ASTNode,
    pub children: RefCell<Vec<&'a PTNode<'a>>>,
    pub parent: Cell<Option<&'a PTNode<'a>>>,
}

impl<'a> PTNode<'a> {
    pub fn text(&'a self) -> &'a str {
        return self.origin.span.slice(self.data);
    }


    pub fn find_leaf_on_pos(&'a self, pos: usize) -> Option<&'a PTNode<'a>> {
        if self.is_leaf() {
            return if self.origin.span.as_range().contains(&pos) {
                Some(self)
            } else {
                None
            }
        }

        for child in self.children.borrow().iter() {
            let v = child.find_leaf_on_pos(pos);
            if v.is_some() { return v;}
        }

        return None;
    }

    pub fn walk<F>(&self, visitor: &mut F) where F : FnMut(&PTNode) {
        visitor(self);
        for child in Deref::deref(&self.children.borrow()) {
            child.walk(visitor);
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.children.borrow().is_empty()
    }
}



pub struct ParseTree<'a> {
    builder: ParseTreeBuilder<'a>,
    root: Cell<Option<&'a PTNode<'a>>>,
    ast: ASTNode,
}

impl <'a>ParseTree<'a> {

    pub fn new(command: &'a str, ast: ASTNode) -> Self {
        let builder = ParseTreeBuilder::new(command);
        let result = Self {
            builder,
            ast,
            root: Default::default()
        };



        result
    }

    pub fn root(&'a self) -> &'a PTNode<'a> {
        if self.root.get().is_none() {
            self.root.set(Some(self.builder.parse_ast(&self.ast)));
        }
        self.root.get().unwrap()
    }
}

struct ParseTreeBuilder<'a> {
    data: &'a str,
    arena: Arena<PTNode<'a>>,
}

impl<'a> ParseTreeBuilder<'a> {
    fn new(data: &'a str) -> Self {
        Self {
            data,
            arena: Arena::new(),
        }
    }
    fn parse_ast(&'a self, ast: &'a ASTNode) -> &'a PTNode<'a> {
        return self.parse_node(ast);
    }


    fn parse_node(&'a self, node: &'a ASTNode) -> &'a PTNode<'a> {
        let kind = match node.value.kind() {
            ASTKind::Error => node.value.downcast_ref::<ErroredASTValue>().unwrap().expected,
            _ => node.value.kind()
        };
        let node: &'a mut PTNode = self.arena.alloc(PTNode {
            data: node.span.slice(self.data),
            kind,
            origin: node,
            parent: Default::default(),
            children: Default::default(),
        });

        for child in &node.origin.children {
            let child_node = self.parse_node(child);
            child_node.parent.set(Some(node));
            node.children.borrow_mut().push(child_node);
        }

        node
    }

}


