//! Allows to export to CSV

use crate::ExportType;
use benchmarker::io::{export_csv_no_file_input, export_html_no_file_input, get_traces};
use clap::Parser;
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
///CLI Arguments for the Exporter
pub struct ExporterCLIArgs {
    ///List of input CSV files to pull from
    #[arg(long, short)]
    pub input: Vec<PathBuf>,
    ///The file to export to, without extension
    #[arg(long, short, default_value_t = String::from("precipice_bench"))]
    pub output_without_extension: String,
    ///How to export the data - a csv with the microsecond values, or an HTML graph
    #[arg(value_enum, short = 't', long, default_value_t = ExportType::HTML)]
    pub output_ty: ExportType,
}

///Run the CLI exporter
pub fn run(
    ExporterCLIArgs {
        input,
        output_without_extension,
        output_ty,
    }: ExporterCLIArgs,
) {
    let traces = get_traces(input, None).expect("unable to get traces");
    match output_ty {
        ExportType::HTML => export_html_no_file_input(output_without_extension, traces)
            .expect("unable to export files to csv"),
        ExportType::CSV => export_csv_no_file_input(output_without_extension, traces)
            .expect("unable to export files to csv"),
    };
}
