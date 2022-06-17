use std::collections::HashMap;
use std::io;
use std::io::Write;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use crate::{completer, parser};
use crate::completer::CompleterManager;
use crate::completer::parse_tree::ParseTreeBuilder;
use crate::parser::ast::ASTNode;

pub struct TUI {
    prompt: String,
    command: String,
    completer: CompleterManager,
    cursor_pos: usize,
}


macro_rules! csi {
    ($( $l:expr ),*) => { concat!("\x1B[", $( $l ),*) };
}

impl TUI {
    pub fn new(prompt: String) -> Self {
        Self {
            prompt,
            command: String::new(),
            completer: CompleterManager::new(),
            cursor_pos: 0,
        }
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
                        self.update_data(&mut stdout);
                    }
                }
                Key::Char(c) => {
                    if c == '\n' {
                        self.command.clear();
                        self.update_data(&mut stdout);
                        write!(stdout, "\n\r").unwrap();
                    } else {
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
        let ast = self.parse_command();
        let highlighted = self.highlight_command(&ast);
        let completions = self.find_completions(&ast);
        let mut shift = 0;

        write!(stdout, "\r{}", csi!("0J")).unwrap();
        for x in completions {
            shift += 1;
            write!(stdout, "\n\r{}", x).unwrap();
        }
        if shift > 0 {
            write!(stdout, "{}", termion::cursor::Up(shift)).unwrap();
        }
        write!(stdout, "\r{}{}", self.prompt, highlighted).unwrap();
    }

    fn parse_command(&self) -> ASTNode {
        let mut error = false;
        let node = parser::CmdParser::new().parse(&mut error, &self.command).unwrap();

        return node;
    }

    fn find_completions(&self, node: &ASTNode) -> Vec<String> {
        let builder = ParseTreeBuilder::new(&self.command);
        let tree = builder.parse_ast(node);


        if let Some(n) = tree.find_leaf_on_pos(self.cursor_pos) {
            self.completer.complete(&n)
        } else {
            return Vec::new();
        }
    }


    fn highlight_command(&self, node: &ASTNode) -> String {
        let command = &self.command;
        let mut insertions = HashMap::<usize, Vec<String>>::new();
        let mut result = String::new();

        node.walk(&mut |node| {
            let v = &node.value;
            insertions.entry(node.span.start()).or_insert(Vec::new())
                .push(v.kind().color_string());
            insertions.entry(node.span.end()).or_insert(Vec::new())
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

