//! Crate for benchmarking arbritary binaries using [`egui`]

#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::missing_docs_in_private_items
)]
#![allow(clippy::too_many_lines)]

//TODO: make some functions use color_eyre or custom error types, to remove the expects

pub mod bencher;
pub mod io;

#[macro_use]
extern crate tracing;

///Separator to be used for storing lists in EGUI.
///
///Since commas can reasonably appear, I'm instead using this, which theoretically shouldn't appear very often.
pub const EGUI_STORAGE_SEPARATOR: &str = "---,---";
