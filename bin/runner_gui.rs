//! This is a module designed to store a struct used for egui windowing reasons.
//!
//! ## State
//! Most of the state of the app is stored inside one enum - [`State`], and whilst Clippy would love to tell you about how some internal structs are large, there should only ever be 1 of these per window.
//!
//! Inside the app, we change state on update using an [`Option`] which stores a new state, which gets changed after the match statement on the internal state.

use benchmarker::{
    bencher::{calculate_mean_standard_deviation, Runner, DEFAULT_RUNS},
    io::{export_csv, export_html},
    EGUI_STORAGE_SEPARATOR,
};
use eframe::{App, CreationContext, Frame, Storage, egui::{CentralPanel, ProgressBar, Widget, Context}};
use egui_file::FileDialog;
use itertools::Itertools;
use crate::egui_utils::EguiList;
use std::{
    ffi::OsStr,
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

///[`State`] has 3 variants - [`State::Setup`], [`State::Running`], and [`State::Finished`]
///
/// - [`State::Setup`] represents the state whilst we're grabbing arguments for the [`Runner`].
/// - [`State::Running`] represents the state whilst we're actively running the binary and keeps track of the runs and getting them.
/// - [`State:PostContents`] represents what we're doing when we've finished - displaying results and stats as well as exporting.
#[allow(clippy::large_enum_variant)]
pub enum State {
    /// [`State::Setup`] represents the state whilst we're grabbing arguments for the [`Runner`].
    Setup {
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
        ///`binary` stores a [`PathBuf`] with the binary we're running
        binary: PathBuf,
    },
    /// [`State:PostContents`] represents what we're doing when we've finished - displaying results and stats as well as exporting.
    Finished {
        /// `run_times` is a [`EguiList`] of [`Duration`]s from the binary run times. If this changes - we need to update `min`, `max`, and `avg`
        run_times: EguiList<Duration>,
        /// `min` is the smallest [`Duration`] from `run_times`
        min: Duration,
        /// `max` is the biggest [`Duration`] from `run_times`
        max: Duration,
        /// `mean` is the mean [`Duration`] from `run_times`
        mean: Duration,
        /// `standard_deviation` is the population standard deviation [`Duration`] from `run_times`
        standard_deviation: Duration,
        /// `export_handle`stores a [`JoinHandle`] from exporting `run_times` to a CSV to avoid blocking in immediate mode and is an [`Option`] to allow us to join the handle when it finishes as that requires ownership.
        export_handle: Option<JoinHandle<io::Result<usize>>>,
        /// `file_name_input` stores a temporary variable to decide the name of the file name
        file_name_input: String,
        /// `trace_name_input` stores a temporary variable to decide the name of the trace
        trace_name_input: String,
        /// File dialog for extra trace names
        extra_trace_names_dialog: Option<FileDialog>,
        /// [`EguiList`] for trace names
        extra_files: EguiList<PathBuf>,
    },
}

impl State {
    ///This creates a new State of [`Self::Setup`], with an empty `current_cli_arg`, no `binary_dialog` and unwrapping `runs_input` to itself or default and `warmup` to itself or false.
    #[instrument]
    fn new_from_args(
        binary: Option<PathBuf>,
        cli_args: Vec<String>,
        runs_input: Option<String>,
        warmup: Option<bool>,
    ) -> Self {
        Self::Setup {
            binary,
            current_cli_arg: String::default(),
            cli_args: EguiList::from(cli_args)
                .is_reorderable(true)
                .is_editable(true),
            binary_dialog: None,
            runs_input: runs_input.unwrap_or_else(|| DEFAULT_RUNS.to_string()),
            warmup: warmup.unwrap_or(false),
        }
    }
}

impl<'a> From<Option<&'a dyn Storage>> for State {
    ///This uses the [`CreationContext`]'s persistent [`Storage`] to build a default state
    #[instrument(skip(cc))]
    fn from(cc: Option<&'a dyn Storage>) -> Self {
        let binary = cc
            .and_then(|s| s.get_string("binary_path")) //if we've got a creation context, grab the binary path
            .map(PathBuf::from); //and turn it into a path
        let cli_args: Vec<String> = cc
            .and_then(|s| s.get_string("cli_args")) //if we've got a creation context, grab the cli args
            .map(|s| {
                if s.is_empty() {
                    //"".split(/* anything */) returns vec![""], which we don't want, so we clear the vec if we see this
                    vec![]
                } else {
                    s.split(EGUI_STORAGE_SEPARATOR) //if not, then split into a string by the separator
                        .map(ToString::to_string)
                        .collect()
                }
            })
            .unwrap_or_default(); //if we didn't get any cli args, just set it to a new vector

        let runs_input = cc.and_then(|s| s.get_string("runs")); //get the runs input
        let warmup = cc
            .and_then(|s| s.get_string("warmup"))
            .and_then(|s| s.parse::<bool>().ok()); //if both warmup is a key, and that evaluates to a bool, then we use Some(that), if not we use None

        Self::new_from_args(binary, cli_args, runs_input, warmup)
    }
}

impl BencherApp {
    ///This creates a new [`BencherApp`], using [`State::from`] from parsing stuff from the [`CreationContext`]'s [`Storage`], which is persistent between shutdowns
    #[instrument(skip(cc))]
    pub fn new(cc: &CreationContext) -> Self {
        //turns the storage into a state
        Self {
            runs: 0,
            state: cc.storage.into(),
        }
    }
}

impl App for BencherApp {
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        //TODO: put this across different methods for each state
        let mut change = None; //Variable to store a new State if we want to change

        //**Huge** match statement on our current state, mutably
        match &mut self.state {
            State::Setup {
                //if we are setting up
                binary,
                binary_dialog,
                runs_input,
                current_cli_arg,
                cli_args,
                warmup,
            } => {
                CentralPanel::default().show(ctx, |ui| {
                    //new central panel
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
                        //if we clicked it, and we don't currently have a dialog open
                        trace!(current_binary=?binary.clone(), "Showing File Dialog");
                        let mut dialog = FileDialog::open_file(binary.clone()); //open a dialog at the location of the current binary, and if we don't have one, its an option so we're all fine
                        dialog.open(); //open the dialog
                        *binary_dialog = Some(dialog); //and save it to the state
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        //Horizontal area to have text appear to the left of a label
                        ui.label("Runs to complete: ");
                        ui.text_edit_singleline(runs_input);
                    });

                    ui.checkbox(warmup, "Do an initial warmup run."); //checkbox for whether or not we do a warmup run

                    ui.separator();

                    ui.label("CLI Arguments");
                    cli_args.display(ui, |arg, _i| arg.to_string()); //display all the CLI arguments, not displaying indicies

                    ui.horizontal(|ui| {
                        ui.label("New Argument");
                        ui.text_edit_singleline(current_cli_arg); //text edit for new argument
                    });
                    if ui.button("Submit new argument!").clicked() {
                        //if we click the new argument button
                        cli_args.push(std::mem::take(current_cli_arg)); //take the current arg - this adds it to the list, and clears the input
                    }

                    if binary.is_some() {
                        //if we have a binary
                        if let Ok(runs) = runs_input.parse::<usize>() {
                            //and we can successfully parse the runs
                            if runs > 0 {
                                // and we have >0 runs
                                ui.separator();
                                if ui.button("Go!").clicked() {
                                    //and we click the go button
                                    trace!("Starting benchmarking");
                                    self.runs = runs; //set the runner app variable for the runs
                                    let (send_stop, recv_stop) = channel(); //Make a new channel for stopping/starting the Runner thread

                                    let (handle, run_recv) = Runner::new(
                                        binary.clone().unwrap(),
                                        cli_args.backing_vec(),
                                        runs,
                                        Some(recv_stop),
                                        *warmup,
                                        true,
                                    )
                                    .start(); //make a new run and start it

                                    change = Some(State::Running {
                                        //make a new State with the relevant variables
                                        run_times: EguiList::default().is_scrollable(true),
                                        stop: send_stop,
                                        run_recv,
                                        handle: Some(handle),
                                        binary: std::mem::take(binary).unwrap(),
                                    });
                                }
                            }
                        }
                    }
                });

                let mut should_close = false; //temp variable to avoid ownership faff
                if let Some(dialog) = binary_dialog {
                    //if we have a dialog
                    if dialog.show(ctx).selected() {
                        //and a file is selected
                        if let Some(file) = dialog.path() {
                            //and we can get a path from it
                            should_close = true; //say that we need to close
                            *binary = Some(file); //set our binary
                            info!(binary=?binary.clone(), "Picked file");
                        }
                    }
                }
                if should_close {
                    trace!("Closing File Dialog");
                    *binary_dialog = None; //if we need to close, set our dialog to None
                }
            }
            State::Running {
                //if we are running runs
                run_times,
                stop,
                run_recv,
                handle,
                binary,
            } => {
                for time in run_recv.try_iter() {
                    //for every message since we last checked, add it to the buffer
                    run_times.push(time);
                }

                if handle.as_ref().map_or(false, JoinHandle::is_finished) {
                    //if we have a handle, and it is finished
                    match std::mem::take(handle).unwrap().join() {
                        //join the handle and report errors - we can unwrap here as we only go above if we have a handle
                        Ok(Err(e)) => error!(%e, "Error from running handle"),
                        Err(_e) => error!("Error joining running handle"),
                        _ => {}
                    };

                    let max = run_times.iter().max().copied().unwrap_or_default(); //get the max and min
                    let min = run_times.iter().min().copied().unwrap_or_default();
                    let (mean, standard_deviation) = calculate_mean_standard_deviation(
                        &run_times.iter().map(Duration::as_micros).collect_vec(), //have to collect vec as we can't know the size of [u128] at compile-time
                    )
                    .unwrap_or_default(); //get the mean and standard deviation

                    let file_name = format!(
                        "{}_{}",
                        binary
                            .file_name()
                            .and_then(OsStr::to_str) //TODO: make this into its own method for consistency
                            .unwrap_or("bench_results"), //same as the runner cli default file name
                        run_times.len()
                    );
                    change = Some(State::Finished {
                        //make a new state
                        //new state
                        run_times: run_times.clone(),
                        min,
                        max,
                        mean,
                        standard_deviation,
                        export_handle: None,
                        file_name_input: file_name.clone(),
                        trace_name_input: file_name, //same default trace name as file name
                        extra_trace_names_dialog: None,
                        extra_files: EguiList::default(),
                    });
                } else {
                    //if we don't have a finished handle
                    CentralPanel::default().show(ctx, |ui| {
                        let runs_so_far = run_times.len();

                        ui.label("Running!");
                        ui.label(format!("{} runs left.", self.runs - runs_so_far));
                        ui.separator();

                        run_times.display(ui, |dur, i| format!("Run {} took {dur:?}", i + 1)); //display all runs
                        ui.separator();

                        ProgressBar::new((runs_so_far as f32) / (self.runs as f32)).ui(ui); //show all runs and add progress bar

                        if ui.button("Stop!").clicked() {
                            info!("Sending stop signal");
                            stop.send(()).expect("Cannot send stop signal"); //if we want to stop, then send stop message on channel
                        }
                    });
                }
            }
            State::Finished {
                //if we've finished the runs
                run_times,
                min,
                max,
                mean,
                standard_deviation,
                export_handle,
                file_name_input,
                trace_name_input,
                extra_files,
                extra_trace_names_dialog,
            } => {
                CentralPanel::default().show(ctx, |ui| {
                    ui.label("All runs finished!");
                    ui.label(format!(
                        "{mean:?} ± {standard_deviation:?}, from {min:?} to {max:?}."
                    ));

                    ui.separator();
                    run_times.display(ui, |dur, i| format!("Run {i} took {dur:?}"));
                    ui.separator();

                    if ui.button("Go back to start").clicked() {
                        //if we need to go back to the start
                        trace!("Going back to start");
                        change = Some(frame.storage().into()); //restart using the storage
                    }

                    if export_handle.is_none() {
                        //if we aren't currently exporting
                        ui.horizontal(|ui| {
                            ui.label("File name (w/o extension): ");
                            ui.text_edit_singleline(file_name_input); //textedit for file name
                        });

                        ui.horizontal(|ui| {
                            ui.label("Trace name: ");
                            ui.text_edit_singleline(trace_name_input); //textedit for trace name
                        });

                        ui.separator();
                        let clicked = ui.button("Add Extra Traces").clicked(); //to avoid short-circuiting not showing the button
                        if clicked && extra_trace_names_dialog.is_none() {
                            let mut dialog = FileDialog::open_file(None);
                            dialog.open(); //make a new file dialog, open it, and then save it to the State
                            *extra_trace_names_dialog = Some(dialog);
                        }

                        extra_files.display(ui, |file, _i| format!("{file:?}")); //display all of the extra trace file names

                        ui.vertical(|ui| {
                            if ui.button("Export to CSV").clicked() {
                                //if we export to CSV
                                info!("Exporting to CSV");

                                let run_times = run_times.clone(); //thread-local clones to avoid move ownership faffery
                                let file_name_input = file_name_input.clone();
                                let trace_name_input = trace_name_input.clone();
                                let extra_traces = extra_files.backing_vec();

                                *export_handle = Some(
                                    std::thread::Builder::new() //new thread for CSV export to avoid blocking on UI
                                        .name("csv_exporter".into())
                                        .spawn(move || {
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
                                        })
                                        .expect("error creating thread"),
                                );
                            }

                            if ui.button("Export to HTML").clicked() {
                                //if we export to HTML
                                info!("Exporting to HTML");

                                let run_times = run_times.clone(); //thread-local clones to avoid move ownership faffery
                                let file_name_input = file_name_input.clone();
                                let trace_name_input = trace_name_input.clone();
                                let extra_traces = extra_files.backing_vec();

                                *export_handle = Some(
                                    std::thread::Builder::new() //new thread for HTML export to avoid blocking on UI
                                        .name("html_exporter".into())
                                        .spawn(move || {
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
                                        })
                                        .expect("error creating thread"),
                                );
                            }
                        });
                    } else {
                        ui.label("Exporting..."); //if we haven't finished, but have a handle then say we're exporting
                    }
                });

                let mut should_close = false; //temp variable for if we need to close stuff to avoid ownership faffery
                if let Some(dialog) = extra_trace_names_dialog {
                    //if we have a dialog open
                    if dialog.show(ctx).selected() {
                        //and a file is selected
                        if let Some(file) = dialog.path() {
                            //and we can get a path
                            should_close = true; //we need to close
                            extra_files.push(file.clone()); //add the path to the extra files list
                            info!(trace=?file, "Picked file for extra traces");
                        }
                    }
                }
                if should_close {
                    info!("Closing File Dialog for traces");
                    *extra_trace_names_dialog = None;
                }
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

    #[instrument(skip(self, storage))]
    fn save(&mut self, storage: &mut dyn Storage) {
        if let State::Setup {
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
