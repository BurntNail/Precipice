
use itertools::Itertools;
use std::{
    env::current_dir,
    io,
    path::PathBuf,
    process::Command,
    sync::mpsc::{channel, Receiver},
    thread::JoinHandle,
    time::{Duration, Instant},
};

pub struct Builder {
    binary: Option<PathBuf>,
    cli_args: Vec<String>,
    runs: Option<usize>,
    stop_channel: Option<Receiver<()>>,
}

const CHUNK_SIZE: usize = 25;

impl Builder {
    pub const fn new() -> Self {
        Self {
            binary: None,
            cli_args: vec![],
            runs: None,
            stop_channel: None,
        }
    }

    #[allow(clippy::missing_const_for_fn)] //pathbuf destructor not at compiletime
    pub fn binary(mut self, string: PathBuf) -> Self {
        self.binary = Some(string);
        self
    }

    pub const fn runs(mut self, runs: usize) -> Self {
        self.runs = Some(runs);
        self
    }

    pub fn stop_channel(mut self, stop_channel: Receiver<()>) -> Self {
        self.stop_channel = Some(stop_channel);
        self
    }

    #[allow(dead_code)]
    pub fn with_cli_arg(mut self, arg: String) -> Self {
        self.cli_args.push(arg);
        self
    }

    pub fn with_cli_args(mut self, mut args: Vec<String>) -> Self {
        self.cli_args.append(&mut args);
        self
    }

    ///Panics if elements are not present
    pub fn start(self) -> (JoinHandle<io::Result<()>>, Receiver<Duration>) {
        let runs = self.runs.unwrap();
        let binary = self.binary.unwrap();
        let cli_args = self.cli_args;
        let stop_recv = self.stop_channel.unwrap();

        let (duration_sender, duration_receiver) = channel();
        let handle = std::thread::spawn(move || {
            let mut command = Command::new(binary);
            command.args(cli_args);
            if let Ok(cd) = current_dir() {
                command.current_dir(cd);
            }

            let mut start = Instant::now();
            for _chunk in (0..runs).into_iter().collect_vec().chunks(CHUNK_SIZE) {
                if stop_recv.try_recv().is_err() {
                    for _ in 0..CHUNK_SIZE {
                        let _output = command.status()?;
                        duration_sender
                            .send(start.elapsed())
                            .expect("Error sending result");
                        start = Instant::now();
                    }
                } else {
                    break;
                }
            }

            Ok(())
        });
        (handle, duration_receiver)
    }
}