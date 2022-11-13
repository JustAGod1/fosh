use std::collections::HashMap;
use std::fmt::{Display, format, Formatter};
use std::io::{Read, Write};
use std::rc::Rc;
use pipe::{PipeReader, PipeWriter};
use crate::builtin::contributors::FilesContributor;
use crate::builtin::engine::{Argument, Type};
use crate::builtin::engine::entities::{Callee, EntitiesManager, Entity, implicit_type};


pub fn initialize_universe(manager: &'static EntitiesManager) {
    manager.global_mut().add_property("cd", make_cd(manager));
}

fn make_cd(manager: &'static EntitiesManager) -> Entity {
    manager.make_entity("Change Directory call".to_string())
        .with_callee(
            Callee::new_pseudo_execution(
                move |args, entities, stdin, stdout, stderr|
                    {
                        write!(stdout, "kek");
                        Ok(entities.make_entity("cd result".to_string()))
                    }
            ).with_arguments(vec![Argument {
                name: "path".to_string(),
                ty: Type::String,
                contributor: &manager.files_contributor,
            }])
        )
}


