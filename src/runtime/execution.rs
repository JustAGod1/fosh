use std::borrow::Borrow;
use std::cell::RefCell;
use fosh::error_printer::ErrorType;
use crate::builtin::engine::parse_tree::PTNode;
use crate::parser::ast::{ASTKind, downcast_to_typed};
use crate::{construct_error_report, entities, EntitiesManager, report, TUI};
use crate::builtin::engine::entities::{Entity, EntityExecutionError, EntityRef, Execution, ExecutionConfig, FoshEntity, PseudoExecution};
use crate::builtin::engine::{Argument, Value};

type ExecutionRef = Box<dyn Execution>;
type FoshResult<A> = Result<A, EntityExecutionError>;

pub fn execute<'a>(command: &'a PTNode<'a>, tui: &TUI) -> FoshResult<EntityRef> {
    execute_delimited(command, tui)
}

fn execute_delimited<'a>(command: &'a PTNode<'a>, tui: &TUI) -> FoshResult<EntityRef> {
    if command.kind != ASTKind::Delimited {
        execute_sequenced(command, tui)
    } else {
        let children = command.children().iter()
            .filter(|c| c.kind != ASTKind::SemiColon)
            .map(|c| *c)
            .collect::<Vec<_>>();

        for node in 0..children.len() - 1 {
            let r = execute_sequenced(children[node], tui);
            if let Err(e) = &r {
                report(command.root(), e);
            }
        }

        execute_sequenced(children.last().unwrap(), tui)
    }
}

fn execute_sequenced<'a>(command: &'a PTNode<'a>, tui: &TUI) -> FoshResult<EntityRef> {
    if command.kind != ASTKind::Sequenced {
        execute_piped(command, tui)
    } else {
        let children = command.children().iter()
            .filter(|c| c.kind != ASTKind::SemiColon)
            .map(|c| *c)
            .collect::<Vec<_>>();

        for node in 0..children.len() - 1 {
            let r = execute_piped(children[node], tui);
            if r.is_err() { return r; }
        }

        execute_piped(children.last().unwrap(), tui)
    }
}

fn execute_piped<'a>(command: &'a PTNode<'a>, tui: &TUI) -> FoshResult<EntityRef> {
    if command.kind != ASTKind::Piped {
        execute_command_or_function(command, tui)
    } else {
        Err(EntityExecutionError::new())
    }
}

fn execute_command_or_function<'a>(command: &'a PTNode<'a>, tui: &TUI) -> FoshResult<EntityRef> {
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
                                pt: command.id(),
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

fn execute_function<'a>(command: &'a PTNode<'a>, tui: &TUI) -> FoshResult<EntityRef> {
    let node = command.children()[1];
    execute_value(node, tui)
}

fn execute_value<'a>(node: &'a PTNode<'a>, tui: &TUI) -> FoshResult<EntityRef> {
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

fn execute_property_insn<'a>(command: &'a PTNode<'a>, tui: &TUI) -> FoshResult<EntityRef> {
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

fn validate_types(arg: Argument, value: &EntityRef) -> bool {
    let r = RefCell::borrow(&value);
    for x in arg.possible_types {
        if r.implicits().contains_key(&x) { return true; }
    }

    return false;
}

fn property_call_execution<'a>(command: &'a PTNode<'a>, tui: &TUI) -> Result<Box<dyn Execution>, EntityExecutionError> {

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
            if exe.arguments.len() != args.len() {
                return Err(EntityExecutionError::new_single(parenthesis.id(), ErrorType::Semantic, format!("Expected {} arguments, got {}", exe.arguments.len(), args.len())));
            }
            for i in 0..args.len() {
                if !validate_types(exe.arguments[i].clone(), &args[i]) {
                    return Err(EntityExecutionError::new_single(parenthesis.children()[1+i].id(), ErrorType::Semantic, format!("Argument is not of type {:?}", exe.arguments[i].possible_types[0])));
                }
            }

            match (exe.callee)(left.clone(), &args, ExecutionConfig {
                std_in: None,
                std_out: None,
                std_err: None,
                pt: command.id(),
            }) {
                Ok(e) => {
                    return Ok(e);
                }
                Err(e) => {
                    Err(e)
                }
            }
        }
    }
}

fn execute_property_call<'a>(command: &'a PTNode<'a>, tui: &TUI) -> FoshResult<EntityRef> {
    property_call_execution(command, tui)?.execute()
}
fn execute_braced_command<'a>(command: &'a PTNode<'a>, tui: &TUI) -> FoshResult<EntityRef> {
    let node = command.children()[1];
    execute_delimited(node, tui)
}

fn execute_primitive<'a>(command: &'a PTNode<'a>) -> FoshResult<EntityRef> {
    return match downcast_to_typed(command).unwrap().infer_value(command) {
        None => {
            Err(EntityExecutionError::new_single(command.id(), ErrorType::Semantic,"Could not infer value"))
        }
        Some(e) => Ok(e)
    };
}

