use std::collections::HashMap;
use std::fmt::{Display, format, Formatter};
use std::io::{Read, Write};
use std::rc::Rc;
use pipe::{PipeReader, PipeWriter};
use fosh::error_printer::ErrorType;
use crate::builtin::contributors::FilesContributor;
use crate::builtin::engine::{Argument, Type, Value};
use crate::builtin::engine::entities::{Callee, EntitiesManager, Entity, FoshEntity, EntityRef, EntityExecutionError};
use crate::entities;


pub fn initialize_universe(manager: &'static EntitiesManager) {
    manager.global().add_property("cd", make_cd(manager));
}

fn make_cd(manager: &'static EntitiesManager) -> EntityRef {
    manager.make_entity("Change Directory call".to_string())
        .with_callee(
            Callee::new_pseudo_execution(
                move |pt, args, stdin, stdout, stderr|
                    {
                        let arg = args.get(0).unwrap().clone();
                        if let Err(e) = std::env::set_current_dir(arg.try_as_string().unwrap()) {
                            return Err(EntityExecutionError::new_single(pt, ErrorType::Execution, format!("Could not change directory: {}", e)));
                        }
                        Ok(entities().make_entity("cd success".to_string()))
                    }
            ).with_arguments(vec![Argument {
                name: "path".to_string(),
                possible_types: vec![Type::String],
                contributor: &manager.files_contributor,
            }]).with_result_prototype(
                move |_, _| {
                    Some(entities().make_entity("cd success".to_string()).with_property("path", Into::<Value>::into("kek".to_string()).into_entity()))
                }
            )
        )
}


