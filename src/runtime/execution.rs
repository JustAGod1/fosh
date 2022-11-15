use std::borrow::Borrow;
use crate::builtin::engine::parse_tree::PTNode;
use crate::parser::ast::{ASTKind, downcast_to_typed};
use crate::{EntitiesManager, TUI};
use crate::builtin::engine::entities::{Entity, EntityExecutionError, Execution, ExecutionConfig};
use crate::builtin::engine::Value;

pub fn execute<'a>(command: &'a PTNode<'a>, tui: &TUI) {
    execute_delimited(command, tui);
}

fn execute_delimited<'a>(command: &'a PTNode<'a>, tui: &TUI) {
    if command.kind != ASTKind::Delimited {
        execute_sequenced(command, tui);
    }
}

fn execute_sequenced<'a>(command: &'a PTNode<'a>, tui: &TUI) {
    if command.kind != ASTKind::Sequenced {
        execute_piped(command, tui);
    }
}

fn execute_piped<'a>(command: &'a PTNode<'a>, tui: &TUI) {
    if command.kind != ASTKind::Piped {
        execute_command_or_function(command, tui);
    }
}

fn execute_command_or_function<'a>(command: &'a PTNode<'a>, tui: &TUI) {
    match command.kind {
        ASTKind::Command | ASTKind::Function => {
            match downcast_to_typed(command).unwrap().infer_value(command) {
                None => {
                    println!("No value inferred for command {}", command.data);
                }
                Some(Ok(e)) => {
                    match e.callee() {
                        None => {
                            println!("execution not found")
                        }
                        Some(exe) => {
                            match (exe.callee)(&[], ExecutionConfig {
                                std_in: None,
                                std_out: None,
                                std_err: None,
                            }) {
                                Ok(mut e) => {
                                    match e.execute() {
                                        Ok(entity) => {
                                            println!("{}", entity);
                                        },
                                        Err(err) => {
                                            println!("{:?}", err);

                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("{:?}", e)
                                }
                            };
                        }
                    }
                }
                Some(Err(e)) => {}
            };
        }
        _ => { panic!("Expected command or function, got {:?}", command.kind) }
    }
}

