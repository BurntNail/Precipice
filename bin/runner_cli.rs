//! Run stuff in a CLI

use crate::ExportType;
use benchmarker::{
    bencher::Builder,
    io::{export_csv, export_html},
};
use clap::{Parser, ValueEnum};
use indicatif::ProgressBar;
use std::{
    ffi::OsStr,
    io::{self, stdin},
    path::PathBuf,
    sync::mpsc::{channel, Receiver},
    time::Duration,
};

/// The CLI args for running stuff
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
    ///Whether or not we should have a warmup run where the results aren't sent to get the program into the cache
    #[arg(short, long, default_value_t = true)]
    warmup: bool,
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

///Run the runner CLI
pub fn run(
    FullCLIArgs {
        binary,
        cli_args,
        runs,
        warmup,
        export_ty,
        export_out_file,
        export_trace_name,
    }: FullCLIArgs,
) {
    let export_out_file = export_out_file.unwrap_or_else(|| {
        let bin_name = binary
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("bench_results");
        format!("{bin_name}_{runs}")
    });
    let export_trace_name = export_trace_name.unwrap_or_else(|| export_out_file.clone());

    let mut found_runs = vec![];

    let (stop_tx, stop_rx) = channel();
    let mut builder = Builder::new()
        .binary(binary)
        .runs(runs)
        .with_warmup(warmup)
        .stop_channel(stop_rx);
    if let Some(cli_args) = cli_args {
        if !cli_args.is_empty() {
            builder = builder.with_cli_args(cli_args.split(' ').map(ToString::to_string).collect());
        }
    }

    let (handle, rx) = builder.start().expect("unable to start builder");

    std::thread::sleep(std::time::Duration::from_millis(100)); //wait to make sure we have the pb under the warmup

    let pb = ProgressBar::new(runs as u64);
    let stdin_nb = spawn_stdin_channel();

    while !handle.is_finished() {
        let mut delta = 0;
        for time in rx.try_iter() {
            found_runs.push(time.as_micros());
            delta += 1;
        }
        pb.inc(delta);

        if stdin_nb.try_iter().next().is_some() {
            println!("Stopping early");
            stop_tx.send(()).expect("error sending stop message");
            break;
        }
    }
    handle
        .join()
        .expect("unable to join handle")
        .expect("error from inside handle");

    let (mean, standard_deviation) = {
        let len = found_runs.len() as f64;
        let mut sum = 0;
        let mut sum_of_squares = 0;

        for item in &found_runs {
            sum += item;
            sum_of_squares += item.pow(2);
        }

        let mean = (sum as f64) / len;
        let variance = mean.mul_add(-mean, (sum_of_squares as f64) / len); //(sum_of_squares as f64) / len - mean.powi(2); mean of squares - square of mean

        (
            Duration::from_secs_f64(mean / 1_000_000.0),
            Duration::from_secs_f64(variance.sqrt() / 1_000_000.0),
        )
    };

    let n = export_ty.export(export_trace_name, found_runs, export_out_file);

    info!(?n, "Finished exporting");
    println!("End Result: {mean:.3?} ± {standard_deviation:.3?}");
}

///Spawns a channel to read from stdin without blocking the main thread
///
///From: <https://stackoverflow.com/questions/30012995/how-can-i-read-non-blocking-from-stdin>
fn spawn_stdin_channel() -> Receiver<String> {
    let (tx, rx) = channel::<String>();
    std::thread::spawn(move || loop {
        let mut buffer = String::new();
        stdin().read_line(&mut buffer).unwrap();
        tx.send(buffer).unwrap();
    });
    rx
}
