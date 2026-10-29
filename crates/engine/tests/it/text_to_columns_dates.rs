//! Text to Columns "Column data format" (Rows&Columns G02): Date in a given order re-reads a mixed
//! column of dates, Text keeps digits as written, Skip leaves a column out.

use gridcraft_core::CellRef;
use gridcraft_engine::Session;
use serde_json::json;

fn book(values: serde_json::Value) -> Session {
    let mut s = Session::new();
    s.new_workbook();
    s.execute("range.setValues", json!({"range": "A1", "values": values})).unwrap();
    s
}

fn shown(s: &Session, a1: &str) -> String {
    let d = s.doc().unwrap();
    let sh = d.wb.active().unwrap();
    gridcraft_engine::display::cell_text(&d.wb, sh, CellRef::parse(a1).unwrap())
}

fn serial(s: &Session, a1: &str) -> f64 {
    s.doc().unwrap().wb.active().unwrap().value(CellRef::parse(a1).unwrap()).as_f64().unwrap()
}

fn ymd_serial(s: &Session, y: i64, m: i64, d: i64) -> f64 {
    gridcraft_core::date::serial_from_ymd(s.doc().unwrap().wb.date_system, y, m, d).unwrap()
}

/// The user's column: day-month-year dates; those with day ≤ 12 were read as US dates on open
/// ("7/8/2024" became 8 July), the rest stayed text ("16-08-2023").
#[test]
fn dmy_fixes_a_column_of_mixed_dates() {
    let mut s = book(json!([["7/8/2024"], ["16-08-2023"], ["11/3/2023"], ["30-03-2022"]]));
    assert_eq!(serial(&s, "A1"), ymd_serial(&s, 2024, 7, 8), "opened as 8 July (month first)");
    s.execute("data.textToColumns", json!({"range": "A1:A4", "format": "DMY"})).unwrap();
    assert_eq!(serial(&s, "A1"), ymd_serial(&s, 2024, 8, 7), "7 August");
    assert_eq!(serial(&s, "A2"), ymd_serial(&s, 2023, 8, 16), "text became a date");
    assert_eq!(serial(&s, "A3"), ymd_serial(&s, 2023, 3, 11));
    assert_eq!(serial(&s, "A4"), ymd_serial(&s, 2022, 3, 30));
    assert_eq!(shown(&s, "A2"), "16/08/2023", "shown day first");
}

#[test]
fn ymd_and_two_digit_years() {
    let mut s = book(json!([["24.08.07"], ["99-12-31"]]));
    s.execute("data.textToColumns", json!({"range": "A1:A2", "format": "YMD"})).unwrap();
    assert_eq!(serial(&s, "A1"), ymd_serial(&s, 2024, 8, 7));
    assert_eq!(serial(&s, "A2"), ymd_serial(&s, 1999, 12, 31));
    assert_eq!(shown(&s, "A1"), "2024-08-07");
}

#[test]
fn a_value_that_is_not_a_date_in_that_order_is_kept() {
    let mut s = book(json!([["16-08-2023"], ["Pune"]]));
    s.execute("data.textToColumns", json!({"range": "A1:A2", "format": "MDY"})).unwrap();
    assert_eq!(shown(&s, "A1"), "16-08-2023", "no month 16: left as it was");
    assert_eq!(shown(&s, "A2"), "Pune");
}

#[test]
fn per_column_formats_text_and_skip() {
    let mut s = book(json!([["007,Pune,04/05/2025"]]));
    s.execute("data.textToColumns", json!({"range": "A1:A1", "delimiters": [","], "columnFormats": ["text", "skip", "DMY"]})).unwrap();
    assert_eq!(shown(&s, "A1"), "007", "text keeps leading zeros");
    assert_eq!(serial(&s, "B1"), ymd_serial(&s, 2025, 5, 4), "skipped column left out; 4 May");
    assert_eq!(shown(&s, "C1"), "", "nothing written past the kept columns");
}

#[test]
fn unknown_formats_are_refused() {
    let mut s = book(json!([["a,b"]]));
    assert!(s.execute("data.textToColumns", json!({"range": "A1", "format": "DDD"})).is_err());
    assert!(s.execute("data.textToColumns", json!({"range": "A1", "columnFormats": ["general", 3]})).is_err());
}
