#![allow(unused_imports)]

extern crate core;

mod tui;
mod parser;
mod builtin;
mod runtime;

use std::collections::HashMap;
use nix::unistd;

use std::io::{stdin, stdout, Write};
use termion::event::Key;
use termion::input::{TermRead};
use termion::raw::IntoRawMode;
use crate::builtin::annotator::EntityAnnotator;
use crate::builtin::engine::entities::EntitiesManager;
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

    let entities = EntitiesManager::new();
    let mut tui = TUI::new("$ ".to_string());

    tui.register_annotator(EntityAnnotator::new(&entities));

    tui.run().unwrap();
}

