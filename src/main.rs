#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::too_many_lines)]

use tracing::{Level, dispatcher::set_global_default};
use tracing_subscriber::FmtSubscriber;

use crate::bencher_app::BencherApp;

mod bencher;
mod bencher_app;

#[macro_use] extern crate tracing;

fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    set_global_default(subscriber.into())
        .expect("setting default subscriber failed");

    eframe::run_native(
        "Benchmarker",
        eframe::NativeOptions::default(),
        Box::new(|cc| Box::new(BencherApp::new(cc))),
    )
    .expect("Error with eframe");
}
