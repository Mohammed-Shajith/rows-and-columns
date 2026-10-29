//! Big workbooks save on a worker thread (Rows&Columns G07): the snapshot becomes the saved
//! version, edits made meanwhile stay unsaved, scripted saves stay immediate, and a quit waits.

use gridcraft_ui_egui::SheetApp;
use gridcraft_ui_egui::control::{Outcome, handle};
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn path(tag: &str) -> String {
    let d = std::env::temp_dir().join(format!("rc-bg-save-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d.join("big.xlsx").to_string_lossy().into_owned()
}

/// 50,000 rows × 4 columns: at the background-save threshold.
fn big() -> Harness {
    let mut h = crate::support::blank_harness(2);
    let rows: Vec<Vec<serde_json::Value>> = (0..50_000).map(|i| vec![json!(i), json!("Pune"), json!("LCV"), json!(i % 97)]).collect();
    h.state_mut().run("range.setValues", json!({"range": "A1", "values": rows})).unwrap();
    h
}

fn wait(h: &mut Harness) {
    for _ in 0..1000 {
        if h.state().saving_in_background().is_none() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.run_steps(1);
    }
    panic!("the background save never finished");
}

#[test]
fn a_big_workbook_saves_in_the_background() {
    let p = path("save");
    let mut h = big();
    h.state_mut().run("file.save", json!({"path": p})).unwrap();
    assert_eq!(h.state().saving_in_background(), Some(p.as_str()));
    wait(&mut h);
    let d = h.state().session.active().unwrap();
    assert!(!d.is_dirty());
    assert_eq!(d.path.as_deref(), Some(p.as_str()));
    assert!(std::fs::metadata(&p).unwrap().len() > 100_000);
}

#[test]
fn an_edit_during_the_save_stays_unsaved() {
    let p = path("edit");
    let mut h = big();
    h.state_mut().run("file.save", json!({"path": p})).unwrap();
    assert!(h.state().saving_in_background().is_some());
    h.state_mut().run("cell.set", json!({"cell": "F1", "input": "typed while saving"})).unwrap();
    wait(&mut h);
    assert!(h.state().session.active().unwrap().is_dirty());
}

#[test]
fn a_scripted_save_is_immediate() {
    let p = path("script");
    let mut h = big();
    let ctx = h.ctx.clone();
    match handle(h.state_mut(), &ctx, "app.save", &json!({"path": p})) {
        Outcome::Done(v) => assert!(v.get("error").is_none(), "{v}"),
        Outcome::Screenshot { .. } => unreachable!(),
    }
    assert!(h.state().saving_in_background().is_none());
    assert!(!h.state().session.active().unwrap().is_dirty(), "on disk when the call returns");
}

#[test]
fn quitting_waits_for_a_background_save() {
    let p = path("quit");
    let mut h = big();
    h.state_mut().run("file.save", json!({"path": p})).unwrap();
    h.input_mut().viewports.entry(egui::ViewportId::ROOT).or_default().events.push(egui::ViewportEvent::Close);
    h.run_steps(1);
    assert!(h.state().saving_in_background().is_some(), "still saving: the quit waits");
    wait(&mut h);
    assert!(!h.state().session.active().unwrap().is_dirty(), "saved before quitting");
}
