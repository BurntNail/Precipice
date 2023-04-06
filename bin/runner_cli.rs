use clap::{Parser, ValueEnum};
use std::{ffi::OsStr, io};

use benchmarker::{
    bencher::Builder,
    io::{export_csv, export_html},
};
use indicatif::ProgressBar;
use std::{path::PathBuf, time::Duration};

#[derive(Clone, Debug, Parser)]
pub struct FullCLIArgs {
    ///The actual binary to run
    #[arg(short, long)]
    binary: PathBuf,
    ///The CLI arguments to pass to the binary
    #[arg(short, long)]
    cli_args: Option<String>,
    ///The number of runs (excluding warm-up runs)
    #[arg(short, long, default_value_t = 10_000)]
    runs: usize,
    ///Whether or not console output from the binary should be shown in the CLI
    #[arg(short, long, default_value_t = false)]
    show_output_in_console: bool,
    ///How to export the data - a csv with the microsecond values, or an HTML graph
    #[arg(value_enum, short = 't', long, default_value_t = ExportType::CSV)]
    export_ty: ExportType,
    ///The file to export to, without extension. This defaults to the binary's name
    #[arg(short = 'f', long)]
    export_out_file: Option<String>,
    ///The trace name to export as. This is the name of the line in the HTML graph and defaults to the binary's name
    #[arg(short = 'n', long)]
    export_trace_name: Option<String>,
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

pub fn run(
    FullCLIArgs {
        binary,
        cli_args,
        runs,
        show_output_in_console,
        export_ty,
        export_out_file,
        export_trace_name,
    }: FullCLIArgs,
) {
    let export_out_file = export_out_file.unwrap_or_else(|| {
        binary
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("bench_results")
            .into()
    });
    let export_trace_name = export_trace_name.unwrap_or_else(|| export_out_file.clone());

    let mut found_runs = vec![];

    let mut builder = Builder::new()
        .binary(binary)
        .runs(runs)
        .with_show_console_output(show_output_in_console);
    if let Some(cli_args) = cli_args {
        if !cli_args.is_empty() {
            builder = builder.with_cli_args(cli_args.split(' ').map(ToString::to_string).collect());
        }
    }

    let (handle, rx) = builder.start().expect("unable to start builder");

    let pb = ProgressBar::new(runs as u64);

    while !handle.is_finished() {
        let mut delta = 0;
        for time in rx.try_iter() {
            found_runs.push(time.as_micros());
            delta += 1;
        }
        pb.inc(delta);
    }
    handle
        .join()
        .expect("unable to join handle")
        .expect("error in handle");

    let (mean, standard_deviation) = {
        let len = found_runs.len() as f64;
        let mut sum = 0;
        let mut sum_of_squares = 0;

        for item in &found_runs {
            sum += item;
            sum_of_squares += item.pow(2);
        }

        let mean = (sum as f64) / len;
        let variance = mean.mul_add(-mean, (sum_of_squares as f64) / len); //(sum_of_squares as f64) / len - mean.powi(2);

        (
            Duration::from_secs_f64(mean / 1_000_000.0),
            Duration::from_secs_f64(variance.sqrt() / 1_000_000.0),
        )
    };

    let n = export_ty.export(export_trace_name, found_runs, export_out_file);

    info!(?n, "Finished exporting");
    info!(?mean, ?standard_deviation);
}
