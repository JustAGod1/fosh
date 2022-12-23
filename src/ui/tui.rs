use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::io;
use std::io::{Read, stdin, Stdout, Write};
use rand::distributions::Open01;
use termion::event::Key;
use termion::input::TermRead;
use termion::is_tty;
use termion::raw::{IntoRawMode, RawTerminal};
use crate::builtin::annotator::downcast_to_annotator;
use crate::builtin::engine::annotator::{AnnotationsSink, Annotator};
use crate::builtin::engine::parse_tree::{parse_line, ParseTree, PTNode};
use crate::parser;
use crate::ui::settings::TUISettings;

macro_rules! csi {
    ($( $l:expr ),*) => { concat!("\x1B[", $( $l ),*) };
}

#[derive(parse_display_derive::Display)]
#[display("{}")]
pub enum CursorMode {
    #[display("\x30")]
    BlinkingBlock,
    #[display("\x31")]
    BlinkingBlockDefault,
    #[display("\x32")]
    SteadyBlock,
    #[display("\x33")]
    BlinkingUnderline,
    #[display("\x34")]
    SteadyUnderline,
    #[display("\x35")]
    BlinkingBar,
    #[display("\x36")]
    SteadyBar,
}

#[derive(parse_display_derive::Display)]
#[display("{}")]
pub enum CSIControlCodes {
    #[display("\x1B[{0}A")]
    CursorUp(usize),

    #[display("\x1B[{0}B")]
    CursorDown(usize),

    #[display("\x1B[{0}C")]
    CursorForward(usize),

    #[display("\x1B[{0}D")]
    CursorBack(usize),

    #[display("\x1B[{0}E")]
    CursorNextLine(usize),

    #[display("\x1B[{0}F")]
    CursorPreviousLine(usize),

    #[display("\x1B[{0}G")]
    CursorHorizontalAbsolute(usize),

    #[display("\x1B[{0};{1}H")]
    CursorPosition(usize, usize),

    #[display("\x1B[{0}J")]
    EraseInDisplay(usize),

    #[display("\x1B[{0}K")]
    EraseInLine(usize),

    #[display("\x1B[{0}S")]
    ScrollUp(usize),

    #[display("\x1B[{0}T")]
    ScrollDown(usize),

    #[display("\x1B[{0};{1}f")]
    HorizontalVerticalPosition(usize, usize),

    #[display("\x1B[{0} q")]
    SetCursorStyle(CursorMode),
}

pub struct TUI<'a> {
    prompt: Cow<'a, str>,
    settings: &'a RefCell<TUISettings>
}

impl<'a> TUI<'a> {
    pub fn new(prompt: Cow<'a, str>, settings: &'a RefCell<TUISettings>) -> Self {
        Self {
            settings,
            prompt,
        }
    }

    pub fn next_line(&mut self) -> Result<Option<String>, io::Error> {
        if true || atty::is(atty::Stream::Stdin) {
            self.next_line_interactive()
        } else {
            self.next_line_bulk()
        }
    }
    fn next_line_bulk(&mut self) -> Result<Option<String>, io::Error> {
        let input = stdin().lock();
        let mut buf = Vec::with_capacity(30);

        let mut read = 0;
        for c in input.bytes() {
            read += 1;
            match c {
                Err(e) => return Err(e),
                Ok(0) | Ok(3) | Ok(4) => return Ok(None),
                Ok(0x7f) => {
                    buf.pop();
                }
                Ok(b'\n') | Ok(b'\r') => break,
                Ok(c) => buf.push(c),
            }
        }

        if read <= 0 { return Ok(None); }

        let string = String::from_utf8(buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Some(string))
    }
    fn next_line_interactive(&mut self) -> Result<Option<String>, io::Error> {
        let mut stdout = match std::io::stdout().into_raw_mode() {
            Ok(o) => o,
            Err(e) => {
                let f = format!("Error: {:?}", e);
                println!("{}", f);
                panic!("Error: {:?}", e);},
        };
        write!(stdout, "{}", self.prompt).unwrap();
        write!(stdout, "{}", CSIControlCodes::SetCursorStyle(CursorMode::SteadyBar)).unwrap();
        stdout.flush()?;

        let mut cursor = 0usize;
        let mut line = String::new();

        let mut stdin = std::io::stdin();

        macro_rules! print_line {
            () => {
                {
                    self.print_cursor_insight(&line, &mut stdout, cursor);
                    self.print_annotated_line(&line, &mut stdout, cursor);
                }
            };
        }
        print_line!();
        for c in stdin.keys() {
            let c = c?;
            match c {
                Key::Ctrl('c') => {
                    return Err(io::Error::new(io::ErrorKind::Interrupted, "Interrupted"));
                }
                Key::Char(c) if c == '\n' => {
                    write!(stdout, "\n\r").unwrap();
                    stdout.flush().unwrap();
                    return Ok(Some(line));
                }
                Key::Char(c) if c != '\n' => {
                    line.insert(cursor, c);
                    cursor += 1;
                    print_line!();
                    stdout.flush()?;
                }
                Key::Right => {
                    if cursor < line.len() {
                        write!(stdout, "{}", CSIControlCodes::CursorForward(1)).unwrap();
                        cursor += 1;
                        stdout.flush()?;
                    }
                }
                Key::Left => {
                    if cursor > 0 {
                        write!(stdout, "{}", CSIControlCodes::CursorBack(1)).unwrap();
                        cursor -= 1;
                        stdout.flush()?;
                    }
                }
                Key::Backspace => {
                    if cursor > 0 {
                        line.remove(cursor - 1);
                        cursor -= 1;
                        print_line!();
                        stdout.flush()?;
                    }
                }

                _ => {}
            }
        }

        drop(stdout);
        println!();

        panic!("Unreachable");
    }

    fn print_cursor_insight(&mut self, line: &str, stdout: &mut RawTerminal<Stdout>, cursor: usize) {
        let tree = parse_line(line);
        if tree.is_none() { return; }
        let tree = tree.unwrap();

        let mut nodes = Vec::new();
        tree.collect(&mut nodes, |a| a.origin.span.start() <= cursor && a.origin.span.end() >= cursor);

        let mut sink = AnnotationsSink::new();

        for node in nodes {
            if let Some(annotator) = downcast_to_annotator(node) {
                annotator.annotate(node, &mut sink);
            }
        }
        write!(stdout, "\r{}", CSIControlCodes::EraseInDisplay(0)).unwrap();
        if !sink.completions.is_empty() {
            write!(stdout, "\n\r").unwrap();
            write!(stdout, "Completions: \n\r").unwrap();
            for completion in sink.completions.iter() {
                write!(stdout, "  {}\n\r", completion).unwrap();
            }

            write!(stdout, "{}\r", CSIControlCodes::CursorUp(sink.completions.len() + 2)).unwrap();
        }
    }

    fn print_annotated_line(&self, line: &str, stdout: &mut RawTerminal<Stdout>, cursor: usize) {
        let tree = parse_line(line);
        if tree.is_none() { return; }
        let tree = tree.unwrap();

        let highlighted = self.highlight_command(&tree, line);


        write!(stdout, "{}{}{}{}{}",
               CSIControlCodes::CursorHorizontalAbsolute(1),
               CSIControlCodes::EraseInLine(0),
               self.prompt,
               highlighted,
               CSIControlCodes::CursorHorizontalAbsolute(cursor as usize + self.prompt.len() + 1),
        ).unwrap();

        stdout.flush().unwrap();

    }

    fn highlight_command<'b>(&self, tree: &'b ParseTree<'b>, line: &str) -> String {
        let node = tree.root();
        let command = line;
        let mut insertions = HashMap::<usize, Vec<String>>::new();
        let mut result = String::new();

        node.walk(&mut |node| {
            let sink = self.run_annotator_on_node(node);

            let v = &node.origin.value;

            insertions.entry(node.origin.span.start()).or_insert(Vec::new())
                .push(v.kind().color_string());

            for x in sink.colors() {
                insertions.entry(node.origin.span.start()).or_insert(Vec::new())
                    .push(self.settings.borrow().color_scheme().get(x).to_string());
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

    fn run_annotator_on_node<'b>(&self, node: &'b PTNode<'b>) -> AnnotationsSink {
        let mut sink = AnnotationsSink::new();
        if let Some(annotator) = downcast_to_annotator(node) {
            annotator.annotate(node, &mut sink);
        }

        sink
    }
}


