#![allow(unused_imports)]

extern crate core;

mod ui;
mod parser;
mod builtin;
mod runtime;

use std::cell::RefCell;
use std::collections::HashMap;
use nix::unistd;

use std::io::{stdin, stdout, Write};
use termion::event::Key;
use termion::input::{TermRead};
use termion::is_tty;
use termion::raw::IntoRawMode;
use crate::builtin::annotator::{EntitiesAnnotator, PathAnnotator};
use crate::builtin::engine::entities::{EntitiesManager, EntityRef};
use crate::builtin::engine::parse_tree::parse_line;
use crate::builtin::entities::initialize_universe;
use crate::runtime::execution::execute;
use crate::ui::settings::TUISettings;
use crate::ui::tui::TUI;

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

static mut ENTITIES: Option<EntitiesManager> = None;

pub fn entities() -> &'static EntitiesManager {
    unsafe {
        ENTITIES.as_ref().unwrap()
    }
}

fn main() {

    if is_tty(&stdin()) {
        if let Err(e) = set_unique_pid() {
            eprintln!("Failed to grab tty: {}", e);
        }
    }


    unsafe {
        ENTITIES = Some(EntitiesManager::new());
    }
    let settings = RefCell::new(TUISettings::new());

    initialize_universe(entities());

    let mut tui = TUI::new("$ ".into(), &settings);

    tui.register_annotator(PathAnnotator::new());
    tui.register_annotator(EntitiesAnnotator::new(entities()));

    loop {
        let line = tui.next_line().unwrap();
        let tree = parse_line(&line).unwrap();

        match execute(tree.root(), &tui) {
            Ok(entity) => {
                println!("Entity: {}", entity.borrow());
            }
            Err(err) => {
                println!("Error: {:?}", err);
            }
        }
    }
}

