//! Module to contain the actual bencher, which runs on its own separate thread.
//!
//! A [`Runner`] is used to create the [`JoinHandle`] and [`Receiver`] where you will get the timing durations - when the [`JoinHandle`] is finished, you know you can safely drop the [`Receiver`], or you need to manually count.
//!
//! ## Example
//! ```rust
//! use std::path::PathBuf;
//! use std::sync::mpsc::{channel, Receiver, Sender};
//! use benchmarker::bencher::Runner;
//!
//! let (handle, rx) = Runner::new(PathBuf::from("/bin/echo"), vec!["Hello".into()], 1_000, None, true);
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
    sync::mpsc::{channel, Receiver},
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
    pub warmup: bool,
}

///Runs a certain number of runs every time we see no stop signal, to avoid constantly polling the stop receiver
const CHUNK_SIZE: usize = 25;

impl Runner {
    ///Constructor
    #[must_use]
    pub fn new(
        binary: PathBuf,
        cli_args: Vec<String>,
        runs: usize,
        stop_rx: Option<Receiver<()>>,
        warmup: bool,
    ) -> Self {
        Self {
            binary,
            cli_args,
            runs,
            stop_rx,
            warmup,
        }
    }

    ///Starts the runner in a new thread
    #[must_use]
    pub fn start(self) -> Option<(JoinHandle<io::Result<()>>, Receiver<Duration>)> {
        let Self {
            runs,
            binary,
            cli_args,
            stop_rx: stop_recv,
            warmup,
        } = self;
        let runs = runs - usize::from(!warmup); // if we warmup, we don't need to minus a run, as we don't send it

        let (duration_sender, duration_receiver) = channel(); //Here, we create a channel to send over the durations
        let handle = std::thread::spawn(move || {
            info!(%runs, ?binary, ?cli_args, ?warmup, "Starting benching.");

            let mut command = Command::new(binary);
            command.args(cli_args); //Create a new Command and add our arguments

            if let Ok(cd) = current_dir() {
                command.current_dir(cd); //If we have a current directory, add that to the Command
            }

            let mut start = Instant::now();

            {
                //either the first run, or the warmup run. we always send output from this one.
                let Output {
                    status,
                    stdout,
                    stderr,
                } = command.output()?;

                if !warmup {
                    duration_sender
                        .send(start.elapsed())
                        .expect("error sending time");
                }
                if !status.success() {
                    error!(?status, "Initial Command failed");
                }

                if !stdout.is_empty() {
                    io::stdout().lock().write_all(&stdout)?;
                }
                if !stderr.is_empty() {
                    io::stderr().lock().write_all(&stderr)?;
                }
            }

            command.stdout(Stdio::null()).stderr(Stdio::null());

            for chunk_size in (0..runs)
                .chunks(CHUNK_SIZE)
                .into_iter()
                .map(Iterator::count)
            {
                if stop_recv
                    .as_ref()
                    .map_or(true, |stop_recv| stop_recv.try_recv().is_err())
                //If we don't receive anything on the stop channel, or we don't have a stop channel
                {
                    trace!(%chunk_size, "Starting batch.");

                    for _ in 0..chunk_size {
                        start = Instant::now(); //send the elapsed duration and reset it

                        let status = command.status()?;

                        duration_sender
                            .send(start.elapsed())
                            .expect("Error sending result");

                        if status.success() {
                            trace!(?status, "Finished command");
                        } else {
                            warn!(?status, "Command failed");
                        }
                    }
                } else {
                    break; //if we do need to stop, break the loop
                }
            }

            Ok(())
        });
        Some((handle, duration_receiver))
    }
}
