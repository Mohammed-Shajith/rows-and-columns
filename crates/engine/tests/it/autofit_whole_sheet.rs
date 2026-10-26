use gridcraft_engine::Session;
use serde_json::json;

/// Data in B, D and F (C and E empty), a merged title over B1:D1 and a wrapped two-line cell.
fn book() -> Session {
    let mut s = Session::new();
    s.new_workbook();
    s.execute(
        "range.setValues",
        json!({"range":"B1", "values":[
            ["A long merged title that must not widen B", "", "", "", "x"],
            ["Ahmedabad", "", "OUTSTATION", "", "1234567.891"],
            ["Hyderabad and Secunderabad", "", "2W", "", "7"],
            ["", "", "Micro LCV", "", "=F2*2"],
        ]}),
    )
    .unwrap();
    s.execute("home.mergeCells", json!({"range":"B1:D1"})).unwrap();
    s
}

fn widths(s: &Session) -> Vec<f32> {
    let sh = s.doc().unwrap().wb.active().unwrap();
    (0..8).map(|c| sh.col_width(c)).collect()
}

#[test]
fn whole_sheet_autofit_matches_per_column_autofit() {
    let mut per_column = book();
    for col in ["B:B", "D:D", "F:F"] {
        per_column.execute("home.autofitColumnWidth", json!({"cols": col})).unwrap();
    }
    let mut whole = book();
    whole.execute("home.autofitColumnWidth", json!({"cols":"A:XFD"})).unwrap();
    assert_eq!(widths(&whole), widths(&per_column));
}

#[test]
fn whole_sheet_autofit_leaves_empty_columns_alone() {
    let mut s = book();
    let before = widths(&s);
    s.execute("home.autofitColumnWidth", json!({"cols":"A:XFD"})).unwrap();
    let after = widths(&s);
    let sh = s.doc().unwrap().wb.active().unwrap();
    for c in [0usize, 2, 4, 6, 7] {
        assert_eq!(after[c], before[c], "empty column {c} changed");
        assert!(!sh.cols.contains_key(&(c as u32)), "empty column {c} got an entry");
    }
    assert!(after[1] > before[1], "B was not fitted");
    assert!(after[5] > before[5], "F was not fitted");
}

#[test]
fn autofit_of_areas_outside_the_data_is_a_no_op() {
    let mut s = book();
    let before = widths(&s);
    s.execute("home.autofitColumnWidth", json!({"cols":"H:XFD"})).unwrap();
    s.execute("home.autofitColumnWidth", json!({"cols":"A:A,C:C,E:E"})).unwrap();
    assert_eq!(widths(&s), before);
}

#[test]
fn autofit_on_an_empty_sheet_is_a_no_op() {
    let mut s = Session::new();
    s.new_workbook();
    s.execute("home.autofitColumnWidth", json!({"cols":"A:XFD"})).unwrap();
    assert!(s.doc().unwrap().wb.active().unwrap().cols.is_empty());
}

/// Wide, tall data: a whole-sheet AutoFit visits each stored cell once, so this finishes in a
/// moment even in a debug build (it used to scan every row for each of 16,384 columns).
#[test]
#[allow(clippy::disallowed_methods)] // native-only test: times the command
fn whole_sheet_autofit_on_many_rows_finishes_quickly() {
    let mut s = Session::new();
    s.new_workbook();
    let rows: Vec<Vec<String>> = (0..20_000).map(|r| vec![format!("City {}", r % 97), format!("{}", r * 7), "facebook".into()]).collect();
    s.execute("range.setValues", json!({"range":"A1", "values": rows})).unwrap();
    let t = std::time::Instant::now();
    s.execute("home.autofitColumnWidth", json!({"cols":"A:XFD"})).unwrap();
    assert!(t.elapsed() < std::time::Duration::from_secs(20), "took {:?}", t.elapsed());
    let sh = s.doc().unwrap().wb.active().unwrap();
    assert_eq!(sh.cols.len(), 3);
}
