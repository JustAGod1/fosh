use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs::File;
use std::future::Future;
use std::io::{Error, Read, stderr, stdin, stdout, Write};
use std::mem::ManuallyDrop;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::os::unix::prelude::{AsFd, OwnedFd};
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::future::BoxFuture;
use nix::unistd::pipe;
use termion::input::TermReadEventsAndRaw;
use fosh::error_printer::ErrorType;
use crate::builtin::engine::parse_tree::PTNode;
use crate::parser::ast::{ASTKind, downcast_to_typed};
use crate::{construct_error_report, entities, EntitiesManager, report, TUI};
use crate::builtin::engine::entities::{AwaitableFuture, Entity, EntityExecutionError, EntityRef, Execution, ExecutionConfig, FoshEntity, FoshResult};
use crate::builtin::engine::{Argument, Value};

pub fn execute<'a>(command: &'a PTNode<'a>, execution: &ExecutionConfig) -> ExecutionState {
    execute_delimited(command, execution)
}

fn execute_delimited<'a>(command: &'a PTNode<'a>, execution: &ExecutionConfig) -> ExecutionState {
    if command.kind != ASTKind::Delimited {
        execute_sequenced(command, execution)
    } else {
        let children = command.children().iter()
            .filter(|c| c.kind != ASTKind::SemiColon)
            .map(|c| *c)
            .collect::<Vec<_>>();

        for node in 0..children.len() - 1 {
            let r = execute_sequenced(children[node], execution).execute();
            if let Err(e) = &r {
                report(command.root(), e);
            }
        }

        execute_sequenced(children.last().unwrap(), execution)
    }
}

fn execute_sequenced<'a>(command: &'a PTNode<'a>, execution: &ExecutionConfig) -> ExecutionState {
    if command.kind != ASTKind::Sequenced {
        execute_piped(command, execution)
    } else {
        let children = command.children().iter()
            .filter(|c| c.kind != ASTKind::SemiColon)
            .map(|c| *c)
            .collect::<Vec<_>>();

        for node in 0..children.len() - 1 {
            let r = execute_piped(children[node], execution).execute();
            if r.is_err() { return r.into(); }
        }

        execute_piped(children.last().unwrap(), execution)
    }
}

fn execute_piped<'a>(command: &'a PTNode<'a>, execution: &ExecutionConfig) -> ExecutionState {
    if command.kind != ASTKind::Piped {
        execute_command_or_function(command, execution)
    } else {
        let stdout = stdout();
        let stderr = stderr();
        let stdin = stdin();
        let final_err = execution.std_err.as_ref()
            .map(|e| e.try_clone())
            .unwrap_or_else(|| stderr.as_fd().try_clone_to_owned());
        let final_out = execution.std_out.as_ref()
            .map(|e| e.try_clone())
            .unwrap_or_else(|| stderr.as_fd().try_clone_to_owned());
        let first_in = execution.std_in.as_ref()
            .map(|e| e.try_clone())
            .unwrap_or_else(|| stderr.as_fd().try_clone_to_owned());

        if final_err.is_err() {
            return Err(EntityExecutionError::new_single(
                command.id(),
                ErrorType::CannotCloneFd,
                final_err.err().unwrap().to_string()
            )).into();
        }
        let final_err = final_err.unwrap();

        if final_out.is_err() {
            return Err(EntityExecutionError::new_single(
                command.id(),
                ErrorType::CannotCloneFd,
                final_out.err().unwrap().to_string()
            )).into();
        }
        let final_out = final_out.unwrap();

        if first_in.is_err() {
            return Err(EntityExecutionError::new_single(
                command.id(),
                ErrorType::CannotCloneFd,
                first_in.err().unwrap().to_string()
            )).into();
        }
        let first_in = first_in.unwrap();

        let children = command.children().iter()
            .filter(|c| c.kind != ASTKind::Pipe)
            .map(|c| *c)
            .collect::<Vec<_>>();

        let mut last_read = first_in;
        let mut executions = VecDeque::new();
        for i in 0..children.len() {
            let child = children[i];
            let config = if i != children.len() - 1 {
                let pipe_result = pipe();

                if let Err(e) = pipe_result {
                    return Err(EntityExecutionError::new_single(
                        command.id(),
                        ErrorType::CannotCreatePipe,
                        format!("Cannot create pipe: {}", e),
                    )).into();
                }

                let (read, write) = pipe_result.unwrap();

                let read = unsafe { OwnedFd::from_raw_fd(read) };
                let write = unsafe { OwnedFd::from_raw_fd(write) };
                let config = ExecutionConfig::new_with_dup(
                    child.id(),
                    &last_read,
                    &write,
                    &final_err
                );
                last_read = read;
                config
            } else {
                ExecutionConfig::new_with_dup(
                    child.id(),
                    &last_read,
                    &final_out,
                    &final_err
                )
            };

            let config = match config {
                Ok(c) => c,
                Err(e) => return {
                    Err(EntityExecutionError::new_single(
                        command.id(),
                        ErrorType::CannotCreatePipe,
                        format!("Cannot create pipe: {}", e),
                    )).into()
                }
            };

            let r = execute_command_or_function(child, &config);
            std::mem::drop(config);

            executions.push_back(r);
        }

        let mut last = None;
        while !executions.is_empty() {
            last = Some(executions.pop_front().unwrap().execute());
        }

        return last.unwrap().into();
    }
}

fn execute_command_or_function<'a>(command: &'a PTNode<'a>, execution: &ExecutionConfig) -> ExecutionState {
    match command.kind {
        ASTKind::Function => {
            execute_function(command, execution)
        }
        ASTKind::Command => {
            return match downcast_to_typed(command).unwrap().infer_value(command) {
                None => {
                    Err(EntityExecutionError::new_single(command.id(), ErrorType::Semantic, "Could not infer execution value")).into()
                }
                Some(e) => {
                    match RefCell::borrow(&e).callee() {
                        None => {
                            Err(EntityExecutionError::new_single(command.id(), ErrorType::Semantic, "No callee")).into()
                        }
                        Some(exe) => {
                            let config = match execution.try_clone() {
                                Ok(c) => c,
                                Err(e) => {
                                    return Err(EntityExecutionError::new_single(command.id(), ErrorType::CannotCloneFd, format!("Cannot clone execution config: {}", e))).into();
                                }
                            };
                            match (exe.callee)(e.clone(), &[], config) {
                                Ok(mut e) => {
                                    e.into()
                                }
                                Err(e) => {
                                    Err(e).into()
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

fn execute_function<'a>(command: &'a PTNode<'a>, execution: &ExecutionConfig) -> ExecutionState {
    let node = command.children()[1];
    execute_value(node, execution)
}

fn execute_value<'a>(node: &'a PTNode<'a>, execution: &ExecutionConfig) -> ExecutionState {
    match node.kind {
        ASTKind::StringLiteral | ASTKind::NumberLiteral => {
            execute_primitive(node)
        }
        ASTKind::BracedCommand => {
            execute_braced_command(node, execution)
        }
        ASTKind::PropertyInsn => {
            execute_property_insn(node, execution)
        }
        ASTKind::PropertyCall => {
            execute_property_call(node, execution)
        }
        _ => {
            panic!("Unexpected function node {:?}", node.kind)
        }
    }
}

fn execute_property_insn<'a>(command: &'a PTNode<'a>, execution: &ExecutionConfig) -> ExecutionState {
    let (left, name) = if command.children().len() > 1 {
        let v = execute_value(command.children()[0], execution).execute();
        if v.is_err() { return v.into(); }
        (v.unwrap(), command.children()[2])
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
    }.into()
}

fn validate_types(arg: Argument, value: &EntityRef) -> bool {
    let r = RefCell::borrow(&value);
    for x in arg.possible_types {
        if r.implicits().contains_key(&x) { return true; }
    }

    return false;
}

fn property_call_execution<'a>(command: &'a PTNode<'a>, execution: &ExecutionConfig) -> ExecutionState {
    let left = execute_property_insn(command.children()[0], execution).execute();
    if left.is_err() {
        return left.into();
    }
    let left = left.unwrap();
    let parenthesis = command.children()[1];

    let mut args = Vec::new();
    for x in parenthesis.children().iter().skip(1) {
        if x.kind == ASTKind::Parameter {
            let r = execute_value(x.children()[0], execution).execute();

            match r {
                Ok(v) => args.push(v),
                Err(e) => return Err(e).into(),
            }
        }
    }

    let x = RefCell::borrow(&left);
    match x.callee() {
        None => {
            Err(EntityExecutionError::new_single(command.id(), ErrorType::Semantic, format!("Property {} is not callable", left.name()))).into()
        }
        Some(exe) => {
            if exe.arguments.len() != args.len() {
                return Err(EntityExecutionError::new_single(parenthesis.id(), ErrorType::Semantic, format!("Expected {} arguments, got {}", exe.arguments.len(), args.len()))).into();
            }
            for i in 0..args.len() {
                if !validate_types(exe.arguments[i].clone(), &args[i]) {
                    return Err(EntityExecutionError::new_single(parenthesis.children()[1 + i].id(), ErrorType::Semantic, format!("Argument is not of type {:?}", exe.arguments[i].possible_types[0]))).into();
                }
            }

            let config = match execution.try_clone() {
                Ok(c) => c,
                Err(e) => {
                    return Err(EntityExecutionError::new_single(command.id(), ErrorType::CannotCloneFd, format!("Cannot clone execution config: {}", e))).into();
                }
            };
            match (exe.callee)(left.clone(), &args, config) {
                Ok(e) => {
                    return e.into();
                }
                Err(e) => {
                    Err(e).into()
                }
            }
        }
    }
}

fn execute_property_call<'a>(command: &'a PTNode<'a>, execution: &ExecutionConfig) -> ExecutionState {
    property_call_execution(command, execution)
}

fn execute_braced_command<'a>(command: &'a PTNode<'a>, execution: &ExecutionConfig) -> ExecutionState {
    let node = command.children()[1];
    execute_delimited(node, execution)
}

fn execute_primitive<'a>(command: &'a PTNode<'a>) -> ExecutionState {
    return match downcast_to_typed(command).unwrap().infer_value(command) {
        None => {
            Err(EntityExecutionError::new_single(command.id(), ErrorType::Semantic, "Could not infer value")).into()
        }
        Some(e) => Ok(e).into()
    };
}

pub enum ExecutionState {
    Value(FoshResult<EntityRef>),
    Execution(Execution),
}

impl ExecutionState {
    pub(crate) fn execute(self) -> FoshResult<EntityRef> {
        match self {
            ExecutionState::Value(e) => e,
            ExecutionState::Execution(e) => e.execute(),
        }
    }
}

impl Into<ExecutionState> for FoshResult<EntityRef> {
    fn into(self) -> ExecutionState {
        ExecutionState::Value(self)
    }
}

impl Into<ExecutionState> for Execution {
    fn into(self) -> ExecutionState {
        ExecutionState::Execution(self)
    }
}
