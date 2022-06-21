use std::ops::Index;
use std::rc::Rc;
use crate::annotator::{Annotator, AnnotatorContext};
use crate::annotator::parse_tree::PTNode;
use crate::builtin::{EntitiesManager, Entity, Value};
use crate::parser::ast::{ASTKind, CallChain, Identifier, NumberLiteral, ParenthesizedArgumentsList, PropertyCall, StringLiteral};

pub struct EntityAnnotator<'b> {
    entities: &'b EntitiesManager,
}

impl<'b> EntityAnnotator<'b> {
    pub fn new(entities: &'b EntitiesManager) -> Self {
        Self { entities }
    }
}

impl<'b> Annotator for EntityAnnotator<'b> {
    fn annotate<'a>(&self, node: &'a PTNode<'a>, context: &mut AnnotatorContext) {
        match node.kind {
            ASTKind::PropertyName => self.annotate_property_name(node, context),
            ASTKind::StringLiteral | ASTKind::NumberLiteral | ASTKind::Identifier =>
                self.annotate_entity_argument(node, context),
            _ => {}
        }
    }
}

fn pt_to_value<'a>(node: &'a PTNode<'a>) -> Option<Result<Value, String>> {
    match node.kind {
        ASTKind::Identifier => Some(Ok(node.value::<Identifier>().get_value(node))),
        ASTKind::StringLiteral => Some(Ok(node.value::<StringLiteral>().get_value(node))),
        ASTKind::NumberLiteral => Some(node.value::<NumberLiteral>().get_value(node)),
        _ => None,
    }
}

impl<'b> EntityAnnotator<'b> {
    fn find_left_hand_entity<'a>(&self, node: &'a PTNode<'a>) -> Result<Rc<dyn Entity>, String> {
        let call = node.find_parent_with_kind(ASTKind::PropertyCall).unwrap();

        if call.parent().map(|x| x.kind) == Some(ASTKind::CallChain) {
            let call_chain = call.parent().unwrap();

            self.entities
                .infer_from_pt(call_chain.value::<CallChain>().get_left_hand(call_chain).unwrap())
        } else {
            Ok(self.entities.global())
        }
    }

    fn annotate_entity_argument<'a>(&self, node: &'a PTNode<'a>, context: &mut AnnotatorContext) {
        let value = pt_to_value(node).unwrap();
        if value.is_err() {
            context.sink().add_error(value.err());
            return;
        }
        let value = value.unwrap();

        let entity = self
            .get_target_property(node.find_parent_with_kind(ASTKind::PropertyCall).unwrap());

        if entity.is_none() {
            return;
        }
        let entity = entity.unwrap();

        let idx = node.position() - 1;

        let arg = entity.args().get(idx);
        if arg.is_none() { return; }
        let arg = arg.unwrap();

        for x in arg.contributor.contribute(value) {
            context.sink().add_completion(x.to_string());
        }
    }

    fn get_target_property<'a>(&self, node: &'a PTNode<'a>) -> Option<Rc<dyn Entity>> {
        let left_entity = self.find_left_hand_entity(node);
        if left_entity.is_err() { return None; }
        let left_entity = left_entity.unwrap();

        let name = node.find_child_with_kind_rec(ASTKind::PropertyName).unwrap().data;

        let properties = left_entity.get_properties();
        properties.get(name).cloned()
    }

    fn annotate_property_name<'a>(&self, node: &'a PTNode<'a>, context: &mut AnnotatorContext) {
        let left_entity = self.find_left_hand_entity(node);
        if left_entity.is_err() { return; }
        let left_entity = left_entity.unwrap();

        let name = node.data;
        let properties = left_entity.get_properties();
        if !properties.contains_key(name) {
            context.sink().add_error(Some(format!("No such property {} on {}", node.data, left_entity)));
        }

        for x in properties.keys() {
            if x.starts_with(name) {
                context.sink().add_completion(x.to_string());
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::annotator::{AnnotationsSink, Annotator, AnnotatorContext};
    use crate::annotator::tests::get_annotations;
    use crate::builtin::EntitiesManager;
    use crate::EntityAnnotator;
    use crate::tui::settings::ColorType;

    pub fn annotate_with_default(s: &str) -> AnnotationsSink {
        let manager = EntitiesManager::new();
        let annotator = EntityAnnotator::new(&manager);

        get_annotations(s, vec![Box::new(annotator)])
    }

    #[test]
    fn global() {
        let annotations = annotate_with_default("$ lo^l");
        assert_eq!(annotations.colors(), &vec![ColorType::Error]);


        let annotations = annotate_with_default("$ ^c");
        assert_eq!(annotations.colors(), &vec![ColorType::Error]);
        assert_eq!(annotations.completions(), &vec!["cd".to_string()]);
    }

    #[test]
    fn test_ill_format() {
        let annotations = annotate_with_default(r#"$ c^d("fk")."#);

        assert_eq!(annotations.colors(), &vec![]);
    }
}