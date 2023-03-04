use crate::bencher::Builder;
use eframe::{App, CreationContext, Frame, Storage};
use egui::{CentralPanel, Context, ProgressBar, ScrollArea, Ui, Widget};
use egui_file::FileDialog;
use std::{
    io::{self, Write},
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
    thread::JoinHandle,
    time::Duration, fs::File,
};

pub struct BencherApp {
    runs: usize,
    state: State,
}

#[allow(clippy::large_enum_variant)]
pub enum State {
    PreContents {
        binary: Option<PathBuf>,
        binary_dialog: Option<FileDialog>, //don't care if it is big - I'll only ever have one `State`
        cli_args: Vec<String>,
        current_cli_arg: String,
        runs_input: String,
    },
    Running {
        run_times: Vec<Duration>,
        stop: Sender<()>,
        run_recv: Receiver<Duration>,
        handle: JoinHandle<io::Result<()>>,
    },
    PostContents {
        run_times: Vec<Duration>,
        min: Duration,
        max: Duration,
        avg: Duration,
        export_handle: Option<JoinHandle<io::Result<usize>>>,
    },
}

impl State {
    fn new(binary: Option<PathBuf>, cli_args: Vec<String>, runs_input: Option<String>) -> Self {
        Self::PreContents {
            binary,
            current_cli_arg: String::default(),
            cli_args,
            binary_dialog: None,
            runs_input: runs_input.unwrap_or_default(),
        }
    }
}

impl BencherApp {
    pub fn new(cc: &CreationContext) -> Self {
        let binary = cc
            .storage
            .and_then(|s| s.get_string("binary_path"))
            .map(PathBuf::from);
        let mut cli_args = cc
            .storage
            .and_then(|s| s.get_string("cli_args"))
            .map(|s| s.split("---,---").map(ToString::to_string).collect())
            .unwrap_or_default();

        if cli_args.len() == 1 && cli_args[0] == String::default() {
            cli_args = vec![];
        }

        let runs_input = cc.storage.and_then(|s| s.get_string("runs"));

        Self {
            runs: 0,
            state: State::new(binary, cli_args, runs_input),
        }
    }
}

impl App for BencherApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        fn show_runs(runs: &[Duration], ui: &mut Ui) {
            ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for (i, run) in runs.iter().enumerate() {
                    ui.label(format!("Run {} took {run:?}.", i + 1));
                }
            });
        }

        let mut change = None;
        match &mut self.state {
            State::PreContents {
                binary,
                binary_dialog,
                runs_input,
                current_cli_arg,
                cli_args,
            } => {
                CentralPanel::default().show(ctx, |ui| {
                    ui.label("Preparing to Bench");
                    ui.separator();

                    ui.label(format!("File to run: {binary:?}"));

                    let clicked = ui.button("Change file").clicked(); //to avoid short-circuiting not showing the button
                    if clicked && binary_dialog.is_none() {
                        let mut dialog = FileDialog::open_file(binary.clone());
                        dialog.open();
                        *binary_dialog = Some(dialog);
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Runs to complete: ");
                        ui.text_edit_singleline(runs_input);
                    });

                    ui.separator();

                    ui.label("CLI Arguments");
                    if !cli_args.is_empty() {
                        ScrollArea::vertical().show(ui, |ui| {
                            //we could have multiple, but immediate mode, so unlikely to affect UX but much easier to juggle for me
                            let mut need_to_remove = None;
                            let mut up = None;
                            let mut down = None;

                            for (i, arg) in cli_args.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.label(arg);
                                    if ui.button("Remove?").clicked() {
                                        need_to_remove = Some(i);
                                    }
                                    if ui.button("Up?").clicked() {
                                        up = Some(i);
                                    }
                                    if ui.button("Down?").clicked() {
                                        down = Some(i);
                                    }
                                });
                            }

                            let len_minus_one = cli_args.len() - 1;
                            if let Some(need_to_remove) = need_to_remove {
                                cli_args.remove(need_to_remove);
                            } else if let Some(up) = up {
                                if up > 0 {
                                    cli_args.swap(up, up - 1);
                                } else {
                                    cli_args.swap(0, len_minus_one);
                                }
                            } else if let Some(down) = down {
                                if down <= len_minus_one {
                                    cli_args.swap(down, down + 1);
                                } else {
                                    cli_args.swap(len_minus_one, 0);
                                }
                            }
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label("New Argument");
                        ui.text_edit_singleline(current_cli_arg);
                    });
                    if ui.button("Submit new argument!").clicked() {
                        cli_args.push(current_cli_arg.clone());
                        current_cli_arg.clear();
                    }

                    if binary.is_some() {
                        let runs = runs_input.parse::<usize>();
                        if let Ok(runs) = runs {
                            ui.separator();
                            if ui.button("Go!").clicked() {
                                self.runs = runs;
                                let (send_stop, recv_stop) = channel();
                                let (handle, run_recv) = Builder::new()
                                    .binary(binary.clone().unwrap())
                                    .runs(runs)
                                    .stop_channel(recv_stop)
                                    .with_cli_args(cli_args.clone())
                                    .start();

                                change = Some(State::Running {
                                    run_times: vec![],
                                    stop: send_stop,
                                    run_recv,
                                    handle,
                                });
                            }
                        }
                    }
                });

                let mut should_close = false;
                if let Some(dialog) = binary_dialog {
                    if dialog.show(ctx).selected() {
                        if let Some(file) = dialog.path() {
                            should_close = true;
                            *binary = Some(file);
                        }
                    }
                }
                if should_close {
                    *binary_dialog = None;
                }
            }
            State::Running {
                run_times,
                stop,
                run_recv,
                handle,
            } => {
                if handle.is_finished() {
                    let max = *run_times.iter().max().unwrap();
                    let min = *run_times.iter().min().unwrap();

                    let avg = run_times.iter().sum::<Duration>() / (run_times.len() as u32);

                    change = Some(State::PostContents {
                        run_times: run_times.clone(),
                        min,
                        max,
                        avg,
                    });
                } else {
                    CentralPanel::default().show(ctx, |ui| {
                        let runs_so_far = run_times.len();

                        ui.label("Running!");
                        ui.label(format!("{} runs left.", self.runs - runs_so_far));
                        ui.separator();

                        show_runs(run_times, ui);
                        ui.separator();

                        ProgressBar::new((runs_so_far as f32) / (self.runs as f32)).ui(ui);

                        if ui.button("Stop!").clicked() {
                            stop.send(()).expect("Cannot send stop signal");
                        }
                    });
                }

                for time in run_recv.try_iter() {
                    run_times.push(time);
                }
            }
            State::PostContents {
                run_times,
                min,
                max,
                avg,
                export_handle
            } => {
                CentralPanel::default().show(ctx, |ui| {
                    ui.label("All runs finished!");
                    ui.separator();
                    show_runs(run_times, ui);
                    ui.separator();

                    ui.label(format!("Max: {max:?}"));
                    ui.label(format!("Min: {min:?}"));
                    ui.label(format!("Mean: {avg:?}"));

                    if export_handle.is_none() {
                        if ui.button("Export to CSV: ").clicked() {
                            let run_times = run_times.clone();
                            *export_handle = Some(std::thread::spawn(move || {
                                let tbr = run_times.into_iter().enumerate().fold(String::new(), |mut st, (i, duration)| {
                                    if i != 0 {
                                        st += &format!(",{duration:?}");
                                    } else {
                                        st += &format!("{duration:?}");
                                    }
                                    st
                                });
                                let mut file = File::create("results.csv")?;
                                writeln!(file, "{tbr}")?;

                                Ok(())
                            }));
                        }
                    }



                    if export_handle.map_or(true, |eh| !eh.is_finished()) {
                        ui.label("Exporting to CSV");
                    } else {
                        let eh = std::mem::take(export_handle).unwrap().join();
                        if let Ok(Err(e)) = eh {
                            eprintln!("Error exprting to CSV: {e}");
                        }
                    }

                });
            }
        }

        if let Some(change) = change {
            if let Some(storage) = frame.storage_mut() {
                self.save(storage);
            }

            self.state = change;
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        if let State::PreContents {
            binary,
            cli_args,
            runs_input,
            ..
        } = &self.state
        {
            if let Some(Ok(binary)) = binary.clone().map(|b| b.into_os_string().into_string()) {
                storage.set_string("binary_path", binary);
            }
            storage.set_string("cli_args", cli_args.join("---,---"));
            storage.set_string("runs", runs_input.to_string());
            storage.flush();
        }
    }
}