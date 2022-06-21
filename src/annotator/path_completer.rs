use std::borrow::{Borrow, BorrowMut};
use std::cell::{RefCell};
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::sync::atomic::AtomicBool;
use crate::annotator::{Annotator, AnnotatorContext};
use crate::annotator::parse_tree::PTNode;
use crate::parser::ast::ASTKind;

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

