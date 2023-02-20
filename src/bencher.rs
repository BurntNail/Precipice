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
    is_mt: Option<bool>,
}

impl Builder {
    pub const fn new() -> Self {
        Self {
            binary: None,
            cli_args: vec![],
            runs: None,
            stop_channel: None,
            is_mt: None,
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

    pub fn is_mt(mut self, is_mt: bool) -> Self {
        self.is_mt = Some(is_mt);
        self
    }

    ///Panics if elements are not present
    pub fn start(self) -> (JoinHandle<io::Result<()>>, Receiver<Duration>) {
        let runs = self.runs.unwrap();
        let binary = self.binary.unwrap();
        let cli_args = self.cli_args;
        let stop_recv = self.stop_channel.unwrap();
        let mt = self.is_mt.unwrap();

        let chunk_size = if mt {
            num_cpus::get() - 1
        } else {
            (runs / 10).max(25)
        }
        .max(1);

        let (duration_sender, duration_receiver) = channel();
        let handle = std::thread::spawn(move || {
            let mut command = Command::new(binary.clone());
            command.args(cli_args.clone());
            if let Ok(cd) = current_dir() {
                command.current_dir(cd);
            }

            let mut handles: Vec<JoinHandle<io::Result<()>>> = vec![];

            let mut start;
            for _chunk in (0..runs).into_iter().collect_vec().chunks(chunk_size) {
                if stop_recv.try_recv().is_err() {
                    for _ in 0..chunk_size {
                        start = Instant::now();
                        if mt {
                            let new_sender = duration_sender.clone();
                            let binary = binary.clone();
                            let cli_args = cli_args.clone();
                            handles.push(std::thread::spawn(move || {
                                let mut command = Command::new(binary);
                                command.args(cli_args);
                                if let Ok(cd) = current_dir() {
                                    command.current_dir(cd);
                                }

                                let start = Instant::now();
                                let _output = command.status()?;
                                new_sender
                                    .send(start.elapsed())
                                    .expect("Error sending result");

                                Ok(())
                            }));
                        } else {
                            let _output = command.status()?;
                            duration_sender
                                .send(start.elapsed())
                                .expect("Error sending result");
                        }
                    }

                    //TODO: threadpool for handles, maybe add rayon as a dep?
                    println!("Handles has {}", handles.len());

                    while !handles.is_empty() {
                        let mut no_removed = 0;
                        for i in 0..handles.len() {
                            if handles[i - no_removed].is_finished() {
                                handles
                                    .remove(i - no_removed)
                                    .join()
                                    .expect("error joining handle")?;
                                no_removed += 1;
                            }
                        }
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
