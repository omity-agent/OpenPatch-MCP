extern crate alloc;
mod engine;
pub mod service;
mod text;
pub(crate) use engine::{parser, patch, path_expansion, seek_sequence};
pub(crate) use service::{command, operation};
