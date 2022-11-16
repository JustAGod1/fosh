use std::fmt::Display;
use crate::builtin::engine::parse_tree::PTNode;
use crate::ui::settings::ColorType;

pub trait Annotator {
    fn annotate<'a>(&self, node: &'a PTNode<'a>, context: &mut AnnotationsSink);
}

pub struct AnnotationsSink {
    pub completions: Vec<String>,
    pub colors: Vec<ColorType>,
    pub hints: Vec<String>,
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
    pub sink: &'a mut AnnotationsSink
}

impl <'a>AnnotatorContext<'a> {
    pub fn new(sink: &'a mut AnnotationsSink) -> Self {
        Self { sink }
    }

    pub fn sink(&mut self) -> &mut AnnotationsSink {
        self.sink
    }
}
