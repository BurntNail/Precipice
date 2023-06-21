//! Module to contain the actual bencher, which runs on its own separate thread.
//!
//! A [`Runner`] is used to create the [`JoinHandle`] and [`Receiver`] where you will get the timing durations - when the [`JoinHandle`] is finished, you know you can safely drop the [`Receiver`], or you need to manually count.
//!
//! ## Example
//! ```rust
//! use std::path::PathBuf;
//! use std::sync::mpsc::{channel, Receiver, Sender};
//! use benchmarker::bencher::{DEFAULT_RUNS, Runner};
//!
//! let (handle, rx) = Runner::new(PathBuf::from("/bin/echo"), vec!["Hello".into()], DEFAULT_RUNS, None, true);
//!
//! while !handle.is_finished() {
//!     //add stuff from rx
//! }
//! handle.join().unwrap().unwrap();
//! ```

use itertools::Itertools;
use std::{
    env::current_dir,
    io,
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::mpsc::{channel, Receiver, TryRecvError},
    thread::JoinHandle,
    time::{Duration, Instant},
};

///Struct to build a Bencher - takes in arguments using a builder pattern, then you can start a run.
///
/// No constructor exists - use the struct literal form with [`Default::default`]
pub struct Runner {
    ///The binary to run
    pub binary: PathBuf,
    ///The args to pass to the binary
    pub cli_args: Vec<String>,
    ///The number of runs
    pub runs: usize,
    ///The channel to stop running
    pub stop_rx: Option<Receiver<()>>,
    ///Whether to run any warmup runs to cache the program
    pub warmup: u8,
    ///Whether or not to print the initial run
    pub print_initial: bool,
}

///Runs a certain number of runs every time we see no stop signal, to avoid constantly polling the stop receiver
const CHUNK_SIZE: usize = 5;

///Useful constant for default runs
pub const DEFAULT_RUNS: usize = 1_000;

impl Runner {
    ///Constructor
    #[must_use]
    pub fn new(
        binary: PathBuf,
        cli_args: Vec<String>,
        runs: usize,
        stop_rx: Option<Receiver<()>>,
        warmup: u8,
        print_initial: bool,
    ) -> Self {
        Self {
            binary,
            cli_args,
            runs,
            stop_rx,
            warmup,
            print_initial,
        }
    }

    ///Starts the runner in a new thread
    #[must_use]
    #[instrument(skip(self))]
    pub fn start(self) -> (JoinHandle<io::Result<()>>, Receiver<Duration>) {
        let Self {
            runs,
            binary,
            cli_args,
            stop_rx,
            warmup,
            print_initial,
        } = self; //destructure self - we can't do this in the method signature as I like using self to call methods, and you can't destructure self

        let (duration_sender, duration_receiver) = channel(); //Here, we create a channel to send over the durations

        let handle = std::thread::Builder::new()
            .name("benchmark_runner".into()) //new thread to run the benchmarks on
            .spawn(move || {
                info!(%runs, ?binary, ?cli_args, ?warmup, "Starting benching.");

                let mut command = Command::new(binary);
                command.args(cli_args); //Create a new Command and add our arguments

                if let Ok(cd) = current_dir() {
                    command.current_dir(cd); //If we have a current directory, add that to the Command
                }


                let mut is_first = true;
                for _ in 0..warmup {
                    //either the first run, or the warmup run. if we print initial, we send the stdout, and we always send the stderr
                    let Output {
                        status,
                        stdout,
                        stderr,
                    } = command.output()?;

                    if !status.success() {
                        //if we don't have an initial success, stop!
                        error!(?status, "Initial Command failed");
                        return Ok(());
                    }

                    if print_initial && is_first && !stdout.is_empty() {
                        is_first = false;
                        //if we have a stdout, print it
                        io::stdout().lock().write_all(&stdout)?;
                    }
                    if !stderr.is_empty() {
                        //if we have a stderr, print it
                        io::stderr().lock().write_all(&stderr)?;
                    }
                }

                command.stdout(Stdio::null()).stderr(Stdio::null()); //now set the command to not have a stdout or stderr

                let mut start;

                for chunk_size in (0..runs)
                    .chunks(CHUNK_SIZE)
                    .into_iter()
                    .map(Iterator::count)
                //run in chunks to avoid constantly polling the stop_rx
                {
                    if stop_rx
                        .as_ref()
                        .map_or(true, |stop_recv| matches!(stop_recv.try_recv(), Err(TryRecvError::Empty)))
                    //If we don't receive anything on the stop channel, or we don't have a stop channel
                    {
                        trace!(%chunk_size, "Starting batch.");

                        for _ in 0..chunk_size {
                            start = Instant::now(); //send the elapsed duration and reset it
                            let status = command.status()?; //run the command
                            let elapsed = start.elapsed(); //get how long it took

                            duration_sender
                                .send(elapsed)
                                .expect("Error sending result");

                            if status.success() {
                                //log the status
                                trace!(?status, "Finished command");
                            } else {
                                warn!(?status, "Command failed");
                            }
                        }
                    } else {
                        break; //if we did receive something on the stop channel, break the loop
                    }
                }

                Ok(())
            })
            .expect("error creating thread");
        (handle, duration_receiver)
    }
}

///Calculate the mean and standard deviation from a list of microsecond run values
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn calculate_mean_standard_deviation(runs: &[u128]) -> Option<(Duration, Duration)> {
    if runs.is_empty() {
        return None;
    }

    let len = runs.len() as f64;
    let mut sum = 0;
    let mut sum_of_squares = 0;

    for item in runs {
        sum += item;
        sum_of_squares += item.pow(2);
    }

    let mean = (sum as f64) / len;
    let variance = mean.mul_add(-mean, (sum_of_squares as f64) / len); //(sum_of_squares as f64) / len - mean.powi(2); mean of squares - square of mean

    Some((
        Duration::from_secs_f64(mean / 1_000_000.0),
        Duration::from_secs_f64(variance.sqrt() / 1_000_000.0),
    )) //divide by 1_000_000 to account for micros being stored
}
