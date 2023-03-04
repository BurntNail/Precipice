//! Crate for benchmarking arbritary binaries using [`egui`]

#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::missing_docs_in_private_items
)]
#![allow(clippy::too_many_lines)]

use tracing::{dispatcher::set_global_default, Level};
use tracing_subscriber::FmtSubscriber;

use crate::bencher_app::BencherApp;

mod bencher;
mod bencher_app;

#[macro_use]
extern crate tracing;

fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish(); //build a console output formatter that only outputs if the level is >= INFO
    set_global_default(subscriber.into()).expect("setting default subscriber failed"); //set the global subscriber to be that subscriber

    eframe::run_native(
        //Run a new native window with default options, and the BencherApp
        "Benchmarker",
        eframe::NativeOptions::default(),
        Box::new(|cc| Box::new(BencherApp::new(cc))),
    )
    .expect("Error with eframe");
}
