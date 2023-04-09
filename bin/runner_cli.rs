//! Run stuff in a CLI

use crate::ExportType;
use benchmarker::bencher::{calculate_mean_standard_deviation, Runner, DEFAULT_RUNS};
use clap::Parser;
use indicatif::ProgressBar;
use std::{
    ffi::OsStr,
    io::stdin,
    path::PathBuf,
    sync::mpsc::{channel, Receiver},
    time::Duration,
};

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
}

///Run the runner CLI
#[instrument]
pub fn run(
    FullCLIArgs {
        //destructure the struct right here to avoid having to do it in the function
        binary,
        cli_args,
        runs,
        no_warmup,
        export_ty,
        export_out_file,
        export_trace_name,
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

    println!("Benching {binary:?} with {cli_args:?}, for {runs} to {export_ty}."); //print a message to the user to confirm what they're doing.
    println!("To cancel at any point, press any key."); //notify the user about cancelling by input

    let (stop_tx, stop_rx) = channel(); //make a channel for stopping
    let mut found_runs = vec![]; //make a vec for runs we've received
    let (handle, rx) = Runner::new(binary, cli_args, runs, Some(stop_rx), !no_warmup).start(); //get a handle from a new runner, with the binary etc

    std::thread::sleep(Duration::from_millis(50)); //wait to make sure that we show the progress bar underneath the initial run

    let progress_bar = ProgressBar::new(runs as u64); //make a new progress bar with the number of runs we expect to do
    let stdin_nb = spawn_stdin_channel(); //make a new channel polling stdin for input

    while !handle.is_finished() {
        //while the handle isn't finished, that is whilst we've still got runs
        let mut delta = 0;
        for time in rx.try_iter() {
            //use try_iter to avoid blocking so we keep on going and updating the progress bar
            found_runs.push(time.as_micros()); //for every run we've got since the last poll, add it to our list
            delta += 1; //and increment our delta
        }
        progress_bar.inc(delta); //update our progress bar with the delta

        if stdin_nb.try_iter().next().is_some() {
            //if we've received something from stdin
            eprintln!("Stopping early");
            stop_tx.send(()).expect("error sending stop message"); //send a stop message
                                                                   // we don't stop the loop to ensure that all the runs get received and it stops
        }
    }
    handle
        .join() //join the handle
        .expect("unable to join handle")
        .expect("error from inside handle");

    let (mean, standard_deviation) = calculate_mean_standard_deviation(&found_runs);

    let n = export_ty.export(export_trace_name, found_runs, export_out_file); //export

    trace!(?n, "Finished exporting");
    println!("End Result: {mean:.3?} ± {standard_deviation:.3?}");
}

///Spawns a channel to read from stdin without blocking the main thread
///
///From: <https://stackoverflow.com/questions/30012995/how-can-i-read-non-blocking-from-stdin>
fn spawn_stdin_channel() -> Receiver<String> {
    let (tx, rx) = channel::<String>();
    std::thread::Builder::new()
        .name("stdin_reader".into())
        .spawn(move || loop {
            let mut buffer = String::new();
            stdin().read_line(&mut buffer).unwrap();
            trace!(?buffer, "Read input from stdin");
            tx.send(buffer).unwrap();
        })
        .expect("error creating thread");
    rx
}
