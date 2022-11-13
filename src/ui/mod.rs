pub mod settings;
mod fosh;
pub mod tui;

use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use crate::{EntitiesAnnotator, EntitiesManager, parser, PathAnnotator};
use crate::builtin::engine::annotator::{AnnotationsSink, Annotator, AnnotatorsManager};
use crate::builtin::engine::parse_tree::{ParseTree, PTNode};
use crate::parser::ast::ASTNode;
use crate::runtime::execution::execute;
use crate::ui::settings::TUISettings;



