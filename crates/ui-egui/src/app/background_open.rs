//! Opening big files without freezing the window (Rows&Columns G07). A file at least
//! `BACKGROUND_OPEN_BYTES` long is read and parsed on a worker thread (`prepare_open`); the window
//! keeps drawing and shows "Opening … — Esc to cancel"; when the worker is done, `file.openPrepared`
//! adopts the workbook like `file.open` would. Smaller files open at once, as before.

use std::sync::mpsc::{Receiver, TryRecvError};

use gridcraft_engine::cmd::file::{PreparedOpen, prepare_open};
use serde_json::json;

use crate::SheetApp;

/// Files at least this big open in the background: below it, opening takes well under a second.
pub(crate) const BACKGROUND_OPEN_BYTES: u64 = 2 * 1024 * 1024;

pub(crate) struct BackgroundOpen {
    path: String,
    /// When it started, in egui seconds.
    started: f64,
    rx: Receiver<Result<PreparedOpen, String>>,
}

impl SheetApp {
    /// The file being opened in the background, if any.
    pub fn opening_in_background(&self) -> Option<&str> {
        self.background_open.as_ref().map(|j| j.path.as_str())
    }

    /// Starts reading `path` on a worker thread when it's big enough to freeze the window.
    /// Returns false when the caller should open it the ordinary way.
    pub(crate) fn open_in_background(&mut self, path: &str) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = path;
            false
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let big = std::fs::metadata(path).is_ok_and(|m| m.len() >= BACKGROUND_OPEN_BYTES);
            let already_open = self.session.documents().iter().any(|d| d.path.as_deref() == Some(path));
            if !big || already_open {
                return false;
            }
            if let Some(busy) = &self.background_open {
                let name = file_name(&busy.path);
                self.message = Some((crate::APP_NAME.into(), format!("Still opening {name}. Wait for it, or press Esc to cancel it first.")));
                return true;
            }
            let (tx, rx) = std::sync::mpsc::channel();
            let p = path.to_string();
            let spawned = std::thread::Builder::new().name("rc-open".into()).spawn(move || {
                let _ = tx.send(prepare_open(&p, None, false).map_err(|e| e.to_string()));
            });
            if spawned.is_err() {
                return false;
            }
            let started = self.ctx_time;
            self.background_open = Some(BackgroundOpen { path: path.to_string(), started, rx });
            true
        }
    }

    /// Adopts a finished background read; Esc cancels a running one. Called every frame.
    pub(crate) fn poll_background_open(&mut self, ctx: &egui::Context) {
        self.ctx_time = ctx.input(|i| i.time);
        let Some(job) = &self.background_open else { return };
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            // The worker finishes on its own; its result is dropped with the receiver.
            let name = file_name(&job.path).to_string();
            self.background_open = None;
            self.message = Some((crate::APP_NAME.into(), format!("Opening {name} was cancelled.")));
            return;
        }
        let outcome = match job.rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
                return;
            }
            Err(TryRecvError::Disconnected) => Err("the file could not be read".to_string()),
        };
        let path = job.path.clone();
        self.background_open = None;
        match outcome {
            Ok(prepared) => {
                let id = self.session.hand_over_open(prepared);
                match self.run("file.openPrepared", json!({"id": id})) {
                    Ok(r) => self.after_open(Some(&path), &r),
                    Err(e) => self.message = Some((crate::APP_NAME.into(), crate::clean_error(&e))),
                }
            }
            // A password-protected workbook: ask for the password; the dialog opens it.
            Err(e) if e.contains(gridcraft_engine::io::PASSWORD_REQUIRED) => self.open_dialog("openPassword", json!({"path": path})),
            Err(e) => self.message = Some((crate::APP_NAME.into(), crate::clean_error(&e))),
        }
        self.after_engine();
    }

    /// "Opening … N s — Esc to cancel", centred over the window while a background open runs.
    pub(crate) fn background_open_overlay(&self, ctx: &egui::Context) {
        let Some(job) = &self.background_open else { return };
        let secs = (ctx.input(|i| i.time) - job.started).max(0.0);
        egui::Area::new(egui::Id::unique("rc_background_open"))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).inner_margin(16.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new().size(22.0));
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(format!("Opening {}…", file_name(&job.path))).strong());
                            ui.label(format!("{secs:.0} s · Esc to cancel"));
                        });
                    });
                });
            });
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}

fn file_name(path: &str) -> &str {
    std::path::Path::new(path).file_name().and_then(std::ffi::OsStr::to_str).unwrap_or(path)
}
