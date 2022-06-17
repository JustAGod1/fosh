mod tui;
mod parser;
mod completer;

use std::collections::HashMap;
use nix::unistd;

use std::io::{stdin, stdout, Write};
use termion::event::Key;
use termion::input::{TermRead};
use termion::raw::IntoRawMode;
use crate::tui::TUI;

fn open_io() -> (std::io::Stdin, termion::raw::RawTerminal<std::io::Stdout>) {
    (stdin(), stdout().into_raw_mode().unwrap())
}

fn set_unique_pid() -> nix::Result<()> {
    let pgid = unistd::getpid();
    if pgid != unistd::getpgrp() {
        unistd::setpgid(pgid, pgid)?;
    }
    if pgid != unistd::tcgetpgrp(nix::libc::STDIN_FILENO)? {
        unistd::tcsetpgrp(nix::libc::STDIN_FILENO, pgid)?;
    }
    Ok(())
}

fn main() {
    set_unique_pid().unwrap();

    TUI::new("$ ".to_string()).run().unwrap();


}

