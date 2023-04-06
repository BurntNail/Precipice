#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::cast_precision_loss)]

mod cli_args;

use benchmarker::bencher::Builder;
use clap::Parser;
use cli_args::Args;
use indicatif::ProgressBar;
use std::{ffi::OsStr, time::Duration};
use tracing::{dispatcher::set_global_default, info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish(); //build a console output formatter that only outputs if the level is >= INFO
    set_global_default(subscriber.into()).expect("setting default subscriber failed"); //set the global subscriber to be that subscriber

    let Args {
        binary,
        cli_args,
        runs,
        show_output_in_console,
        export_ty,
        export_out_file,
        export_trace_name,
    } = Args::parse();

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
