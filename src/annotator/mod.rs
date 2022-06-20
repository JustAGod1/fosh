use std::fmt::Display;
use crate::annotator::parse_tree::PTNode;
use crate::annotator::path_completer::PathAnnotator;
use crate::tui::settings::ColorType;

pub mod parse_tree;
pub mod path_completer;

pub struct AnnotatorsManager<'a> {
    annotators: Vec<Box<dyn Annotator + 'a>>,
}

impl <'a>AnnotatorsManager<'a> {
    pub fn new() -> Self {
        let mut annotators: Vec<Box<dyn Annotator>> = Vec::new();
        annotators.push(Box::new(PathAnnotator::new()));
        Self { annotators }
    }

    pub fn annotate(&self, node: &'a PTNode<'a>, sink: &mut AnnotationsSink) {
        let mut context = AnnotatorContext::new(sink);
        for annotator in &self.annotators {
            annotator.annotate(node, &mut context);
        }
    }

    pub fn register_annotator(&mut self, annotator: Box<dyn Annotator + 'a>) {
        self.annotators.push(annotator);
    }
}


pub trait Annotator {
    fn annotate<'a>(&self, node: &'a PTNode<'a>, context: &mut AnnotatorContext);
}

pub struct AnnotationsSink {
    completions: Vec<String>,
    colors: Vec<ColorType>,
    hints: Vec<String>,
}

impl AnnotationsSink {
    pub fn new() -> Self {
        Self {
            completions: Vec::new(),
            colors: Vec::new(),
            hints: Vec::new(),
        }
    }

    pub fn add_completion<S: Into<String>>(&mut self, completion: S) {
        self.completions.push(completion.into());
    }

    pub fn add_error<S : Display>(&mut self, error: Option<S>) {
        if let Some(error) = error {
            self.hints.push(format!("Error: {}", error));
        }
        self.colors.push(ColorType::Error);

    }

    pub fn add_color(&mut self, color: ColorType) {
        self.colors.push(color);
    }

    pub fn add_hint<S: Into<String>>(&mut self, hint: S) {
        self.hints.push(hint.into());
    }


    pub fn completions(&self) -> &Vec<String> {
        &self.completions
    }
    pub fn colors(&self) -> &Vec<ColorType> {
        &self.colors
    }
    pub fn hints(&self) -> &Vec<String> {
        &self.hints
    }
}


pub struct AnnotatorContext<'a> {
    sink: &'a mut AnnotationsSink
}

impl <'a>AnnotatorContext<'a> {
    pub fn new(sink: &'a mut AnnotationsSink) -> Self {
        Self { sink }
    }

    pub fn sink(&mut self) -> &mut AnnotationsSink {
        self.sink
    }
}

#[cfg(test)]
pub mod tests {
    use crate::annotator::{AnnotationsSink, Annotator, AnnotatorsManager};
    use crate::parser::tests::build_pt_def;

    pub fn get_annotations<'a>(s: &str, annotators: Vec<Box<dyn Annotator + 'a>>) -> AnnotationsSink {
        let mut manager = AnnotatorsManager::new();
        for x in annotators {
            manager.register_annotator(x)
        }

        let replaced = s.replace("^", "");
        let pt = build_pt_def(replaced.as_str());
        let pt = pt.root();

        let mut sink = AnnotationsSink::new();
        let node = pt.find_leaf_on_pos(s.find("^").unwrap()).unwrap();
        manager.annotate(node, &mut sink);

        sink
    }

}