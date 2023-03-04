use std::{
    env::current_dir,
    io,
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc::{channel, Receiver},
    thread::JoinHandle,
    time::{Duration, Instant},
};

pub struct Builder {
    binary: Option<PathBuf>,
    cli_args: Vec<String>,
    runs: Option<usize>,
    stop_channel: Option<Receiver<()>>,
    show_output_in_console: bool,
}

const CHUNK_SIZE: usize = 25;

impl Builder {
    pub const fn new() -> Self {
        Self {
            binary: None,
            cli_args: vec![],
            runs: None,
            stop_channel: None,
            show_output_in_console: false,
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

    pub const fn with_show_console_output(mut self, show_output_in_console: bool) -> Self {
        self.show_output_in_console = show_output_in_console;
        self
    }

    ///Panics if elements are not present
    pub fn start(self) -> (JoinHandle<io::Result<()>>, Receiver<Duration>) {
        let runs = self.runs.unwrap();
        let binary = self.binary.unwrap();
        let cli_args = self.cli_args;
        let stop_recv = self.stop_channel.unwrap();
        let show_output_in_console = self.show_output_in_console;

        let (duration_sender, duration_receiver) = channel();
        let handle = std::thread::spawn(move || {
            info!(%runs, ?binary, ?cli_args, ?show_output_in_console, "Starting benching.");

            let mut command = Command::new(binary);
            command.args(cli_args);

            if !show_output_in_console {
                command.stdout(Stdio::null());
            }

            if let Ok(cd) = current_dir() {
                command.current_dir(cd);
            }

            let mut start = Instant::now();
            let no_runs = runs / CHUNK_SIZE;
            // let pb = ProgressBar::new(no_runs as u64);
            for i in 0..no_runs {
                if stop_recv.try_recv().is_err() {
                    let no_runs_inside = if i == no_runs - 1 {
                        runs % CHUNK_SIZE
                    } else {
                        CHUNK_SIZE
                    };

                    for _ in 0..no_runs_inside {
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
