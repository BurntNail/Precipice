#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::missing_docs_in_private_items
)]
#![allow(clippy::too_many_lines, clippy::cast_precision_loss)]
//! This is precipice - a binary to benchmark stuff

use crate::{
    exporter_gui::ExporterApp,
    runner_cli::{run, FullCLIArgs},
    runner_gui::BencherApp,
};
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;

mod exporter_gui;
mod runner_cli;
mod runner_gui;

#[macro_use]
extern crate tracing;

#[derive(Clone, Debug, strum::Display, Parser)]
#[command(author, version, about, long_about = None)]
///CLI arguments
pub enum Args {
    ///Collate together different runs in a GUI
    ExporterGUI,
    ///Make runs and quickly export them in a GUI
    RunnerGUI,
    ///Collate together different runs in a CLI
    ExporterCLI,
    ///Make runs and quickly export them in a CLI
    RunnerCLI(FullCLIArgs),
}

fn main() {
    Registry::default()
        .with(EnvFilter::from_default_env())
        .with(
            HierarchicalLayer::new(2)
                .with_targets(true)
                .with_bracketed_fields(true),
        )
        .init();

    match Args::parse() {
        Args::ExporterCLI => todo!("Exporter Cli"),
        Args::RunnerCLI(args) => run(args),
        Args::ExporterGUI => {
            eframe::run_native(
                //Run a new native window with default options, and the BencherApp
                "Benchmarker Exporter",
                eframe::NativeOptions::default(),
                Box::new(|cc| Box::new(ExporterApp::new(cc.storage))),
            )
            .expect("Error with eframe");
        }
        Args::RunnerGUI => {
            eframe::run_native(
                //Run a new native window with default options, and the BencherApp
                "Benchmarker",
                eframe::NativeOptions::default(),
                Box::new(|cc| Box::new(BencherApp::new(cc))),
            )
            .expect("Error with eframe");
        }
    }
}
