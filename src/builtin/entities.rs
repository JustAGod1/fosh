use std::collections::HashMap;
use std::fmt::{Display, format, Formatter};
use std::io::{Read, Write};
use std::rc::Rc;
use pipe::{PipeReader, PipeWriter};
use crate::builtin::contributors::FilesContributor;
use crate::builtin::engine::{Argument, Type};
use crate::builtin::engine::entities::{Callee, EntitiesManager, Entity, FoshEntity, EntityRef, implicit_type};
use crate::entities;


pub fn initialize_universe(manager: &'static EntitiesManager) {
    manager.global().add_property("cd", make_cd(manager));
}

fn make_cd(manager: &'static EntitiesManager) -> EntityRef {
    manager.make_entity("Change Directory call".to_string())
        .with_callee(
            Callee::new_pseudo_execution(
                move |args, stdin, stdout, stderr|
                    {
                        Ok(entities().make_entity("cd success".to_string()))
                    }
            ).with_arguments(vec![Argument {
                name: "path".to_string(),
                ty: Type::String,
                contributor: &manager.files_contributor,
            }])
        )
}


