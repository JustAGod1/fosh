use std::borrow::Borrow;
use std::cell::{Cell, Ref, RefCell, RefMut, UnsafeCell};
use std::ops::Deref;
use typed_arena::Arena;
use crate::parser;
use crate::parser::ast::{ASTKind, ASTNode, ASTValue, ASTError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PTNodeId(usize);

pub struct PTNode<'a> {
    root: UnsafeCell<Option<&'a PTNode<'a>>>,
    pub data: &'a str,
    pub kind: ASTKind,
    pub origin: &'a ASTNode,
    children: RefCell<Vec<&'a PTNode<'a>>>,
    parent: Cell<Option<&'a PTNode<'a>>>,
    position: usize,
    id: PTNodeId,
}

#[allow(dead_code)]
impl<'a> PTNode<'a> {
    pub fn text(&'a self) -> &'a str {
        return self.data;
    }


    pub fn root(&self) -> &'a PTNode<'a> {
        unsafe { &*self.root.get()}.unwrap()
    }

    pub fn id(&self) -> PTNodeId {
        self.id
    }

    pub fn find_leaf_on_pos(&'a self, pos: usize) -> Option<&'a PTNode<'a>> {
        if self.is_leaf() {
            return if self.origin.span.as_range().contains(&pos) {
                Some(self)
            } else {
                None
            };
        }

        for child in self.children.borrow().iter() {
            let v = child.find_leaf_on_pos(pos);
            if v.is_some() { return v; }
        }

        return None;
    }

    pub fn walk<F>(&'a self, visitor: &mut F) where F: FnMut(&'a PTNode<'a>) {
        visitor(self);
        for child in Deref::deref(&self.children.borrow()) {
            child.walk(visitor);
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.children.borrow().is_empty()
    }

    pub fn find_child_with_kind<'b>(&'b self, kind: ASTKind) -> Option<&'b PTNode<'a>> {
        for child in self.children.borrow().iter() {
            if child.kind == kind {
                return Some(child);
            }
        }
        return None;
    }
    pub fn find_child_with_kind_rec<'b>(&'b self, kind: ASTKind) -> Option<&'b PTNode<'a>> {
        if self.kind == kind || (kind == ASTKind::Error && self.origin.value.kind() == ASTKind::Error) {
            return Some(self);
        }
        for child in self.children.borrow().iter() {
            let v = child.find_child_with_kind_rec(kind);
            if v.is_some() { return v; }
        }
        return None;
    }

    pub fn children(&'a self) -> Ref<Vec<&PTNode<'a>>> {
        return RefCell::borrow(&self.children);
    }

    pub fn parent(&'a self) -> Option<&'a PTNode<'a>> {
        return self.parent.get();
    }

    pub fn value<T: ASTValue>(&self) -> &T {
        self.origin.value.downcast_ref().unwrap()
    }

    pub fn find_parent_with_kind(&'a self, kind: ASTKind) -> Option<&'a PTNode<'a>> {
        if self.kind == kind {
            return Some(self);
        }
        if self.parent.get().is_none() {
            return None;
        }
        return self.parent.get().unwrap().find_parent_with_kind(kind);
    }

    pub fn find_node(&'a self, id: PTNodeId) -> Option<&'a PTNode<'a>> {
        if self.id == id {
            return Some(self);
        }
        for child in self.children.borrow().iter() {
            let v = child.find_node(id);
            if v.is_some() { return v; }
        }
        return None;
    }

    pub fn position(&self) -> usize {
        self.position
    }
}


pub struct ParseTree<'a> {
    builder: ParseTreeBuilder<'a>,
    root: Cell<Option<&'a PTNode<'a>>>,
    ast: ASTNode,
}

impl<'a> ParseTree<'a> {
    pub fn new(command: &'a str, ast: ASTNode) -> Self {
        let builder = ParseTreeBuilder::new(command);
        let result = Self {
            builder,
            ast,
            root: Default::default(),
        };


        result
    }

    pub fn root(&'a self) -> &'a PTNode<'a> {
        if self.root.get().is_none() {
            self.root.set(Some(self.builder.parse_ast(&self.ast)));
        }
        self.root.get().unwrap()
    }

    pub fn collect<F>(&'a self, container: &mut Vec<&'a PTNode<'a>>, predicate: F)
        where F: Fn(&'a PTNode<'a>) -> bool {
        self.root().walk(&mut |node| {
            if predicate(node) {
                container.push(node);
            }
        });
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
        return self.parse_node(ast, 0, 0, None).0;
    }


    fn parse_node(&'a self, node: &'a ASTNode, position: usize, id: usize, mut root: Option<&'a PTNode<'a>>) -> (&'a PTNode<'a>, usize) {
        let mut id = id;
        let kind = match node.value.kind() {
            ASTKind::Error => node.value.downcast_ref::<ASTError>().unwrap().expected.kind(),
            _ => node.value.kind()
        };
        let node: &'a mut PTNode = self.arena.alloc(PTNode {
            root: root.into(),
            data: node.span.slice(self.data),
            kind,
            origin: node,
            parent: Default::default(),
            children: Default::default(),
            position,
            id: PTNodeId(id),
        });

        let node_root: &mut Option<&'a PTNode<'a>> = unsafe { std::mem::transmute(node.root.get())  };
        if node_root.is_none() { *node_root = Some(node); root = Some(node); }

        let mut pos = 0usize;
        for child in &node.origin.children {
            let (child_node, new_id) = self.parse_node(child, pos, id + 1, root.clone());
            id = new_id + 1;
            child_node.parent.set(Some(node));
            node.children.borrow_mut().push(child_node);
            pos += 1;
        }

        (node, id)
    }
}

pub fn parse_line(line: &str) -> Option<ParseTree> {
    let ast = parser::parse(line);
    if ast.is_err() {
        return None;
    }
    let tree = ParseTree::new(line, ast.unwrap());

    return Some(tree);
}
