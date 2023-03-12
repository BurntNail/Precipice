//! Crate for benchmarking arbritary binaries using [`egui`]

#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::missing_docs_in_private_items
)]
#![allow(clippy::too_many_lines)]

pub mod bencher;
pub mod io;
pub mod list;

#[macro_use]
extern crate tracing;
