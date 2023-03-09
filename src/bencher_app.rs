//! This is a module designed to store a struct used for egui windowing reasons.
//!
//! ## State
//! Most of the state of the app is stored inside one enum - [`State`], and whilst Clippy would love to tell you about how some internal structs are large, there should only ever be 1 of these per window.
//!
//! Inside the app, we change state on update using an [`Option`] which stores a new state, which gets changed after the match statement on the internal state.

use crate::bencher::Builder;
use eframe::{App, CreationContext, Frame, Storage};
use egui::{CentralPanel, Context, ProgressBar, ScrollArea, Ui, Widget};
use egui_file::FileDialog;
use plotly::{Histogram, Plot};
use std::{
    collections::HashMap,
    fs::File,
    hash::Hash,
    io::{self, Write},
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
/// - [`State::PreContents`] represents the state whilst we're grabbing arguments for the [`Builder`].
/// - [`State::Running`] represents the state whilst we're actively running the binary and keeps track of the runs and getting them.
/// - [`State:PostContents`] represents what we're doing when we've finished - displaying results and stats as well as exporting.
///
/// ## Pre
///
/// Most of them are optional, but we can only hit *Go!* if we have a valid integer for `runs` and a file for `binary`
///
/// This stores all of the important variables for running the binaries:
/// - `binary` stores an [`Option`] of a [`PathBuf`] which is the binary we are going to run - Optional because the user doesn't have one when they first open the app.
/// - `binary_dialog` stores an [`Option`] of a [`FileDialog`] which is the Dialog object from [`egui_file`] that lets a user pick a file - NB: no validation on whether or not it is a binary
/// - `cli_args` stores a [`Vec`] of [`String`]s for all of the arguments we'll pass to `binary`
/// - `current_cli_arg` stores a temporary [`String`] for user input of the next `cli_arg` to add to the list
/// - `runs_input` stores a temporary [`String`] for user input of the `runs`
/// - `show_output_in_console` stores a [`bool`] on whether or not to redirect output from the program
///
///
/// ## During
///
/// This stores all the important variables whilst running the binary.
/// - `run_times` is a [`Vec`] of [`Duration`]s that we've received so far from the [`Builder`]
/// - `stop` is a unit tuple [`Sender`] which allows us to tell the [`Builder`] thread to stop execution as soon as it finishes with the current chunk.
/// - `run_recv` is a [`Receiver`] for getting new [`Duration`]s to send to `run_times`.
/// - `handle` stores a [`JoinHandle`] from [`Builder`], and is an [`Option`] to allow us to join the handle when it finishes as that requires ownership.
///
/// ## After
///
/// This stores all the variables for giving things to users, and mostly stay constants apart from the `export_handle`
/// - `run_times` is a [`Vec`] of [`Duration`]s from the binary run times.
/// - `min` is the smallest [`Duration`] from `run_times`
/// - `max` is the biggest [`Duration`] from `run_times`
/// - `avg` is the mean [`Duration`] from `run_times`
/// - `export_handle`stores a [`JoinHandle`] from exporting `run_times` to a CSV to avoid blocking in immediate mode and is an [`Option`] to allow us to join the handle when it finishes as that requires ownership.
#[allow(clippy::large_enum_variant)]
pub enum State {
    /// [`State::PreContents`] represents the state whilst we're grabbing arguments for the [`Builder`].
    PreContents {
        /// `binary` stores an [`Option`] of a [`PathBuf`] which is the binary we are going to run - Optional because the user doesn't have one when they first open the app.
        binary: Option<PathBuf>,
        /// `binary_dialog` stores an [`Option`] of a [`FileDialog`] which is the Dialog object from [`egui_file`] that lets a user pick a file - NB: no validation on whether or not it is a binary
        binary_dialog: Option<FileDialog>, //don't care if it is big - I'll only ever have one `State`
        /// `cli_args` stores a [`Vec`] of [`String`]s for all of the arguments we'll pass to `binary`
        cli_args: Vec<String>,
        /// `current_cli_arg` stores a temporary [`String`] for user input of the next `cli_arg` to add to the list
        current_cli_arg: String,
        /// `runs_input` stores a temporary [`String`] for user input of the `runs`
        runs_input: String,
        /// `show_output_in_console` stores a [`bool`] on whether or not to redirect output from the program
        show_output_in_console: bool,
    },
    /// [`State::Running`] represents the state whilst we're actively running the binary and keeps track of the runs and getting them.
    Running {
        /// `run_times` is a [`Vec`] of [`Duration`]s that we've received so far from the [`Builder`]
        run_times: Vec<Duration>,
        /// `stop` is a unit tuple [`Sender`] which allows us to tell the [`Builder`] thread to stop execution as soon as it finishes with the current chunk.
        stop: Sender<()>,
        /// `run_recv` is a [`Receiver`] for getting new [`Duration`]s to send to `run_times`.
        run_recv: Receiver<Duration>,
        /// `handle` stores a [`JoinHandle`] from [`Builder`], and is an [`Option`] to allow us to join the handle when it finishes as that requires ownership.
        handle: Option<JoinHandle<io::Result<()>>>,
    },
    /// [`State:PostContents`] represents what we're doing when we've finished - displaying results and stats as well as exporting.
    PostContents {
        /// `run_times` is a [`Vec`] of [`Duration`]s from the binary run times. If this changes - we need to update `min`, `max`, and `avg`
        run_times: Vec<Duration>,
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
    },
}

impl State {
    ///This creates a new State of [`Self::PreContents`], with an empty `current_cli_arg`, no `binary_dialog` and unwrapping `runs_input` to itself or default and `show_output_in_console` to itself or false.
    fn new(
        binary: Option<PathBuf>,
        cli_args: Vec<String>,
        runs_input: Option<String>,
        show_output_in_console: Option<bool>,
    ) -> Self {
        Self::PreContents {
            binary,
            current_cli_arg: String::default(),
            cli_args,
            binary_dialog: None,
            runs_input: runs_input.unwrap_or_default(),
            show_output_in_console: show_output_in_console.unwrap_or(false),
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
                    s.split("---,---").map(ToString::to_string).collect() //"".split(/* anything */) returns vec![""], which we don't want, so we clear the vec if we see this
                }
            })
            .unwrap_or_default();

        let runs_input = cc.and_then(|s| s.get_string("runs")); //get the runs input
        let show_output_in_console = cc
            .and_then(|s| s.get_string("show_output_in_console"))
            .and_then(|s| s.parse::<bool>().ok()); //if both show_output_in_console is a key, and that evaluates to a bool, then we use Some(that), if not we use None

        State::new(binary, cli_args, runs_input, show_output_in_console)
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
        ///Show all of the runs in a [`ScrollArea`] numbering each of them, of height 300
        fn show_runs(runs: &[Duration], ui: &mut Ui) {
            ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for (i, run) in runs.iter().enumerate() {
                    ui.label(format!("Run {} took {run:?}.", i + 1));
                }
            });
        }

        let mut change = None; //Variable to store a new State if we want to change

        //Huge match statement on our current state, mutably
        match &mut self.state {
            State::PreContents {
                binary,
                binary_dialog,
                runs_input,
                current_cli_arg,
                cli_args,
                show_output_in_console,
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

                    ui.checkbox(show_output_in_console, "Show output in console");

                    ui.separator();

                    ui.label("CLI Arguments");
                    if !cli_args.is_empty() {
                        //If we don't have any arguments, then we don't need any of this and some of the logic gets screwed
                        ScrollArea::vertical().show(ui, |ui| {
                            //we could have multiple (as in vecs rather than options), but immediate mode, so unlikely to affect UX but much easier to juggle for me
                            let mut need_to_remove = None; //we need to remove this index
                            let mut up = None; //move this index up a position
                            let mut down = None; //move this index down a position

                            for (i, arg) in cli_args.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    //for each of our CLI args, make a new horizontal environment (to almost mimic a table without alignment), and add buttons for remove/up/down, and if we get input then set relevant variables
                                    ui.label(arg); //Set
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
                                let removed = cli_args.remove(need_to_remove);
                                info!(?removed, "Removing CLI Arg");
                            } else if let Some(up) = up {
                                //extra code with checking <> 0 for wrapping around rather than just normal swapping
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
                            if runs > 0 {
                                ui.separator();
                                if ui.button("Go!").clicked() {
                                    //only make a button if we have a valid runs, and we have a binary
                                    info!("Starting a new run!");
                                    self.runs = runs;
                                    let (send_stop, recv_stop) = channel(); //Make a new channel for stopping/starting the Builder thread

                                    if let Some((handle, run_recv)) = Builder::new()
                                        .binary(binary.clone().unwrap())
                                        .runs(runs)
                                        .stop_channel(recv_stop)
                                        .with_cli_args(cli_args.clone())
                                        .with_show_console_output(*show_output_in_console) //Make a new builder using Builder pattern
                                        .start()
                                    {
                                        change = Some(State::Running {
                                            //make a new State with the relevant variables
                                            run_times: vec![],
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
                    });
                } else {
                    CentralPanel::default().show(ctx, |ui| {
                        let runs_so_far = run_times.len();

                        ui.label("Running!");
                        ui.label(format!("{} runs left.", self.runs - runs_so_far));
                        ui.separator();

                        show_runs(run_times, ui);
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
            } => {
                CentralPanel::default().show(ctx, |ui| {
                    ui.label("All runs finished!");
                    ui.separator();
                    show_runs(run_times, ui);
                    ui.separator();

                    ui.label(format!("Max: {max:?}"));
                    ui.label(format!("Min: {min:?}"));
                    ui.label(format!("Mean: {avg:?}")); //display min/max/avg/runs

                    ui.separator();

                    if ui.button("Go back to start").clicked() {
                        info!("Going back to start");
                        change = Some(Self::default_state_from_cc(frame.storage()));
                    }

                    //if we aren't currently exporting
                    if export_handle.is_none() {
                        ui.horizontal(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("File name (w/o extension): ");
                                ui.text_edit_singleline(file_name_input);
                            });

                            ui.vertical(|ui| {
                                if ui.button("Export to CSV").clicked() {
                                    info!("Exporting to CSV");

                                    let run_times = run_times.clone(); //clone this for the thread
                                    let file_name_input = file_name_input.clone();

                                    *export_handle = Some(std::thread::spawn(move || {
                                        let map = map_from_vec(
                                            run_times.into_iter().map(|x| x.as_micros()),
                                        );
                                        let tbr = map.into_iter().fold(
                                            String::new(),
                                            |st, (micros, count)| {
                                                st + &format!("{micros},{count}\n")
                                            },
                                        ); //create the CSV by folding on a String
                                        let mut file = File::create(file_name_input + ".csv")?;
                                        let tbr = format!("run_time_micros,count\n{tbr}");
                                        file.write(tbr.as_bytes()) //create a file, write to it, return errors or bytes written
                                    }));
                                }

                                if ui.button("Export to HTML").clicked() {
                                    info!("Exporting to HTML");

                                    let run_times = run_times.clone(); //thread-local clone
                                    let file_name_input = file_name_input.clone();

                                    *export_handle = Some(std::thread::spawn(move || {
                                        let mut plot = Plot::new();
                                        plot.add_trace(Histogram::new(
                                            run_times.clone().into_iter().map(|x| x.as_micros()).collect(),
                                        ).name("Yeet"));
                                        plot.add_trace(Histogram::new(
                                            run_times.into_iter().map(|x| x.as_micros() / 2).collect(),
                                        ).name("Skeet"));
                                        let html = plot.to_html();

                                        let mut file = File::create(file_name_input + ".html")?;
                                        let bytes = html.as_bytes();
                                        file.write_all(bytes)?;

                                        Ok(bytes.len())
                                    }));
                                }
                            });
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
            show_output_in_console,
            ..
        } = &self.state
        {
            if let Some(Ok(binary)) = binary.clone().map(|b| b.into_os_string().into_string()) {
                //only save binary if we can export it to a valid String - utf-8 issues can arise from PathBufs
                storage.set_string("binary_path", binary);
            }
            storage.set_string("cli_args", cli_args.join("---,---"));
            storage.set_string("runs", runs_input.to_string());
            storage.set_string("show_output_in_console", show_output_in_console.to_string());

            info!("Saved stuff");

            storage.flush();
        }
    }
}

///Converts an [`Iterator`] into a [`HashMap`] of how many times each item occurs
fn map_from_vec<N: Hash + Eq>(vec: impl Iterator<Item = N>) -> HashMap<N, usize> {
    let mut map = HashMap::new();
    for item in vec {
        *map.entry(item).or_default() += 1_usize;
    }
    map
}
