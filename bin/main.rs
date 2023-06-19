#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::missing_docs_in_private_items
)]
#![allow(clippy::too_many_lines, clippy::cast_precision_loss)]
//! This is precipice - a binary to benchmark stuff

//imports
use crate::{
    exporter_cli::ExporterCLIArgs, exporter_gui::ExporterApp, runner_cli::FullCLIArgs,
    runner_gui::BencherApp,
};
use benchmarker::io::{export_csv, export_html};
use clap::{Parser, ValueEnum};
use std::io;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;

mod exporter_cli;
mod exporter_gui;
mod runner_cli;
mod runner_gui;
mod egui_utils;

//allow me to use tracing macros (eg. info! etc) without needing to import all of them.
#[macro_use]
extern crate tracing;

#[derive(Clone, Debug, strum::Display, Parser)] //allow me to print/clone the enum, as well as to parse it as CLI args
#[command(author, version, about, long_about = None)] //use the author/version/about from the Cargo.toml file
///CLI arguments
pub enum Args {
    ///Collate together different runs in a GUI
    ExporterGUI,
    ///Make runs and quickly export them in a GUI
    RunnerGUI,
    ///Collate together different runs in a CLI
    ExporterCLI(ExporterCLIArgs),
    ///Make runs and quickly export them in a CLI
    RunnerCLI(FullCLIArgs),
}

fn main() {
    //setup tracing and tracing-tree via tracing-subscriber from the environment variables
    Registry::default()
        .with(EnvFilter::from_default_env())
        .with(
            HierarchicalLayer::new(2)
                .with_targets(true)
                .with_bracketed_fields(true)
                .with_thread_names(true),
        )
        .init();

    match Args::parse() {
        //switch statement on the arguments, parsed from the CLI, which is an enum, so we switch on that enum
        Args::ExporterCLI(args) => exporter_cli::run(args),
        Args::RunnerCLI(args) => runner_cli::run(args),
        Args::ExporterGUI => {
            eframe::run_native(
                //Run a new native window with default options, and the ExporterApp
                "Precipice Exporter",
                eframe::NativeOptions::default(),
                Box::new(|cc| Box::new(ExporterApp::new(cc.storage))),
            )
            .expect("Error with eframe");
        }
        Args::RunnerGUI => {
            eframe::run_native(
                //Run a new native window with default options, and the BencherApp
                "Precipice Runner",
                eframe::NativeOptions::default(),
                Box::new(|cc| Box::new(BencherApp::new(cc))),
            )
            .expect("Error with eframe");
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum, strum::Display)]
#[allow(clippy::upper_case_acronyms)]
///Any format
pub enum ExportType {
    ///HTML graph
    HTML,
    ///CSV file with everything
    CSV,
}

impl ExportType {
    ///Export to the relevant format
    ///
    /// # Errors
    /// If we can't write to or create the file
    #[instrument]
    pub fn export(
        self,
        trace_name: String,
        runs: Vec<u128>,
        export_file_name: String,
    ) -> io::Result<usize> {
        match self {
            Self::HTML => export_html(
                Some((trace_name, runs)),
                export_file_name,
                Vec::<String>::new(), //since we don't have any extra traces for here, we just give it an empty list. If we don't give it a type using the turbofish, then we get compiler errors on interpreting generics.
            ),
            Self::CSV => export_csv(
                Some((trace_name, runs)),
                export_file_name,
                Vec::<String>::new(),
            ),
        }
    }
}
