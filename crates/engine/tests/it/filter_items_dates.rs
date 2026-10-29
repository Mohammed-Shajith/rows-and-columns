//! `filter_items`: the AutoFilter dropdown's values with their dates (Rows&Columns G12).

use gridcraft_engine::Session;
use gridcraft_engine::cmd::data::filter_items;
use serde_json::json;

fn items(values: serde_json::Value) -> (Vec<gridcraft_engine::cmd::data::FilterItem>, bool) {
    let rows = values.as_array().map_or(0, Vec::len);
    let mut s = Session::new();
    s.new_workbook();
    s.execute("range.setValues", json!({"range": "A1", "values": values})).unwrap();
    s.execute("data.filter", json!({"range": format!("A1:A{rows}")})).unwrap();
    let wb = &s.doc().unwrap().wb;
    filter_items(wb, wb.active_sheet(), 0)
}

#[test]
fn a_date_column_is_recognised_with_its_years_months_and_days() {
    let (it, dates) = items(json!([["Day"], ["2025-03-20"], ["2025-03-04"], [""], ["2026-01-02"]]));
    assert!(dates, "every non-blank value is a date");
    let ymd: Vec<_> = it.iter().filter_map(|i| i.date).collect();
    assert_eq!(ymd, [(2025, 3, 4), (2025, 3, 20), (2026, 1, 2)], "sorted, with their parts");
    assert!(it.iter().any(|i| i.text == "(Blanks)" && i.date.is_none()));
    assert!(it.iter().filter(|i| i.date.is_some()).all(|i| i.number && i.on));
}

#[test]
fn numbers_and_text_are_not_date_columns() {
    let (it, dates) = items(json!([["Amount"], [5], [20]]));
    assert!(!dates);
    assert!(it.iter().all(|i| i.number && i.date.is_none()));
    let (_, dates) = items(json!([["Mixed"], ["2025-03-04"], ["Pune"]]));
    assert!(!dates, "one text value makes it a text column");
    let (it, dates) = items(json!([["Empty"], [""]]));
    assert!(!dates && it.len() == 1, "only (Blanks) is not a date column");
}

#[test]
fn ticks_follow_the_current_values_filter() {
    let mut s = Session::new();
    s.new_workbook();
    s.execute("range.setValues", json!({"range": "A1", "values": [["Fruit"], ["Apple"], ["Banana"]]})).unwrap();
    s.execute("data.filter", json!({"range": "A1:A3"})).unwrap();
    s.execute("data.filterBy", json!({"column": "A", "values": ["Banana"]})).unwrap();
    let wb = &s.doc().unwrap().wb;
    let (it, _) = filter_items(wb, wb.active_sheet(), 0);
    let on: Vec<_> = it.iter().map(|i| (i.text.as_str(), i.on)).collect();
    assert_eq!(on, [("Apple", false), ("Banana", true)]);
}
