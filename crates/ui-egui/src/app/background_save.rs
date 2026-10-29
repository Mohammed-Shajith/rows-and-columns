//! Saving big workbooks without freezing the window (Rows&Columns G07). ⌘S / File › Save on a
//! workbook of `BACKGROUND_SAVE_CELLS` or more takes a snapshot (copy-on-write, instant), writes it
//! on a worker thread and shows "Saving … N s" in the corner; when the write finishes the snapshot
//! becomes the saved version (edits made meanwhile stay unsaved). Saves the quit prompt makes, and
//! scripted saves, stay immediate (`saving_sync`). Quitting while a save runs waits for it.

use std::sync::mpsc::{Receiver, TryRecvError};

use gridcraft_engine::cmd::file::{PreparedSave, finish_save, prepare_save};
use serde_json::{Value as Json, json};

use crate::SheetApp;

/// Workbooks with at least this many cells save in the background (about half a second or more).
pub(crate) const BACKGROUND_SAVE_CELLS: usize = 200_000;

pub(crate) struct BackgroundSave {
    prepared: PreparedSave,
    started: f64,
    rx: Receiver<Result<usize, String>>,
}

impl SheetApp {
    /// The file being saved in the background, if any.
    pub fn saving_in_background(&self) -> Option<&str> {
        self.background_save.as_ref().map(|j| j.prepared.path.as_str())
    }

    /// Starts `file.save` (`params`) in the background when the workbook is big enough; `None`
    /// means the caller saves the ordinary way.
    pub(crate) fn try_background_save(&mut self, params: &Json) -> Option<Result<Json, String>> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = params;
            None
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.saving_sync {
                return None;
            }
            let cells: usize = self.session.active()?.wb.sheets.iter().map(|sh| sh.cells.len()).sum();
            if cells < BACKGROUND_SAVE_CELLS {
                return None;
            }
            if let Some(busy) = &self.background_save {
                let name = file_name(&busy.prepared.path).to_string();
                return Some(Err(format!("Still saving {name}. Try again when it has finished.")));
            }
            let path = params.get("path").and_then(Json::as_str);
            let prepared = match prepare_save(&mut self.session, path) {
                Ok(Some(p)) => p,
                Ok(None) => return None,
                Err(e) => return Some(Err(e.to_string())),
            };
            let (tx, rx) = std::sync::mpsc::channel();
            let job = prepared.job();
            let spawned = std::thread::Builder::new().name("rc-save".into()).spawn(move || {
                let _ = tx.send(job.run().map_err(|e| e.to_string()));
            });
            if spawned.is_err() {
                return None;
            }
            let path = prepared.path.clone();
            self.background_save = Some(BackgroundSave { prepared, started: self.ctx_time, rx });
            Some(Ok(json!({"path": path, "background": true})))
        }
    }

    /// Records a finished background save; quits afterwards if a quit was waiting for it.
    pub(crate) fn poll_background_save(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.background_save else { return };
        let outcome = match job.rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
                return;
            }
            Err(TryRecvError::Disconnected) => Err("the save stopped unexpectedly".to_string()),
        };
        let Some(job) = self.background_save.take() else { return };
        match outcome {
            Ok(bytes) => {
                finish_save(&mut self.session, job.prepared, bytes);
            }
            Err(e) => {
                let name = file_name(&job.prepared.path).to_string();
                self.message = Some((crate::APP_NAME.into(), format!("Couldn't save {name}: {}", crate::clean_error(&e))));
                self.quit_when_saved = false;
            }
        }
        self.after_engine();
        if std::mem::take(&mut self.quit_when_saved) {
            // The close goes through the guard again: anything still unsaved is asked about.
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// "Saving … N s" in the bottom-right corner while a background save runs.
    pub(crate) fn background_save_overlay(&self, ctx: &egui::Context) {
        let Some(job) = &self.background_save else { return };
        let secs = (ctx.input(|i| i.time) - job.started).max(0.0);
        let waiting = if self.quit_when_saved { " · quitting when done" } else { "" };
        egui::Area::new(egui::Id::unique("rc_background_save"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -40.0))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).inner_margin(10.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new().size(16.0));
                        ui.label(format!("Saving {}… {secs:.0} s{waiting}", file_name(&job.prepared.path)));
                    });
                });
            });
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}

fn file_name(path: &str) -> &str {
    std::path::Path::new(path).file_name().and_then(std::ffi::OsStr::to_str).unwrap_or(path)
}
