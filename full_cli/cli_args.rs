use clap::{Parser, ValueEnum};
use std::io;

use benchmarker::io::{export_csv, export_html};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub binary: PathBuf,
    #[arg(short, long)]
    pub cli_args: Vec<String>,
    #[arg(short, long, default_value_t = 1000)]
    pub runs: usize,
    #[arg(short, long, default_value_t = false)]
    pub show_output_in_console: bool,
    #[arg(value_enum, short = 't', long, default_value_t = ExportType::CSV)]
    pub export_ty: ExportType,
    #[arg(short = 'f', long)]
    pub export_out_file: String,
    #[arg(short = 'n', long)]
    pub export_trace_name: String,
}

#[derive(Copy, Clone, Debug, ValueEnum, strum::Display)]
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
            ExportType::HTML => export_html(
                Some((trace_name, runs)),
                export_file_name,
                Vec::<String>::new(),
            ),
            ExportType::CSV => export_csv(
                Some((trace_name, runs)),
                export_file_name,
                Vec::<String>::new(),
            ),
        }
    }
}
