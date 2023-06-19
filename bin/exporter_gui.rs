//! Binary for dealing with exporting traces to a file via a GUI interface.
//! 
//! It caches which files were picked last save, and then allows you to pick the files to take from (adding their traces to a list), the export name, and whether or not we totally clear out a file when we write to it.
//! 
//! The file reading is done on a separate thread to avoid UI slowing down whilst the file is read.

//imports
use benchmarker::{
    egui_utils::{ChangeType, EguiList},
    io::{export_csv_no_file_input, export_html_no_file_input, import_csv},
    EGUI_STORAGE_SEPARATOR,
};
use eframe::{App, Frame, Storage, egui::{Context, CentralPanel}};
use egui_file::FileDialog;
use itertools::Itertools;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::mpsc::{channel, Receiver, Sender},
};

///Struct for an [`eframe::App`] for exports.
pub struct ExporterApp {
    ///Current list of files we've read from - used for storing to load for next time
    files: Vec<PathBuf>,
    ///List of traces we've read from the above files
    traces: EguiList<(PathBuf, String, Vec<u128>)>,
    ///File dialog for adding new files for traces
    add_file_dialog: Option<FileDialog>,
    ///Sender for files to the loader thread
    file_tx: Sender<PathBuf>,
    ///Receiver to get back traces from the loader thread
    trace_rx: Receiver<(PathBuf, String, Vec<u128>)>,
    ///The name to export the resulting file to, excluding extensions
    export_name: String,
    ///Whether or not we clear all traces associated with a file, when we re-import that file
    remove_existing_files_on_add_existing_file: bool,
}

impl ExporterApp {
    ///Constructor - uses the `storage` to get the files
    #[instrument(skip(storage))]
    pub fn new(storage: Option<&dyn Storage>) -> Self {
        trace!(has_storage=?storage.is_some(), "Starting new Exporter App");
        
        let files: Vec<PathBuf> = storage
            .and_then(|s| s.get_string("files")) //if we have storage, then get out the content from the key "files"
            .map(|s| {
                //if we got the content, then map it to the contents because it is a list
                trace!(?s, "Got files");
                if s.is_empty() {
                    //"".split(/* anything */) returns vec![""], which we don't want, so we check if the string is empty first, before splitting it
                    vec![]
                } else {
                    s.split(EGUI_STORAGE_SEPARATOR)
                        .map(ToString::to_string) //since we get borrowed strings, and its easier to deal with owned strings (as we can't return a borrow of something from inside the function), we make them all into owned strings
                        .collect() //and we make it into
                }
            })
            .unwrap_or_default() //if we didn't get the content, we just make an empty vector
            .into_iter() //which we turn into an iterator
            .unique() //and only get the unique items to avoid duplicates
            .filter_map(|s| {
                //then, we filter through them and map items
                let path = Path::new(&s); //we make a path from the strinh
                if path.exists() {
                    let pb = path.to_path_buf();
                    trace!(?pb, "Found path");
                    Some(pb) //if it exists, we return it, and if not we don't
                } else {
                    warn!(?s, ?path, "File not found");
                    None
                }
            })
            .collect();

        let (file_tx, file_rx) = channel();
        let (trace_tx, trace_rx) = channel(); //here we make 2 channels for where we can send files to the thread and receive traces from the thread

        std::thread::Builder::new() //make a new thread for handling the loading of new files
            .name("exporter_file_loader".into()) //we send files to the thread
            .spawn(move || {
                //then it sends traces back to us
                handle_loading(file_rx, trace_tx); //and stops if we tell it
            })
            .expect("error creating thread");

        for file in &files {
            //for each file we got from the storage, we try and get it from the thread
            file_tx //we don't need to worry about them being invalid, as they get checked above
                .send(file.clone())
                .expect("unable to send files from init load");
        }

        Self {
            files,
            traces: EguiList::default().is_scrollable(true).is_editable(true),
            add_file_dialog: None,
            file_tx,
            trace_rx,
            export_name: String::default(),
            remove_existing_files_on_add_existing_file: false,
        }
    }
}

///This is the meat and potatoes of the loader thread - it basically just waits for files to arrive and parses all of them, and then repeats. If it sees the `stop_rx` complaining, then it stops.
#[allow(clippy::needless_pass_by_value)]
#[instrument]
fn handle_loading(file_rx: Receiver<PathBuf>, trace_tx: Sender<(PathBuf, String, Vec<u128>)>) {
    while let Ok(file) = file_rx.try_recv() {
        match import_csv(file.clone()) {
            Ok(traces) => {
                for (name, list) in traces {
                    trace_tx
                        .send((file.clone(), name, list))
                        .expect("unable to send new trace");
                }
            }
            Err(e) => {
                error!(?e, "Error reading traces");
            }
        };
    }
}
impl App for ExporterApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.label("Benchmarker Imports/Exports"); //add a title
            ui.separator();

            ui.checkbox(
                //make a checkbox
                &mut self.remove_existing_files_on_add_existing_file,
                "Remove old traces when re-adding files?",
            );
            if self.add_file_dialog.is_none() && ui.button("Add new file").clicked() {
                //if we don't have a dialog currently open AND we click the new file button
                let mut dialog = FileDialog::open_file(self.files.last().cloned()); //make a new file dialog with the last file currently open to save the person reopening the directories. since the constructor takes an option, if we don't have any files, it just is None and we don't have to worry about it
                dialog.open(); //open the dialog

                self.add_file_dialog = Some(dialog); //and add it to the member variable
            }
            let mut needs_to_close = false; //variable for if we need to close it to avoid ownership faffery
            if let Some(dialog) = &mut self.add_file_dialog {
                //if we have a dialog, take a mutable reference
                if dialog.show(ctx).selected() {
                    //show it, and if it is selected
                    if let Some(file) = dialog.path() {
                        //and there is a path
                        needs_to_close = true; //we need to now close the dialog

                        if file.extension() == Some(OsStr::new("csv")) {
                            //if it is a CSV file
                            if self.files.contains(&file) {
                                //and we already have it
                                if self.remove_existing_files_on_add_existing_file {
                                    //if we need to remove the old traces from that file
                                    #[allow(clippy::needless_collect)]
                                    let inidicies: Vec<usize> = self
                                        .traces
                                        .iter()
                                        .enumerate()
                                        .filter_map(|(i, (trace_file, _, _))| {
                                            if trace_file == &file {
                                                Some(i)
                                            } else {
                                                None
                                            } //get all the indicies of traces associated with that file
                                        })
                                        .collect(); //ignore clippy error - we need this to avoid borrow checker stuff. afaik its cheaper to collect and into_iter here, than it is to clone all the traces
                                    for (offset, i) in inidicies.into_iter().enumerate() {
                                        self.traces.remove(i - offset); //and remove them
                                    }
                                }
                            } else {
                                self.files.push(file.clone()); //if we don't already have it, we add it
                            }

                            self.file_tx
                                .send(file) //send it to the loader thread
                                .expect("unable to send pathbuf to file tx");
                        } else {
                            error!(?file, "File doesn't end in CSV"); //if we don't get a CSV file, error out
                        }
                    }
                }
            }
            if needs_to_close {
                //if we need to close it, just forget it
                self.add_file_dialog = None;
            }
            ui.separator();

            if !self.traces.is_empty() {
                //if we have any traces
                ui.label("Traces to use:");
                self.traces.display(ui, |(file, name, list), _i| {
                    format!("File: {file:?}, {name} with {} elements.", list.len())
                }); //display each trace with their file names, trace names and number of elements
                ui.separator();
            }

            ui.horizontal(|ui| {
                ui.label("Export File Name"); //text box and label for file name
                ui.text_edit_singleline(&mut self.export_name);

                ui.vertical(|ui| {
                    if ui.button("Export to CSV").clicked() {
                        //export to CSV button with all our traces
                        export_csv_no_file_input(
                            &self.export_name,
                            self.traces
                                .clone()
                                .into_iter()
                                .map(|(_file, name, list)| (name, list))
                                .collect(),
                        )
                        .expect("unable to export files to csv");
                    }
                    if ui.button("Export to HTML").clicked() {
                        //export to HTML button with all our traces
                        export_html_no_file_input(
                            &self.export_name,
                            self.traces
                                .clone()
                                .into_iter()
                                .map(|(_file, name, list)| (name, list))
                                .collect(),
                        )
                        .expect("unable to export files to csv");
                    }
                });
            });
        });

        while let Ok(new_trace) = self.trace_rx.try_recv() {
            //poll our trace receiver for new traces. use try_recv to avoid blocking on a UI thread
            self.traces.push(new_trace); //and add all of them
        }

        if let Some(change) = self.traces.had_update() {
            match change {
                //if our traces EguiList had an update, match on it
                ChangeType::Removed(_) => {
                    //here, we go through every trace in our traces EguiList, keep the unique ones, and then get owned versions of the remaining ones
                    self.files = self
                        .traces
                        .iter()
                        .map(|(file, _, _)| file)
                        .unique()
                        .cloned()
                        .collect(); //we can't just remove the item, as we might still have other traces from that file
                }
                _ => {
                    trace!(?change, "list change in exporter traces list");
                }
            }
        }
    }

    #[instrument(skip(self, storage))]
    fn save(&mut self, storage: &mut dyn Storage) {
        let files_to_save = self
            .files
            .clone()
            .into_iter() //for each file
            .filter_map(|pb| match pb.into_os_string().into_string() {
                Ok(p) => Some(p), //if we can turn it into a string, do it
                Err(e) => {
                    error!(?e, "Error getting String of PathBuf for saving.");
                    None //if not, we just don't save it
                }
            })
            .join(EGUI_STORAGE_SEPARATOR); //join with the separator into a String
        trace!("Saving current files");
        storage.set_string("files", files_to_save); //and save them
    }
}
