use benchmarker::list::EguiList;
use eframe::{App, Frame, Storage};
use egui::{CentralPanel, Context};
use egui_file::FileDialog;
use itertools::Itertools;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

pub struct ExporterApp {
    files: EguiList<PathBuf>,
    add_file_dialog: Option<FileDialog>, //TODO: separate thread to import traces as we get them, then add/remove traces rather than files
}

impl Default for ExporterApp {
    fn default() -> Self {
        Self {
            files: EguiList::default().is_scrollable(true).is_editable(true),
            add_file_dialog: None,
        }
    }
}

impl ExporterApp {
    pub fn new(storage: Option<&dyn Storage>) -> Self {
        info!("Starting new EA");
        let mut s = Self::default();
        let mut traces = storage
            .and_then(|s| s.get_string("files"))
            .map(|s| {
                if s.is_empty() {
                    vec![]
                } else {
                    s.split("---,---").map(ToString::to_string).collect() //"".split(/* anything */) returns vec![""], which we don't want, so we clear the vec if we see this
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
        s.files.append(&mut traces);
        s
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
                        self.files.push(file);
                    }
                }
            }
            if needs_to_close {
                self.add_file_dialog = None;
            }
            ui.separator();

            ui.label("Files to use:");
            self.files.display(ui, |pb, _| format!("{pb:?}"));
        });
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
            .join(",");
        storage.set_string("files", files_to_save);
    }
}
