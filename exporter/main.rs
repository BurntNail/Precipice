#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::too_many_lines)]

use crate::exporter_app::ExporterApp;
use tracing::{dispatcher::set_global_default, Level};
use tracing_subscriber::FmtSubscriber;

mod exporter_app;

fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish(); //build a console output formatter that only outputs if the level is >= INFO
    set_global_default(subscriber.into()).expect("setting default subscriber failed"); //set the global subscriber to be that subscriber

    eframe::run_native(
        //Run a new native window with default options, and the BencherApp
        "Benchmarker Exporter",
        eframe::NativeOptions::default(),
        Box::new(|cc| Box::new(ExporterApp::new(cc.storage))),
    )
    .expect("Error with eframe");
}
