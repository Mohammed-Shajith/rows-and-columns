//! Opening in two halves (Rows&Columns G07): `prepare_open` reads and parses on any thread,
//! `file.openPrepared` adopts the result, with the same outcome as `file.open`.

use gridcraft_core::CellRef;
use gridcraft_engine::Session;
use gridcraft_engine::cmd::file::prepare_open;
use serde_json::json;

fn csv(name: &str, body: &str) -> String {
    let dir = std::env::temp_dir().join(format!("rc-open-prepared-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("orders.csv");
    std::fs::write(&path, body).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn a_file_prepared_on_another_thread_opens_like_file_open() {
    let path = csv("thread", "CITY,COUNT\nPune,3\nDelhi,5\n");
    let p = path.clone();
    let prepared = std::thread::spawn(move || prepare_open(&p, None, false)).join().unwrap().unwrap();

    let mut s = Session::new();
    s.new_workbook();
    let id = s.hand_over_open(prepared);
    let r = s.execute("file.openPrepared", json!({"id": id})).unwrap();

    let mut direct = Session::new();
    direct.new_workbook();
    let r2 = direct.execute("file.open", json!({"path": path})).unwrap();
    assert_eq!(r["title"], r2["title"]);
    assert_eq!(r["sheets"], r2["sheets"]);
    let (a, b) = (s.doc().unwrap(), direct.doc().unwrap());
    assert_eq!(a.path, b.path, "same save target");
    assert_eq!(s.documents().len(), 1, "the untouched blank Book1 was replaced");
    for c in ["A1", "A2", "B3"] {
        let c = CellRef::parse(c).unwrap();
        assert_eq!(a.wb.active().unwrap().value(c), b.wb.active().unwrap().value(c));
    }
}

#[test]
fn an_id_opens_once_and_unknown_ids_are_refused() {
    let path = csv("once", "A\n1\n");
    let mut s = Session::new();
    s.new_workbook();
    let id = s.hand_over_open(prepare_open(&path, None, false).unwrap());
    assert!(s.execute("file.openPrepared", json!({"id": id})).is_ok());
    assert!(s.execute("file.openPrepared", json!({"id": id})).is_err(), "already taken");
    assert!(s.execute("file.openPrepared", json!({"id": 999})).is_err());
    assert!(s.execute("file.openPrepared", json!({})).is_err());
}

#[test]
fn prepare_reports_a_missing_file() {
    assert!(prepare_open("/nonexistent/rows-and-columns/none.csv", None, false).is_err());
}
