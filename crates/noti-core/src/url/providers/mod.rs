//! Per-family URL parser modules.
//!
//! Each submodule groups URL parsers by provider family, making it easy to
//! find and extend the parsing logic for a given provider without scrolling
//! through a 2000-line match arm.

pub mod chat;
pub mod email;
pub mod misc;
pub mod push;
pub mod sms;
