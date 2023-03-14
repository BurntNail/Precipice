use benchmarker::{
    io::{export_csv_no_file_input, export_html_no_file_input, import_csv},
    list::EguiList,
    EGUI_STORAGE_SEPARATOR,
};
use eframe::{App, Frame, Storage};
use egui::{CentralPanel, Context};
use egui_file::FileDialog;
use itertools::Itertools;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::mpsc::{channel, Receiver, Sender},
    thread::JoinHandle,
};
use tracing::{error, info, warn};

pub struct ExporterApp {
    files: Vec<PathBuf>,
    traces: EguiList<(PathBuf, String, Vec<u128>)>,
    add_file_dialog: Option<FileDialog>,
    loader_thread: Option<JoinHandle<()>>,
    file_tx: Sender<PathBuf>,
    stop_tx: Sender<()>,
    trace_rx: Receiver<(PathBuf, String, Vec<u128>)>,
    export_name: String,
}

impl ExporterApp {
    pub fn new(storage: Option<&dyn Storage>) -> Self {
        info!("Starting new EA");
        let files = storage
            .and_then(|s| s.get_string("files"))
            .map(|s| {
                if s.is_empty() {
                    vec![]
                } else {
                    s.split(EGUI_STORAGE_SEPARATOR)
                        .map(ToString::to_string)
                        .collect() //"".split(/* anything */) returns vec![""], which we don't want, so we clear the vec if we see this
                }
            })
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| {
                let path = Path::new(&s);
                if path.exists() {
                    Some(path.to_path_buf())
                } else {
                    warn!(?s, ?path, "File not found");
                    None
                }
            })
            .collect();

        let (file_tx, file_rx) = channel();
        let (stop_tx, stop_rx) = channel();
        let (trace_tx, trace_rx) = channel();

        let handle = std::thread::spawn(move || {
            handle_loading(file_rx, stop_rx, trace_tx);
        });

        Self {
            files,
            traces: EguiList::default().is_scrollable(true).is_editable(true),
            add_file_dialog: None,
            loader_thread: Some(handle),
            file_tx,
            stop_tx,
            trace_rx,
            export_name: String::default(),
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn handle_loading(
    file_rx: Receiver<PathBuf>,
    stop_rx: Receiver<()>,
    trace_tx: Sender<(PathBuf, String, Vec<u128>)>,
) {
    while stop_rx.try_recv().is_err() {
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
}
impl App for ExporterApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.label("Benchmarker Imports/Exports");
            ui.separator();

            if self.add_file_dialog.is_none() && ui.button("Add new file").clicked() {
                let mut dialog = FileDialog::open_file(self.files.get(0).cloned());
                dialog.open();

                self.add_file_dialog = Some(dialog);
            }
            let mut needs_to_close = false;
            if let Some(dialog) = &mut self.add_file_dialog {
                if dialog.show(ctx).selected() {
                    if let Some(file) = dialog.path() {
                        needs_to_close = true;

                        if file.extension() == Some(OsStr::new("csv")) {
                            self.files.push(file.clone());
                            self.file_tx
                                .send(file)
                                .expect("unable to send pathbuf to file tx");
                        } else {
                            error!(?file, "File doesn't end in CSV");
                        }
                    }
                }
            }
            if needs_to_close {
                self.add_file_dialog = None;
            }
            ui.separator();

            if !self.traces.is_empty() {
                ui.label("Traces to use:");
                self.traces.display(ui, |(file, name, list), _i| {
                    format!("File: {file:?}, {name} with {} elements.", list.len())
                });
                ui.separator();
            }

            ui.horizontal(|ui| {
                ui.label("File Name");
                ui.text_edit_singleline(&mut self.export_name);

                ui.vertical(|ui| {
                    if ui.button("Export to CSV").clicked() {
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
            self.traces.push(new_trace);
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        let files_to_save = self
            .files
            .clone()
            .into_iter()
            .filter_map(|pb| match pb.into_os_string().into_string() {
                Ok(p) => Some(p),
                Err(e) => {
                    error!(?e, "Error getting String of PathBuf for saving.");
                    None
                }
            })
            .join(EGUI_STORAGE_SEPARATOR);
        storage.set_string("files", files_to_save);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.stop_tx.send(()); //only errors are around dropped threads - here thats a pro not a con
        if let Some(handle) = std::mem::take(&mut self.loader_thread) {
            handle.join().expect("unable to join loader handle");
        }
    }
}
