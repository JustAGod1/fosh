use std::ops::Index;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use crate::builtin::engine::annotator::{AnnotationsSink, Annotator, AnnotatorContext};
use crate::builtin::engine::entities::{EntitiesManager, FoshEntity};
use crate::builtin::engine::parse_tree::PTNode;
use crate::builtin::engine::{Type, Value};
use crate::entities;
use crate::parser::ast::{ASTKind, downcast_to_typed, Identifier, NumberLiteral, Parameter, ParenthesizedArgumentsList, PropertyCall, PropertyName, StringLiteral, Typed};

pub fn downcast_to_annotator<'a>(node: &'a PTNode<'a>) -> Option<&'a dyn Annotator> {
    match node.kind {
        ASTKind::Parameter => Some(node.value::<Parameter>()),
        ASTKind::PropertyName => Some(node.value::<PropertyName>()),
        _ => None
    }
}

impl Annotator for PropertyName {
    fn annotate<'a>(&self, node: &'a PTNode<'a>, sink: &mut AnnotationsSink) {
        let parent = node.parent().unwrap();
        let left =
        if parent.children().len() > 1 {
            downcast_to_typed(parent.children()[0]).unwrap().infer_value(parent.children()[0])
        } else {
            Some(entities().global())
        };
        if left.is_none() { return; }
        let left = left.unwrap();
        let left = left.borrow();
        let properties = left.properties();

        let text = node.data;

        for x in properties.keys() {
            if x.starts_with(text) {
                sink.completions.push(x.to_string());
            }
        }
    }
}

impl Annotator for Parameter {
    fn annotate<'a>(&self, node: &'a PTNode<'a>, sink: &mut AnnotationsSink) {
        let idx = node.position();

        // Property call
        let parent = node.parent().unwrap().parent().unwrap();
        let left = parent.children()[0];
        let left = downcast_to_typed(left).unwrap().infer_value(left);
        if left.is_none() { return; }
        let left = left.unwrap();
        let left = left.borrow();
        let callee = left.callee().as_ref();
        if callee.is_none() { return; }
        let callee = callee.unwrap();

        if idx >= callee.arguments.len() { return; }
        let arg = &callee.arguments[idx];

        let me = self.infer_value(node).unwrap();
        let me_ref = me.borrow();

        let value = if me_ref.implicits().contains_key(&Type::Number) {
            me_ref.implicits()[&Type::Number](me.clone())
        } else if me_ref.implicits().contains_key(&Type::String){
            me_ref.implicits()[&Type::String](me.clone())
        } else {
            me.clone().into()
        };

        arg.contributor.contribute(value)
            .iter()
            .for_each(|a| sink.completions.push(a.to_string()));



    }
}
