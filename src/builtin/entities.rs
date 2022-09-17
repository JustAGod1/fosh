use std::collections::HashMap;
use std::fmt::{Display, format, Formatter};
use std::io::{Read, Write};
use std::rc::Rc;
use pipe::{PipeReader, PipeWriter};
use crate::builtin::contributors::FilesContributor;
use crate::builtin::engine::{Argument, Type};
use crate::builtin::engine::entities::{EntitiesManager, Entity, implicit_type};


pub fn initialize_universe<'a>(manager: &'a EntitiesManager<'a>) {

    manager.global().add_property("cd", make_cd(manager));
}

fn make_cd<'a>(manager: &'a EntitiesManager<'a>) -> Entity<'a> {
    manager.make_entity("Change Directory call".to_string())
        .with_arguments(vec![Argument {
            name: "path".to_string(),
            ty: Type::String,
            contributor: &manager.files_contributor,
        }])
        .with_callee(move |_entity, args| {
            let path = implicit_type(Type::String, args.get(0));
            let path = match path {
                Some(s) => s,
                None => panic!("cd: expected string")
            };
            Ok(
                manager
                    .make_entity(format!("cd {}", path))
                    .with_pseudo_execution(|e, com| {
                        write!(com.std_err, "kek");
                        Ok(())
                    })
            )
        })
}


