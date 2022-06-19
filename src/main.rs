#![allow(unused_imports)]

mod tui;
mod parser;
mod completer;
mod builtin;

use std::collections::HashMap;
use nix::unistd;

use std::io::{stdin, stdout, Write};
use termion::event::Key;
use termion::input::{TermRead};
use termion::raw::IntoRawMode;
use crate::tui::TUI;

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

