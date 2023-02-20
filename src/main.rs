#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::too_many_lines)]

use crate::bencher_app::BencherApp;

mod bencher;
mod bencher_app;

fn main() {
    eframe::run_native(
        "KE Bencher",
        eframe::NativeOptions::default(),
        Box::new(|cc| Box::new(BencherApp::new(cc))),
    )
    .expect("Error with eframe");
}
