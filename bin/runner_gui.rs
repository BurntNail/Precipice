//! This is a module designed to store a struct used for egui windowing reasons.
//!
//! ## State
//! Most of the state of the app is stored inside one enum - [`State`], and whilst Clippy would love to tell you about how some internal structs are large, there should only ever be 1 of these per window.
//!
//! Inside the app, we change state on update using an [`Option`] which stores a new state, which gets changed after the match statement on the internal state.

use benchmarker::{
    bencher::Runner,
    egui_utils::EguiList,
    io::{export_csv, export_html},
    EGUI_STORAGE_SEPARATOR,
};
use eframe::{App, CreationContext, Frame, Storage};
use egui::{CentralPanel, Context, ProgressBar, Widget};
use egui_file::FileDialog;
use std::{
    io,
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
    thread::JoinHandle,
    time::Duration,
};

///[`egui`] app to actually run the benchmark - most of the state is stored inside [`State`]
pub struct BencherApp {
    ///The number of runs we're going to do
    runs: usize,
    ///**The** [`State`]
    state: State,
}

///[`State`] has 3 variants - [`State::PreContents`], [`State::Running`], and [`State::PostContents`]
///
/// - [`State::PreContents`] represents the state whilst we're grabbing arguments for the [`Runner`].
/// - [`State::Running`] represents the state whilst we're actively running the binary and keeps track of the runs and getting them.
/// - [`State:PostContents`] represents what we're doing when we've finished - displaying results and stats as well as exporting.
#[allow(clippy::large_enum_variant)]
pub enum State {
    /// [`State::PreContents`] represents the state whilst we're grabbing arguments for the [`Runner`].
    PreContents {
        /// `binary` stores an [`Option`] of a [`PathBuf`] which is the binary we are going to run - Optional because the user doesn't have one when they first open the app.
        binary: Option<PathBuf>,
        /// `binary_dialog` stores an [`Option`] of a [`FileDialog`] which is the Dialog object from [`egui_file`] that lets a user pick a file - NB: no validation on whether or not it is a binary
        binary_dialog: Option<FileDialog>, //don't care if it is big - I'll only ever have one `State`
        /// `cli_args` stores a [`EguiList`] of [`String`]s for all of the arguments we'll pass to `binary`
        cli_args: EguiList<String>,
        /// `current_cli_arg` stores a temporary [`String`] for user input of the next `cli_arg` to add to the list
        current_cli_arg: String,
        /// `runs_input` stores a temporary [`String`] for user input of the `runs`
        runs_input: String,
        /// `warmup` stores a [`bool`] on whether or not to do a warmup run
        warmup: bool,
    },
    /// [`State::Running`] represents the state whilst we're actively running the binary and keeps track of the runs and getting them.
    Running {
        /// `run_times` is a [`EguiList`] of [`Duration`]s that we've received so far from the [`Runner`]
        run_times: EguiList<Duration>,
        /// `stop` is a unit tuple [`Sender`] which allows us to tell the [`Runner`] thread to stop execution as soon as it finishes with the current chunk.
        stop: Sender<()>,
        /// `run_recv` is a [`Receiver`] for getting new [`Duration`]s to send to `run_times`.
        run_recv: Receiver<Duration>,
        /// `handle` stores a [`JoinHandle`] from [`Runner`], and is an [`Option`] to allow us to join the handle when it finishes as that requires ownership.
        handle: Option<JoinHandle<io::Result<()>>>,
    },
    /// [`State:PostContents`] represents what we're doing when we've finished - displaying results and stats as well as exporting.
    PostContents {
        /// `run_times` is a [`EguiList`] of [`Duration`]s from the binary run times. If this changes - we need to update `min`, `max`, and `avg`
        run_times: EguiList<Duration>,
        /// `min` is the smallest [`Duration`] from `run_times`
        min: Duration,
        /// `max` is the biggest [`Duration`] from `run_times`
        max: Duration,
        /// `avg` is the mean [`Duration`] from `run_times`
        avg: Duration,
        /// `export_handle`stores a [`JoinHandle`] from exporting `run_times` to a CSV to avoid blocking in immediate mode and is an [`Option`] to allow us to join the handle when it finishes as that requires ownership.
        export_handle: Option<JoinHandle<io::Result<usize>>>,
        /// `file_name_input` stores a temporary variable to decide the name of the file name
        file_name_input: String,
        /// `trace_name_input` stores a temporary variable to decide the name of the trace
        trace_name_input: String,
        /// File dialog for extra trace names
        extra_trace_names_dialog: Option<FileDialog>,
        /// [`EguiList`] for trace names
        extra_traces: EguiList<PathBuf>,
    },
}

impl State {
    ///This creates a new State of [`Self::PreContents`], with an empty `current_cli_arg`, no `binary_dialog` and unwrapping `runs_input` to itself or default and `warmup` to itself or false.
    fn new(
        binary: Option<PathBuf>,
        cli_args: Vec<String>,
        runs_input: Option<String>,
        warmup: Option<bool>,
    ) -> Self {
        Self::PreContents {
            binary,
            current_cli_arg: String::default(),
            cli_args: EguiList::from(cli_args)
                .is_reorderable(true)
                .is_editable(true),
            binary_dialog: None,
            runs_input: runs_input.unwrap_or_default(),
            warmup: warmup.unwrap_or(false),
        }
    }
}

impl BencherApp {
    ///This uses the [`CreationContext`]'s persistent [`Storage`] to build a default state
    fn default_state_from_cc(cc: Option<&dyn Storage>) -> State {
        let binary = cc
            .and_then(|s| s.get_string("binary_path"))
            .map(PathBuf::from); //get the binary path and make it into a PathBuf
        let cli_args: Vec<String> = cc
            .and_then(|s| s.get_string("cli_args"))
            .map(|s| {
                if s.is_empty() {
                    vec![]
                } else {
                    s.split(EGUI_STORAGE_SEPARATOR)
                        .map(ToString::to_string)
                        .collect() //"".split(/* anything */) returns vec![""], which we don't want, so we clear the vec if we see this
                }
            })
            .unwrap_or_default();

        let runs_input = cc.and_then(|s| s.get_string("runs")); //get the runs input
        let warmup = cc
            .and_then(|s| s.get_string("warmup"))
            .and_then(|s| s.parse::<bool>().ok()); //if both warmup is a key, and that evaluates to a bool, then we use Some(that), if not we use None

        State::new(binary, cli_args, runs_input, warmup)
    }

    ///This creates a new [`BencherApp`], using [`State::new`] from parsing stuff from the [`CreationContext`]'s [`Storage`], which is persistent between shutdowns
    pub fn new(cc: &CreationContext) -> Self {
        Self {
            runs: 0,
            state: Self::default_state_from_cc(cc.storage),
        }
    }
}

impl App for BencherApp {
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        let mut change = None; //Variable to store a new State if we want to change

        //Huge match statement on our current state, mutably
        match &mut self.state {
            State::PreContents {
                binary,
                binary_dialog,
                runs_input,
                current_cli_arg,
                cli_args,
                warmup,
            } => {
                CentralPanel::default().show(ctx, |ui| {
                    ui.label("Preparing to Bench"); //title label
                    ui.separator();

                    //If we have a binary, display it, if not say we don't have one yet
                    if let Some(binary) = binary {
                        ui.label(format!("File to run: {binary:?}"));
                    } else {
                        ui.label("No file selected");
                    }

                    let clicked = ui.button("Change file").clicked(); //to avoid short-circuiting not showing the button
                    if clicked && binary_dialog.is_none() {
                        info!(binary=?binary.clone(), "Showing File Dialog");
                        let mut dialog = FileDialog::open_file(binary.clone());
                        dialog.open(); //make a new file dialog, open it, and then save it to the State
                        *binary_dialog = Some(dialog);
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        //Horizontal area to have text appear to the left of a label
                        ui.label("Runs to complete: ");
                        ui.text_edit_singleline(runs_input);
                    });

                    ui.checkbox(warmup, "Do an initial warmup run.");

                    ui.separator();

                    ui.label("CLI Arguments");
                    cli_args.display(ui, |arg, _i| arg.to_string()); //display all the CLI arguments, not caring about indicies

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
                            if runs > 0 {
                                //only if we have >0 runs, can we actually start the runs - if you want to deal with exports use that program not this one
                                ui.separator();
                                if ui.button("Go!").clicked() {
                                    //only make a button if we have a valid runs, and we have a binary
                                    info!("Starting a new run!");
                                    self.runs = runs;
                                    let (send_stop, recv_stop) = channel(); //Make a new channel for stopping/starting the Builder thread

                                    if let Some((handle, run_recv)) = Runner::new(std::mem::take(binary).unwrap(), cli_args.backing_vec(), runs, Some(recv_stop), *warmup)
                                        .start()
                                    {
                                        change = Some(State::Running {
                                            //make a new State with the relevant variables
                                            run_times: EguiList::default().is_scrollable(true),
                                            stop: send_stop,
                                            run_recv,
                                            handle: Some(handle),
                                        });
                                    } else {
                                        warn!("Tried to start Builder with missing variables...");
                                        //If we can't make a builder, then we are missing variables
                                    }
                                }
                            }
                        }
                    }
                });

                let mut should_close = false;
                if let Some(dialog) = binary_dialog {
                    if dialog.show(ctx).selected() {
                        //if we select a file, and it is a file then we need to close the thing (after we can get ownership of it, later), and set the binary
                        if let Some(file) = dialog.path() {
                            should_close = true;
                            *binary = Some(file);
                            info!(binary=?binary.clone(), "Picked file");
                        }
                    }
                }
                if should_close {
                    info!("Closing File Dialog");
                    *binary_dialog = None;
                }
            }
            State::Running {
                run_times,
                stop,
                run_recv,
                handle,
            } => {
                for time in run_recv.try_iter() {
                    //for every message since we last checked, add it to the buffer
                    trace!(?time, "Adding new time");
                    run_times.push(time);
                }

                if handle.as_ref().map_or(false, JoinHandle::is_finished) {
                    //if we have a handle that has yet finished
                    match std::mem::take(handle).unwrap().join() {
                        //join the handle and report errors
                        Ok(Err(e)) => error!(%e, "Error from running handle"),
                        Err(_e) => error!("Error joining running handle"),
                        _ => {}
                    };

                    let max = run_times.iter().max().copied().unwrap_or_default();
                    let min = run_times.iter().min().copied().unwrap_or_default();
                    let avg = if run_times.is_empty() {
                        Duration::default()
                    } else {
                        run_times.iter().sum::<Duration>() / (run_times.len() as u32)
                    }; //calc min/max/avg

                    change = Some(State::PostContents {
                        //new state
                        run_times: run_times.clone(),
                        min,
                        max,
                        avg,
                        export_handle: None,
                        file_name_input: "results".into(),
                        trace_name_input: "trace".into(),
                        extra_trace_names_dialog: None,
                        extra_traces: EguiList::default(), //TODO: cache this over time with the others
                    });
                } else {
                    CentralPanel::default().show(ctx, |ui| {
                        let runs_so_far = run_times.len();

                        ui.label("Running!");
                        ui.label(format!("{} runs left.", self.runs - runs_so_far));
                        ui.separator();

                        run_times.display(ui, |dur, i| format!("Run {i} took {dur:?}")); //display all runs
                        ui.separator();

                        ProgressBar::new((runs_so_far as f32) / (self.runs as f32)).ui(ui); //show all runs and add progress bar

                        if ui.button("Stop!").clicked() {
                            info!("Sending stop signal"); //if we want to stop, then send message on channel
                            stop.send(()).expect("Cannot send stop signal");
                        }
                    });
                }
            }
            State::PostContents {
                run_times,
                min,
                max,
                avg,
                export_handle,
                file_name_input,
                trace_name_input,
                extra_traces,
                extra_trace_names_dialog,
            } => {
                CentralPanel::default().show(ctx, |ui| {
                    ui.label("All runs finished!");
                    ui.separator();
                    run_times.display(ui, |dur, i| format!("Run {i} took {dur:?}"));
                    ui.separator();

                    ui.label(format!("Max: {max:?}"));
                    ui.label(format!("Min: {min:?}"));
                    ui.label(format!("Mean: {avg:?}")); //display min/max/avg/runs

                    ui.separator();

                    if ui.button("Go back to start").clicked() {
                        info!("Going back to start");
                        change = Some(Self::default_state_from_cc(frame.storage()));
                        //option for going back to the start
                    }

                    //if we aren't currently exporting
                    if export_handle.is_none() {
                        ui.horizontal(|ui| {
                            ui.label("File name (w/o extension): ");
                            ui.text_edit_singleline(file_name_input);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Trace name: ");
                            ui.text_edit_singleline(trace_name_input);
                        });

                        ui.separator();
                        let clicked = ui.button("Add Extra Traces").clicked(); //to avoid short-circuiting not showing the button
                        if clicked && extra_trace_names_dialog.is_none() {
                            let mut dialog = FileDialog::open_file(None);
                            dialog.open(); //make a new file dialog, open it, and then save it to the State
                            *extra_trace_names_dialog = Some(dialog);
                        }

                        extra_traces.display(ui, |file, _i| format!("{file:?}"));

                        ui.vertical(|ui| {
                            if ui.button("Export to CSV").clicked() {
                                info!("Exporting to CSV");

                                let run_times = run_times.clone(); //thread-local clones
                                let file_name_input = file_name_input.clone();
                                let trace_name_input = trace_name_input.clone();
                                let extra_traces = extra_traces.backing_vec();

                                *export_handle = Some(std::thread::spawn(move || {
                                    export_csv(
                                        //start a CSV export
                                        Some((
                                            trace_name_input,
                                            run_times
                                                .backing_vec()
                                                .into_iter()
                                                .map(|d| d.as_micros())
                                                .collect(),
                                        )),
                                        file_name_input,
                                        extra_traces,
                                    )
                                }));
                            }

                            if ui.button("Export to HTML").clicked() {
                                info!("Exporting to HTML");

                                let run_times = run_times.clone(); //thread-local clones
                                let file_name_input = file_name_input.clone();
                                let trace_name_input = trace_name_input.clone();
                                let extra_traces = extra_traces.backing_vec();

                                *export_handle = Some(std::thread::spawn(move || {
                                    export_html(
                                        //start an HTML export
                                        Some((
                                            trace_name_input,
                                            run_times
                                                .backing_vec()
                                                .into_iter()
                                                .map(|d| d.as_micros())
                                                .collect(),
                                        )),
                                        file_name_input,
                                        extra_traces,
                                    )
                                }));
                            }
                        });
                    } else if export_handle
                        .as_ref()
                        .map_or(false, JoinHandle::is_finished)
                    //if the handle has finished
                    {
                        match std::mem::take(export_handle).unwrap().join() {
                            //report errors
                            Ok(Err(e)) => error!(%e, "Error exporting"),
                            Ok(Ok(n)) => info!(%n, "Finished exporting"),
                            Err(_e) => error!("Error joining handle for exporting"),
                        }
                    } else {
                        ui.label("Exporting..."); //if we haven't finished, but have a handle then say we're exporting
                    }
                });

                let mut should_close = false;
                if let Some(dialog) = extra_trace_names_dialog {
                    if dialog.show(ctx).selected() {
                        //if we select a file, and it is a file then we need to close the thing (after we can get ownership of it, later), and set the binary
                        if let Some(file) = dialog.path() {
                            should_close = true;
                            extra_traces.push(file.clone());
                            info!(trace=?file, "Picked file for extra traces");
                        }
                    }
                }
                if should_close {
                    info!("Closing File Dialog for traces");
                    *extra_trace_names_dialog = None;
                } //TODO: make own class/method for these dialogs
            }
        }

        if let Some(change) = change {
            //if we need to change, then save stuff and change
            if let Some(storage) = frame.storage_mut() {
                info!("Saving on state change");
                self.save(storage);
            }

            self.state = change;
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        if let State::PreContents {
            //we only need to save Pre stuff, so check if we've got that
            binary,
            cli_args,
            runs_input,
            warmup,
            ..
        } = &self.state
        {
            if let Some(Ok(binary)) = binary.clone().map(|b| b.into_os_string().into_string()) {
                //only save binary if we can export it to a valid String - utf-8 issues can arise from PathBufs
                storage.set_string("binary_path", binary);
            }
            storage.set_string("cli_args", cli_args.join(EGUI_STORAGE_SEPARATOR));
            storage.set_string("runs", runs_input.to_string());
            storage.set_string("warmup", warmup.to_string());

            info!("Saved stuff");

            storage.flush();
        }
    }
}
