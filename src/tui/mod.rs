pub mod settings;

use std::collections::HashMap;
use std::io;
use std::io::Write;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use crate::{parser};
use crate::builtin::engine::annotator::{AnnotationsSink, Annotator, AnnotatorsManager};
use crate::builtin::engine::parse_tree::{ParseTree, PTNode};
use crate::parser::ast::ASTNode;
use crate::tui::settings::TUISettings;

pub struct TUI<'a> {
    prompt: String,
    command: String,
    annotators: AnnotatorsManager<'a>,
    cursor_pos: usize,
    settings: TUISettings,
}


macro_rules! csi {
    ($( $l:expr ),*) => { concat!("\x1B[", $( $l ),*) };
}

impl <'a>TUI<'a> {
    pub fn new(prompt: String) -> Self {


        let result = Self {
            prompt,
            command: String::new(),
            annotators: AnnotatorsManager::new(),
            cursor_pos: 0,
            settings: TUISettings::new(),
        };

        result
    }

    pub fn register_annotator<T : 'a + Annotator>(&mut self, annotator: T) {
        self.annotators.register_annotator(Box::new(annotator));
    }

    pub fn run(&mut self) -> Result<(), io::Error> {
        let mut stdout = std::io::stdout().into_raw_mode().unwrap();
        stdout.flush().unwrap();
        let stdin = std::io::stdin();


        for c in stdin.keys() {
            match c? {
                Key::Ctrl('c') => {
                    println!("\n\rExiting...");
                    stdout.write(b"\r").unwrap();
                    return Ok(());
                }
                Key::Backspace => {
                    if self.command.len() > 0 {
                        self.command.pop();
                        self.cursor_pos=self.cursor_pos.checked_sub(1).unwrap_or(0);
                        self.update_data(&mut stdout);
                    }
                }
                Key::Char(c) => {
                    if c == '\n' {
                        self.cursor_pos=0;
                        self.command.clear();
                        self.update_data(&mut stdout);
                        write!(stdout, "\n\r").unwrap();
                    } else {
                        self.cursor_pos+=1;
                        stdout.write(c.to_string().as_bytes()).unwrap();
                        self.command.push(c);

                        self.update_data(&mut stdout);
                    }
                }
                _ => {}
            }

            stdout.flush().unwrap();
        }

        Ok(())
    }

    fn update_data(&self, stdout: &mut std::io::Stdout) {
        let ast = self.parse_command().unwrap();

        let insight = self.cursor_insight(&ast);
        let highlighted = self.highlight_command(&ast);

        let mut shift = 0;

        write!(stdout, "\r{}", csi!("0J")).unwrap();
        if let Some(insight) = insight {
            for x in insight.completions() {
                shift += 1;
                write!(stdout, "\n\r{}", x).unwrap();
            }
        }
        if shift > 0 {
            write!(stdout, "{}", termion::cursor::Up(shift)).unwrap();
        }
        write!(stdout, "\r{}{}", self.prompt, highlighted).unwrap();
    }

    fn parse_command(&self) -> Option<ParseTree> {
        let ast = parser::parse(&self.command);
        if ast.is_err() {
            return None;
        }
        let tree = ParseTree::new(&self.command, ast.unwrap());

        return Some(tree);
    }

    fn cursor_insight(&self, tree: &'a ParseTree<'a>) -> Option<AnnotationsSink> {
        let cursor_pos = if self.cursor_pos == self.command.len() {
            self.cursor_pos.checked_sub(1).unwrap_or(0)
        } else {
            self.cursor_pos
        };
        let tree = tree.root();

        if let Some(n) = tree.find_leaf_on_pos(cursor_pos) {
            let mut sink = AnnotationsSink::new();
            self.annotators.annotate(n, &mut sink);
            Some(sink)
        } else {
            None
        }
    }

    fn run_annotator_on_node(&self, node: &'a PTNode<'a>) -> AnnotationsSink {
        let mut sink = AnnotationsSink::new();
        self.annotators.annotate(node, &mut sink);

        sink
    }


    fn highlight_command(&self, tree: &'a ParseTree<'a>) -> String {
        let node = tree.root();
        let command = &self.command;
        let mut insertions = HashMap::<usize, Vec<String>>::new();
        let mut result = String::new();

        node.walk(&mut |node| {
            let sink = self.run_annotator_on_node(node);

            let v = &node.origin.value;

            insertions.entry(node.origin.span.start()).or_insert(Vec::new())
                .push(v.kind().color_string());

            for x in sink.colors() {
                insertions.entry(node.origin.span.start()).or_insert(Vec::new())
                    .push(self.settings.color_scheme().get(x).to_string());
            }

            insertions.entry(node.origin.span.end()).or_insert(Vec::new())
                .push(termion::color::Fg(termion::color::Reset).to_string());
        });


        result.push_str(&termion::color::Bg(termion::color::Reset).to_string());
        result.push_str(&termion::color::Fg(termion::color::Reset).to_string());
        for (i, s) in command.chars().enumerate() {
            if let Some(insertions) = insertions.get(&i) {
                for ins in insertions {
                    result.push_str(&ins);
                }
            }
            result.push(s);
        }
        result.push_str(&termion::color::Bg(termion::color::Reset).to_string());
        result.push_str(&termion::color::Fg(termion::color::Reset).to_string());

        result
    }
}

