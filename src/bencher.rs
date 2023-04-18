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
//! let (handle, rx) = Runner::new_sync(PathBuf::from("/bin/echo"), vec!["Hello".into()], DEFAULT_RUNS, None, true, false);
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
    mem::ManuallyDrop,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::mpsc::{channel, Receiver},
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tokio::{
    io::AsyncWriteExt,
    process::Command as TokioCommand,
    sync::mpsc::{unbounded_channel as tokio_channel, UnboundedReceiver as TokioReceiver},
    task::JoinHandle as TokioJoinHandle,
};

///Union for a receiver
union Receivers {
    ///sync stdlib version
    sync: ManuallyDrop<Receiver<()>>,
    ///async [`tokio`] version
    not_sync: ManuallyDrop<TokioReceiver<()>>,
}

///Struct to build a Bencher - takes in arguments using a builder pattern, then you can start a run.
///
/// No constructor exists - use the struct literal form with [`Default::default`]
pub struct Runner<const IS_ASYNC: bool> {
    ///The binary to run
    binary: PathBuf,
    ///The args to pass to the binary
    cli_args: Vec<String>,
    ///The number of runs
    runs: usize,
    ///The channel to stop running
    stop_rx: Option<Receivers>,
    ///Whether to run any warmup runs to cache the program
    warmup: bool,
    ///Whether or not to print the initial run
    print_initial: bool,
}

///Runs a certain number of runs every time we see no stop signal, to avoid constantly polling the stop receiver
const CHUNK_SIZE: usize = 5;

///Useful constant for default runs
pub const DEFAULT_RUNS: usize = 1_000;

impl Runner<false> {
    ///Constructor
    #[must_use]
    pub fn new_sync(
        binary: PathBuf,
        cli_args: Vec<String>,
        runs: usize,
        stop_rx: Option<Receiver<()>>,
        warmup: bool,
        print_initial: bool,
    ) -> Self {
        Self {
            binary,
            cli_args,
            runs,
            stop_rx: stop_rx.map(|r| Receivers {
                sync: ManuallyDrop::new(r),
            }),
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
        let runs = runs - usize::from(!warmup); // if we warmup, we don't need to minus a run, as we don't send it
        let stop_rx = unsafe { stop_rx.map(|x| x.sync) };

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

                let mut start = Instant::now();

                {
                    //either the first run, or the warmup run. if we print initial, we send the stdout, and we always send the stderr
                    let Output {
                        status,
                        stdout,
                        stderr,
                    } = command.output()?;

                    if !warmup {
                        //but we only send the time if this isn't a warmup
                        let elapsed = start.elapsed();
                        duration_sender.send(elapsed).expect("error sending time");
                    }
                    if !status.success() {
                        //if we don't have an initial success, stop!
                        error!(?status, "Initial Command failed");
                        return Ok(());
                    }

                    if !stdout.is_empty() && print_initial {
                        //if we have a stdout, print it
                        io::stdout().lock().write_all(&stdout)?;
                    }
                    if !stderr.is_empty() {
                        //if we have a stderr, print it
                        io::stderr().lock().write_all(&stderr)?;
                    }
                }

                command.stdout(Stdio::null()).stderr(Stdio::null()); //now set the command to not have a stdout or stderr

                for chunk_size in (0..runs)
                    .chunks(CHUNK_SIZE)
                    .into_iter()
                    .map(Iterator::count)
                //run in chunks to avoid constantly polling the stop_rx
                {
                    if stop_rx
                        .as_ref()
                        .map_or(true, |stop_rx| stop_rx.try_recv().is_err())
                    //If we don't receive anything on the stop channel, or we don't have a stop channel
                    {
                        trace!(%chunk_size, "Starting batch.");

                        for _ in 0..chunk_size {
                            start = Instant::now(); //send the elapsed duration and reset it

                            let status = command.status()?; //run the command

                            duration_sender
                                .send(start.elapsed())
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

                if let Some(mut stop_rx) = stop_rx {
                    unsafe {
                        ManuallyDrop::drop(&mut stop_rx);
                    }
                }

                Ok(())
            })
            .expect("error creating thread");

        (handle, duration_receiver)
    }
}

impl Runner<true> {
    ///Constructor
    #[must_use]
    pub fn new_async(
        binary: PathBuf,
        cli_args: Vec<String>,
        runs: usize,
        stop_rx: Option<TokioReceiver<()>>,
        warmup: bool,
        print_initial: bool,
    ) -> Self {
        Self {
            binary,
            cli_args,
            runs,
            stop_rx: stop_rx.map(|r| Receivers {
                not_sync: ManuallyDrop::new(r),
            }),
            warmup,
            print_initial,
        }
    }

    ///Starts the runner using async tasks
    #[must_use]
    #[instrument(skip(self))]
    pub async fn start(self) -> (TokioJoinHandle<io::Result<()>>, TokioReceiver<Duration>) {
        let Self {
            runs,
            binary,
            cli_args,
            stop_rx,
            warmup,
            print_initial,
        } = self; //destructure self - we can't do this in the method signature as I like using self to call methods, and you can't destructure self
        let runs = runs - usize::from(!warmup); // if we warmup, we don't need to minus a run, as we don't send it
        let mut stop_rx = unsafe { stop_rx.map(|x| x.not_sync) };

        let (duration_sender, duration_receiver) = tokio_channel(); //Here, we create a channel to send over the durations

        let handle = tokio::task::spawn(async move {
            info!(%runs, ?binary, ?cli_args, ?warmup, "Starting benching.");

            let mut command = TokioCommand::new(binary);
            command.args(cli_args); //Create a new Command and add our arguments

            if let Ok(cd) = current_dir() {
                command.current_dir(cd); //If we have a current directory, add that to the Command
            }

            let mut start = Instant::now();

            {
                //either the first run, or the warmup run. if we print initial, we send the stdout, and we always send the stderr
                let Output {
                    status,
                    stdout,
                    stderr,
                } = command.output().await?;

                if !warmup {
                    //but we only send the time if this isn't a warmup
                    let elapsed = start.elapsed();
                    duration_sender.send(elapsed).expect("error sending time");
                }
                if !status.success() {
                    //if we don't have an initial success, stop!
                    error!(?status, "Initial Command failed");
                    return Ok(());
                }

                if !stdout.is_empty() && print_initial {
                    //if we have a stdout, print it

                    tokio::io::stdout().write_all(&stdout).await?;
                }
                if !stderr.is_empty() {
                    //if we have a stderr, print it
                    tokio::io::stderr().write_all(&stderr).await?;
                }
            }

            command.stdout(Stdio::null()).stderr(Stdio::null()); //now set the command to not have a stdout or stderr

            let mut chunks = Vec::with_capacity(runs / CHUNK_SIZE + 1);
            let mut left = runs;
            loop {
                if left > CHUNK_SIZE {
                    left -= CHUNK_SIZE;
                    chunks.push(CHUNK_SIZE);
                } else {
                    chunks.push(left);
                    break;
                }
            }

            for chunk_size in chunks
            //run in chunks to avoid constantly polling the stop_rx
            {
                if stop_rx
                    .as_mut()
                    .map_or(true, |stop_recv| stop_recv.try_recv().is_err())
                //If we don't receive anything on the stop channel, or we don't have a stop channel
                {
                    trace!(%chunk_size, "Starting batch.");

                    for _ in 0..chunk_size {
                        start = Instant::now(); //send the elapsed duration and reset it

                        let status = command.status().await?; //run the command

                        duration_sender
                            .send(start.elapsed())
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

            if let Some(mut stop_rx) = stop_rx {
                unsafe {
                    ManuallyDrop::drop(&mut stop_rx);
                }
            }

            Ok(())
        });

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
