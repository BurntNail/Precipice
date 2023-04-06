use clap::{Parser, ValueEnum};
use std::io;

use benchmarker::io::{export_csv, export_html};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    ///The actual binary to run
    #[arg(short, long)]
    pub binary: PathBuf,
    ///The CLI Arguments, all in one String
    #[arg(short, long)]
    pub cli_args: Option<String>,
    ///The number of runs (excluding warm-up runs)
    #[arg(short, long, default_value_t = 10_000)]
    pub runs: usize,
    ///Whether or not console output from the binary should be shown in the CLI
    #[arg(short, long, default_value_t = false)]
    pub show_output_in_console: bool,
    ///How to export the data - a csv with the microsecond values, or an HTML graph
    #[arg(value_enum, short = 't', long, default_value_t = ExportType::CSV)]
    pub export_ty: ExportType,
    ///The file to export to, without extension. This defaults to the binary's name
    #[arg(short = 'f', long)]
    pub export_out_file: Option<String>,
    ///The trace name to export as. This is the name of the line in the HTML graph and defaults to the binary's name
    #[arg(short = 'n', long)]
    pub export_trace_name: Option<String>,
}

#[derive(Copy, Clone, Debug, ValueEnum, strum::Display)]
#[allow(clippy::upper_case_acronyms)]
pub enum ExportType {
    HTML,
    CSV,
}

impl ExportType {
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
                Vec::<String>::new(),
            ),
            Self::CSV => export_csv(
                Some((trace_name, runs)),
                export_file_name,
                Vec::<String>::new(),
            ),
        }
    }
}
