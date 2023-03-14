//! Crate for benchmarking arbritary binaries using [`egui`]

#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::missing_docs_in_private_items
)]
#![allow(clippy::too_many_lines)]

//TODO: loads of docs

pub mod bencher;
pub mod egui_utils;
pub mod io;

#[macro_use]
extern crate tracing;

///Separator to be used for storing lists in EGUI
pub const EGUI_STORAGE_SEPARATOR: &str = "---,---";
