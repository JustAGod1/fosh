use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::ops::Range;

#[derive(parse_display_derive::Display)]
#[derive(Debug)]
pub enum ErrorType {
    Syntax,
    Semantic,
    Execution,
}

pub struct ErrorReport<'a> {
    span: Range<usize>,
    text: &'a str,

    hints: Vec<Cow<'a, str>>,
    notes: Vec<Cow<'a, str>>,

    error_type: ErrorType,
}

impl<'a> ErrorReport<'a> {
    pub fn new(span: Range<usize>, text: &'a str, error_type: ErrorType) -> Self {
        Self {
            span,
            text,
            hints: Vec::new(),
            error_type,
            notes: Vec::new(),
        }
    }

    pub fn add_hint<S : Into<Cow<'a, str>>>(&mut self, hint: S) {
        self.hints.push(hint.into());
    }

    pub fn add_note<S : Into<Cow<'a, str>>>(&mut self, note: S)  {
        self.notes.push(note.into());
    }
}

impl<'a> Display for ErrorReport<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "error: {}\n", self.error_type)?;
        if self.text.len() <= 80 {
            print_with_highlight(self.text, self.span.clone(), f)?;
        } else {
            let delta = (80 -(self.span.end - self.span.start + 1)).max(0);

            let start = self.span.start.checked_sub(delta).unwrap_or(0);
            let end = (self.span.end + delta).min(self.text.len());

            print_with_highlight(&self.text[start..end], self.span.start - start..self.span.end - start, f)?;
        }

        for hint in &self.hints {
            writeln!(f, "hint: {}", hint)?;
        }
        for hint in &self.notes {
            writeln!(f, "note: {}", hint)?;
        }

        Ok(())

    }
}

// print given text and underline the span with ^^^^^
fn print_with_highlight(text: &str, span: Range<usize>, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}\n", text)?;

    for i in 0..text.len() {
        if span.start <= i && i < span.end {
            write!(f, "^")?;
        } else {
            write!(f, " ")?;
        }
    }
    write!(f, "\n")


}