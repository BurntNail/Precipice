//! Run stuff in a CLI

use crate::ExportType;
use benchmarker::bencher::{calculate_mean_standard_deviation, Runner, DEFAULT_RUNS};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::{ffi::OsStr, path::PathBuf, time::Duration};
use tokio::sync::mpsc::unbounded_channel;

/// The CLI args for running stuff
#[derive(Clone, Debug, Parser)] //struct for CLI args which can be parsed/cloned/printed
pub struct FullCLIArgs {
    ///The actual binary to run
    #[arg(short, long)]
    binary: PathBuf,
    ///The CLI arguments to pass to the binary
    #[arg(short, long)]
    cli_args: Option<String>,
    ///The number of runs (excluding warm-up runs)
    #[arg(short, long, default_value_t = DEFAULT_RUNS)]
    runs: usize,
    ///Whether or not we should have a warmup run where the results aren't sent to get the program into the cache
    #[arg(short = 'w', long, default_value_t = false)]
    no_warmup: bool,
    ///How to export the data - a csv with the microsecond values, or an HTML graph
    #[arg(value_enum, short = 't', long, default_value_t = ExportType::CSV)]
    export_ty: ExportType,
    ///The file to export to, without extension. This defaults to the binary's name
    #[arg(short = 'f', long)]
    export_out_file: Option<String>,
    ///The trace name to export as. This is the name of the line in the HTML graph and defaults to the binary's name
    #[arg(short = 'n', long)]
    export_trace_name: Option<String>,
    ///Whether or not we should print the inital run.
    #[arg(short, long, default_value_t = false)]
    print_initial: bool,
}

///Run the runner CLI
#[instrument]
pub async fn run(
    FullCLIArgs {
        //destructure the struct right here to avoid having to do it in the function
        binary,
        cli_args,
        runs,
        no_warmup,
        export_ty,
        export_out_file,
        export_trace_name,
        print_initial,
    }: FullCLIArgs,
) {
    let export_out_file = export_out_file.unwrap_or_else(|| {
        //shadow the export_out_file, and if we don't have it
        if export_trace_name.is_some() {
            //if we have the trace name, use that
            export_trace_name.clone().unwrap()
        } else {
            let bin_name = binary
                .file_name()
                .and_then(OsStr::to_str) //if not, try to get the binary name
                .unwrap_or("bench_results"); //falling back to bench_results
            format!("{bin_name}_{runs}") //and add the number of results
        }
    });
    let export_trace_name = export_trace_name.unwrap_or_else(|| export_out_file.clone()); //shadow the export_trace_name, if we don't have it use the same name as the file
    let cli_args = match cli_args {
        Some(cli_args) if !cli_args.is_empty() => {
            //if the CLI args aren't empty, split them by spaces
            cli_args.split(' ').map(ToString::to_string).collect()
        }
        _ => vec![], //if it is empty or we didn't get anything, then return an empty vec. this way we avoid a vec![""]
    };

    let Some(file_name) = binary.file_name().map(OsStr::to_os_string) else {
        panic!("need a binary to bench, not a folder");
    };

    {
        //scoped variables to print a message to the user to let them know what they are doing.
        let binary = match file_name.into_string() {
            Ok(s) => s,
            Err(s) => format!("{s:?}"),
        };
        let binary_and_args = if cli_args.is_empty() {
            binary
        } else {
            binary + &cli_args.join(" ")
        };

        println!("{} {}", "Benchmark:".bold(), binary_and_args.italic());
    }

    let (stop_tx, stop_rx) = unbounded_channel(); //make a channel for stopping

    let mut found_runs = vec![]; //make a vec for runs we've received
    let (handle, mut rx) = Runner::new(
        binary,
        cli_args,
        runs,
        Some(stop_rx),
        !no_warmup,
        print_initial,
    )
    .start_async()
    .await; //get a handle from a new runner, with the binary etc

    std::thread::sleep(Duration::from_millis(50)); //wait to make sure that we show the progress bar underneath the initial run

    let progress_bar = ProgressBar::new(runs as u64); //make a new progress bar with the number of runs we expect to do
    {
        let progress_bar = progress_bar.clone();
        ctrlc::set_handler(move || {
            stop_tx.send(()).expect("Could not send signal on channel.");
            progress_bar.abandon_with_message("Stopped by User");
        })
        .expect("Error setting Ctrl-C handler"); //if we receive a stop signal, stop the benching
    }

    progress_bar.set_style(
        ProgressStyle::with_template(
            "{spinner} Elapsed: [{elapsed_precise}], ETA: [{eta_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7}",
        )
        .unwrap()
        .progress_chars("##-"),
    );

    while !handle.is_finished() {
        if let Some(time) = rx.recv().await {
            found_runs.push(time.as_micros()); //for every run we've got since the last poll, add it to our list
            progress_bar.inc(1);
        }
    }
    handle
        .await
        .expect("error in tokio")
        .expect("error from thread");

    progress_bar.finish_and_clear();
    println!();

    let min_max_median: Option<(u128, u128, u128)> = found_runs
        .iter()
        .min()
        .copied()
        .zip(found_runs.iter().max().copied())
        .zip(found_runs.get(found_runs.len() / 2).copied())
        .map(|((a, b), c)| (a, b, c));
    let mean_standard_deviation = calculate_mean_standard_deviation(&found_runs);
    let no_runs = found_runs.len();

    let n = export_ty.export(export_trace_name, found_runs, export_out_file); //export

    trace!(?n, "Finished exporting");
    if let Some((mean, standard_deviation)) = mean_standard_deviation {
        println!(
            "{}: {} ± {} : {}",
            "Mean ± Standard Deviation : Runs".bold(),
            format!("{mean:.3?}").bright_green(),
            format!("{standard_deviation:.3?}").bright_green(),
            no_runs.bright_white(),
        );
    }
    if let Some((min, max, median)) = min_max_median {
        println!(
            "{}: {} … {} … {}",
            "Min … Median … Max              ".bold(),
            format!("{:.3?}", Duration::from_micros(min as u64)).bright_blue(),
            format!("{:.3?}", Duration::from_micros(median as u64)).bright_green(),
            format!("{:.3?}", Duration::from_micros(max as u64)).bright_red()
        );
    }
}
