//! Saving in three steps (Rows&Columns G07): a snapshot, a job on another thread, then marking the
//! snapshot saved. Same file as `file.save`; edits during the job keep the document dirty.

use gridcraft_engine::Session;
use gridcraft_engine::cmd::file::{finish_save, prepare_save};
use serde_json::json;

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rc-save-prepared-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn book() -> Session {
    let mut s = Session::new();
    s.new_workbook();
    s.execute("range.setValues", json!({"range": "A1", "values": [["CITY", "COUNT"], ["Pune", 3], ["Delhi", 5]]})).unwrap();
    s
}

#[test]
fn a_background_save_writes_the_same_workbook_and_clears_dirty() {
    let d = dir("same");
    let (bg, direct) = (d.join("bg.xlsx"), d.join("direct.xlsx"));
    let mut s = book();
    assert!(s.doc().unwrap().is_dirty());
    let prepared = prepare_save(&mut s, Some(bg.to_str().unwrap())).unwrap().expect("an xlsx save splits");
    let job = prepared.job();
    let bytes = std::thread::spawn(move || job.run()).join().unwrap().unwrap();
    let r = finish_save(&mut s, prepared, bytes);
    assert_eq!(r["bytes"], json!(bytes));
    assert!(!s.doc().unwrap().is_dirty(), "the snapshot is the saved version");
    assert_eq!(s.doc().unwrap().path.as_deref(), bg.to_str());

    let mut other = book();
    other.execute("file.save", json!({"path": direct.to_str().unwrap()})).unwrap();
    let (a, b) = (Session::new(), Session::new());
    let (mut a, mut b) = (a, b);
    a.execute("file.open", json!({"path": bg.to_str().unwrap()})).unwrap();
    b.execute("file.open", json!({"path": direct.to_str().unwrap()})).unwrap();
    let cell = |s: &Session, a1: &str| s.doc().unwrap().wb.active().unwrap().value(gridcraft_core::CellRef::parse(a1).unwrap());
    for c in ["A1", "B2", "A3", "B3"] {
        assert_eq!(cell(&a, c), cell(&b, c), "{c}");
    }
}

#[test]
fn edits_made_during_the_save_keep_the_document_dirty() {
    let d = dir("dirty");
    let mut s = book();
    let prepared = prepare_save(&mut s, Some(d.join("book.xlsx").to_str().unwrap())).unwrap().unwrap();
    let job = prepared.job();
    s.execute("cell.set", json!({"cell": "C1", "input": "typed while saving"})).unwrap();
    let bytes = job.run().unwrap();
    finish_save(&mut s, prepared, bytes);
    assert!(s.doc().unwrap().is_dirty(), "the edit isn't in the file");
}

#[test]
fn closing_the_document_during_the_save_is_harmless() {
    let d = dir("closed");
    let mut s = book();
    let prepared = prepare_save(&mut s, Some(d.join("book.xlsx").to_str().unwrap())).unwrap().unwrap();
    let job = prepared.job();
    s.execute("file.close", json!({"force": true})).unwrap();
    let bytes = job.run().unwrap();
    let r = finish_save(&mut s, prepared, bytes);
    assert!(r["bytes"].as_u64().unwrap() > 0);
}

#[test]
fn saves_that_cannot_split_say_so() {
    let mut s = book();
    assert!(prepare_save(&mut s, None).unwrap().is_none(), "no file yet: Save As asks");
    let d = dir("pdf");
    assert!(prepare_save(&mut s, Some(d.join("p.pdf").to_str().unwrap())).unwrap().is_none(), "a PDF is a print");
}
