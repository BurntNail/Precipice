//! Module to contain the actual bencher, which runs on its own separate thread.
//!
//! A [`Builder`] is used to create the [`JoinHandle`] and [`Receiver`] where you will get the timing durations - when the [`JoinHandle`] is finished, you know you can safely drop the [`Receiver`], or you need to manually count.
//!
//! ## Example
//! ```rust
//! use std::path::PathBuf;
//! use std::sync::mpsc::{channel, Receiver, Sender};
//! use benchmarker::bencher::Builder;
//!
//! let (handle, rx) = Builder::new()
//!     .binary(PathBuf::from("/bin/echo"))
//!     .with_cli_arg("hello".into())
//!     .runs(1_000)
//!     .start().unwrap();
//!
//! //then, poll the receiver for new durations
//! //and end the handle when you feel like it
//! ```
//!
//! ## Usage of Builder
//! The builder contains the following fields:
//! - a [`PathBuf`] to store the binary to run
//! - a [`Vec`] of [`String`]s to store the program's arguments
//! - an [`usize`] to store how many runs to do
//! - a [`bool`] to decide whether or not to show the binary's output in this program's console
//! - a [`Receiver`] to check for signals to stop the program
//!
//! You **must** pass in *at the very minimum*:
//! - the [`PathBuf`] using [`Self::binary`]
//! - the [`usize`] using [`Self::runs`]
//!
//! All of the others are optional

use itertools::Itertools;
use std::{
    env::current_dir,
    io,
    io::Write,
    path::PathBuf,
    process::{Command, Output},
    sync::mpsc::{channel, Receiver},
    thread::JoinHandle,
    time::{Duration, Instant},
};

///Struct to build a Bencher - takes in arguments using a builder pattern, then you can start a run.
pub struct Builder {
    ///The binary to run
    binary: Option<PathBuf>,
    ///The args to pass to the binary
    cli_args: Vec<String>,
    ///The number of runs
    runs: Option<usize>,
    ///The channel to stop running
    stop_channel: Option<Receiver<()>>,
    ///Whether or not to inherit stdout
    show_output_in_console: bool,
}

///Runs a certain number of runs every time we see no stop signal, to avoid constantly polling the stop receiver
const CHUNK_SIZE: usize = 25;

impl Builder {
    ///Creates a default builder:
    ///
    /// - `binary` is `None`
    /// - `cli_args` is empty
    /// - `runs` is `None`
    /// - `stop_channel` is None,
    #[must_use]
    pub const fn new() -> Self {
        Self {
            binary: None,
            cli_args: vec![],
            runs: None,
            stop_channel: None,
            show_output_in_console: false,
        }
    }

    ///This sets the binary to run, overwriting anything already there
    #[allow(clippy::missing_const_for_fn)] //pathbuf destructor not at compiletime
    #[must_use]
    pub fn binary(mut self, string: PathBuf) -> Self {
        self.binary = Some(string);
        self
    }

    ///This sets the number of times to run the program, overwriting anything already there
    #[must_use]
    pub const fn runs(mut self, runs: usize) -> Self {
        self.runs = Some(runs);
        self
    }

    ///This adds a stop channel, overwriting anything already there
    #[must_use]
    pub fn stop_channel(mut self, stop_channel: Receiver<()>) -> Self {
        self.stop_channel = Some(stop_channel);
        self
    }

    ///This adds an argument to the list, adding to the existing ones
    #[must_use]
    pub fn with_cli_arg(mut self, arg: String) -> Self {
        self.cli_args.push(arg);
        self
    }

    ///This adds a number of new arguments to the list, adding to the existing ones
    #[must_use]
    pub fn with_cli_args(mut self, mut args: Vec<String>) -> Self {
        self.cli_args.append(&mut args);
        self
    }

    ///This sets whether or not we redirect console output, overwriting anything already there
    #[must_use]
    pub const fn with_show_console_output(mut self, show_output_in_console: bool) -> Self {
        self.show_output_in_console = show_output_in_console;
        self
    }

    ///Panics if elements are not present
    #[must_use]
    pub fn start(self) -> Option<(JoinHandle<io::Result<()>>, Receiver<Duration>)> {
        //Here we make bindings to lots of the internal variables, returning None if we can't see the ones we need
        let runs = self.runs?;
        let binary = self.binary?;
        let cli_args = self.cli_args;
        let stop_recv = self.stop_channel;
        let show_output_in_console = self.show_output_in_console;

        let (duration_sender, duration_receiver) = channel(); //Here, we create a channel to send over the durations
        let handle = std::thread::spawn(move || {
            info!(%runs, ?binary, ?cli_args, ?show_output_in_console, "Starting benching.");

            let mut command = Command::new(binary);
            command.args(cli_args); //Create a new Command and add our arguments

            if let Ok(cd) = current_dir() {
                command.current_dir(cd); //If we have a current directory, add that to the Command
            }

            let mut start;
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

                        let (status, stdout, stderr) = {
                            let Output {
                                status,
                                stdout,
                                stderr,
                            } = command.output()?;

                            if show_output_in_console {
                                (status, stdout, stderr)
                            } else {
                                (status, vec![], stderr) //always preserve stderr
                            }
                        };

                        duration_sender
                            .send(start.elapsed())
                            .expect("Error sending result");

                        if status.success() {
                            trace!(?status, "Finished command");
                        } else {
                            warn!(?status, "Command failed");
                        }

                        if !stdout.is_empty() {
                            io::stdout().lock().write_all(&stdout)?;
                        }
                        if !stderr.is_empty() {
                            io::stderr().lock().write_all(&stderr)?;
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
