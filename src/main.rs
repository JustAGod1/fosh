#![allow(unused_imports)]

extern crate core;

mod ui;
mod parser;
mod builtin;
mod runtime;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use nix::unistd;

use std::io::{stderr, stdin, stdout, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use nix::libc::dup;
use termion::event::Key;
use termion::input::{TermRead};
use termion::is_tty;
use termion::raw::IntoRawMode;
use fosh::error_printer::ErrorReport;
use crate::builtin::engine::entities::{EntitiesManager, EntityExecutionError, EntityRef, ExecutionConfig};
use crate::builtin::engine::parse_tree::{parse_line, PTNode, PTNodeId};
use crate::builtin::entities::initialize_universe;
use crate::parser::ast::ASTKind;
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

pub fn construct_error_report<'a, 'b>(s: &'b str, root: &'a PTNode<'a>, error: &EntityExecutionError) -> Vec<ErrorReport<'b>> {
    let mut reports = Vec::new();

    for (node_id, msg) in &error.errors {
        let node = root.find_node(*node_id).unwrap();
        let mut report = ErrorReport::new(
            node.origin.span.as_range(),
            s,
            msg.kind
        );

        for note in &msg.notes {
            report.add_note(note.to_owned());
        }

        for hint in &msg.hints {
            report.add_hint(hint.to_owned());
        }

        reports.push(report);
    }

    reports
}

pub fn report<'a>(root: &'a PTNode<'a>, error: &EntityExecutionError) {
    let reports = construct_error_report(root.data, root, error);
    for report in reports {
        println!("{}", report);
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

    let mut tui = TUI::new(">> ".into(), &settings);

    loop {
        let line = tui.next_line().unwrap();
        if line.is_none() { break; }
        let line = line.unwrap();
        if line.is_empty() { continue; }
        let tree = parse_line(&line).unwrap();

        if tree.root().find_child_with_kind_rec(ASTKind::Error).is_some() {
            println!("Syntax error");
            continue;
        }

        let config = ExecutionConfig {
            std_in: None,
            std_out: None,
            std_err: None,
            pt: tree.root().id()
        };
        match execute(tree.root(), &config).execute() {
            Ok(entity) => {
                println!("Entity: {}", entity.borrow());
            }
            Err(err) => {
                let reports = construct_error_report(&line, tree.root(), &err);
                for report in reports {
                    println!("{}", report);
                }
            }
        }
    }
}

