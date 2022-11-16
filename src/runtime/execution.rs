use std::borrow::Borrow;
use std::cell::RefCell;
use fosh::error_printer::ErrorType;
use crate::builtin::engine::parse_tree::PTNode;
use crate::parser::ast::{ASTKind, downcast_to_typed};
use crate::{entities, EntitiesManager, TUI};
use crate::builtin::engine::entities::{Entity, EntityExecutionError, EntityRef, Execution, ExecutionConfig, FoshEntity};
use crate::builtin::engine::Value;

pub fn execute<'a>(command: &'a PTNode<'a>, tui: &TUI) -> Result<EntityRef, EntityExecutionError> {
    execute_delimited(command, tui)
}

fn execute_delimited<'a>(command: &'a PTNode<'a>, tui: &TUI) -> Result<EntityRef, EntityExecutionError> {
    if command.kind != ASTKind::Delimited {
        execute_sequenced(command, tui)
    } else {
        Err(EntityExecutionError::new())
    }
}

fn execute_sequenced<'a>(command: &'a PTNode<'a>, tui: &TUI) -> Result<EntityRef, EntityExecutionError> {
    if command.kind != ASTKind::Sequenced {
        execute_piped(command, tui)
    } else {
        Err(EntityExecutionError::new())
    }
}

fn execute_piped<'a>(command: &'a PTNode<'a>, tui: &TUI) -> Result<EntityRef, EntityExecutionError> {
    if command.kind != ASTKind::Piped {
        execute_command_or_function(command, tui)
    } else {
        Err(EntityExecutionError::new())
    }
}

fn execute_command_or_function<'a>(command: &'a PTNode<'a>, tui: &TUI) -> Result<EntityRef, EntityExecutionError> {
    match command.kind {
        ASTKind::Function => {
            execute_function(command, tui)
        }
        ASTKind::Command => {
            return match downcast_to_typed(command).unwrap().infer_value(command) {
                None => {
                    Err(EntityExecutionError::new_single(command.id(), ErrorType::Semantic,"Could not infer execution value"))
                }
                Some(e) => {
                    match RefCell::borrow(&e).callee() {
                        None => {
                            Err(EntityExecutionError::new_single(command.id(), ErrorType::Semantic,"No callee"))
                        }
                        Some(exe) => {
                            match (exe.callee)(e.clone(), &[], ExecutionConfig {
                                std_in: None,
                                std_out: None,
                                std_err: None,
                            }) {
                                Ok(mut e) => {
                                    match e.execute() {
                                        Ok(entity) => {
                                            Ok(entity)
                                        }
                                        Err(err) => {
                                            Err(err)
                                        }
                                    }
                                }
                                Err(e) => {
                                    Err(e)
                                }
                            }
                        }
                    }
                }
            };
        }
        _ => { panic!("Expected command or function, got {:?}", command.kind) }
    }
}

fn execute_function<'a>(command: &'a PTNode<'a>, tui: &TUI) -> Result<EntityRef, EntityExecutionError> {
    let node = command.children()[1];
    execute_value(node, tui)
}

fn execute_value<'a>(node: &'a PTNode<'a>, tui: &TUI) -> Result<EntityRef, EntityExecutionError> {
    match node.kind {
        ASTKind::StringLiteral | ASTKind::NumberLiteral => {
            execute_primitive(node)
        }
        ASTKind::BracedCommand => {
            execute_braced_command(node, tui)
        }
        ASTKind::PropertyInsn => {
            execute_property_insn(node, tui)
        }
        ASTKind::PropertyCall => {
            execute_property_call(node, tui)
        }
        _ => {
            panic!("Unexpected function node {:?}", node.kind)
        }
    }
}

fn execute_property_insn<'a>(command: &'a PTNode<'a>, tui: &TUI) -> Result<EntityRef, EntityExecutionError> {
    let (left, name) = if command.children().len() > 1 {
        (execute_value(command.children()[0], tui)?, command.children()[2])
    } else {
        (entities().global(), command.children()[0])
    };

    let x = RefCell::borrow(&left);
    match x.properties().get(name.data) {
        None => {
            Err(EntityExecutionError::new_single(command.id(), ErrorType::Semantic, format!("Property {} does not exist in {}", name.data, left.name())))
        }
        Some(e) => {
            Ok(e.clone())
        }
    }
}
fn execute_property_call<'a>(command: &'a PTNode<'a>, tui: &TUI) -> Result<EntityRef, EntityExecutionError> {
    let left = execute_property_insn(command.children()[0], tui)?;
    let parenthesis = command.children()[1];

    let mut args = Vec::new();
    for x in parenthesis.children().iter().skip(1) {
        if x.kind == ASTKind::Parameter {
            args.push(execute_value(x.children()[0], tui)?);
        }
    }

    let x = RefCell::borrow(&left);
    match x.callee() {
        None => {
            Err(EntityExecutionError::new_single(command.id(), ErrorType::Semantic, format!("Property {} is not callable", left.name())))
        }
        Some(exe) => {
            match (exe.callee)(left.clone(), &args, ExecutionConfig {
                std_in: None,
                std_out: None,
                std_err: None,
            }) {
                Ok(mut e) => {
                    match e.execute() {
                        Ok(entity) => {
                            Ok(entity)
                        }
                        Err(err) => {
                            Err(err)
                        }
                    }
                }
                Err(e) => {
                    Err(e)
                }
            }
        }
    }

}
fn execute_braced_command<'a>(command: &'a PTNode<'a>, tui: &TUI) -> Result<EntityRef, EntityExecutionError> {
    let node = command.children()[1];
    execute_delimited(node, tui)
}

fn execute_primitive<'a>(command: &'a PTNode<'a>) -> Result<EntityRef, EntityExecutionError> {
    return match downcast_to_typed(command).unwrap().infer_value(command) {
        None => {
            Err(EntityExecutionError::new_single(command.id(), ErrorType::Semantic,"Could not infer value"))
        }
        Some(e) => Ok(e)
    };
}
