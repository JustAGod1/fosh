use std::ops::Index;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use crate::builtin::engine::annotator::{Annotator, AnnotatorContext};
use crate::builtin::engine::entities::EntitiesManager;
use crate::builtin::engine::parse_tree::PTNode;
use crate::builtin::engine::Value;
use crate::parser::ast::{ASTKind, Identifier, NumberLiteral, ParenthesizedArgumentsList, PropertyCall, StringLiteral};

pub struct PathAnnotator {
    names: Arc<Mutex<Vec<String>>>,
}

impl Annotator for PathAnnotator {
    fn annotate<'a>(&self, node: &'a PTNode<'a>, context: &mut AnnotatorContext) {
        if node.kind != ASTKind::CommandName {
            return;
        }
        let names = self.names.lock().unwrap();
        let mut result = Vec::new();

        let text = node.text();
        for name in names.iter() {
            if name.starts_with(text) {
                result.push(name.clone());
                if result.len() >= 5 { break; }
            }
        }

        for x in result {
            context.sink.completions.push(x)
        }
    }
}

impl PathAnnotator {
    pub fn new() -> Self {
        let r = Self {
            names: Default::default(),
        };
        let arc = r.names.clone();
        std::thread::spawn(move || {
            Self::update_cache(arc)
        });

        return r;
    }

    fn update_cache(weak: Arc<Mutex<Vec<String>>>) {
        if let Some(v) = std::env::var_os("PATH").map(|v| v.to_string_lossy().to_string()) {
            for x in v.split(":") {
                if let Ok(v) = std::fs::read_dir(x) {
                    for entry in v {
                        if let Ok(entry) = entry {
                            if let Ok(name) = entry.file_name().into_string() {
                                if let Ok(meta) = entry.metadata() {
                                    if meta.file_type().is_file() {
                                        let mut names = weak.lock().unwrap();
                                        names.push(name);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}


#[cfg(test)]
pub mod tests {
    use crate::builtin::annotator::PathAnnotator;
    use crate::builtin::contributors::FilesContributor;
    use crate::builtin::engine::annotator::AnnotationsSink;
    use crate::builtin::engine::annotator::tests::get_annotations;
    use crate::builtin::engine::entities::EntitiesManager;
    use crate::tui::settings::ColorType;

    pub fn annotate_with_default(s: &str) -> AnnotationsSink {
        let manager = EntitiesManager::new();
        let annotator = PathAnnotator::new();

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

    #[test]
    fn test_error_complete() {
        let annotations = annotate_with_default(r#"$^"#);

        assert_eq!(annotations.colors(), &vec![]);
    }
}