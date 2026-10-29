//! Rows&Columns' Excel-style AutoFilter dropdown (flaws G06, G12): the column's name as title,
//! Auto Apply, "Choose One" conditions and a date column's Year › Month › Day tree.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers};
use egui_kittest::kittest::Queryable;
use gridcraft_engine::Session;
use gridcraft_ui_egui::SheetApp;
use serde_json::{Value as Json, json};

type Harness = egui_kittest::Harness<'static, SheetApp>;

/// `values` in column A from A1 (a heading first) under an AutoFilter, menu opened with Alt+Down.
fn open(values: Json) -> Harness {
    let rows = values.as_array().map_or(0, Vec::len);
    let mut session = Session::new();
    session.new_workbook();
    session.execute("range.setValues", json!({"range": "A1", "values": values})).unwrap();
    session.execute("data.filter", json!({"range": format!("A1:A{rows}")})).unwrap();
    session.execute("selection.set", json!({"cell": "A1"})).unwrap();
    let mut h = crate::common::builder().with_size(egui::vec2(1200.0, 900.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        crate::common::app(session),
    );
    h.run_steps(4);
    key(&mut h, Key::ArrowDown, Modifiers::ALT);
    assert!(h.state().grid.filter_menu.is_some(), "Alt+Down opens the filter menu");
    h
}

fn key(h: &mut Harness, k: Key, m: Modifiers) {
    h.input_mut().events.push(Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    h.input_mut().events.push(Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
    h.run_steps(2);
}

fn click(h: &mut Harness, label: &str) {
    h.get_by_label(label).click();
    h.run_steps(2);
}

/// Data rows 2..=n+1: which the filter hides.
fn hidden(h: &Harness, n: u32) -> Vec<bool> {
    let sh = h.state().session.active().unwrap().wb.active().unwrap();
    (1..=n).map(|r| sh.is_row_hidden(r)).collect()
}

#[test]
fn title_is_the_column_name_and_auto_apply_filters_at_once() {
    let mut h = open(json!([["Fruit"], ["Apple"], ["Banana"], ["Cherry"]]));
    assert!(h.query_all_by_label("Fruit").count() >= 1, "the menu is titled with the heading");
    assert!(h.query_by_label("Auto Apply").is_some());

    // Auto Apply is on: unticking a value hides its row straight away and the menu stays open.
    click(&mut h, "Banana");
    assert_eq!(hidden(&h, 3), [false, true, false]);
    assert!(h.state().grid.filter_menu.is_some());

    // Off: ticks wait for Apply Filter.
    click(&mut h, "Auto Apply");
    click(&mut h, "Cherry");
    assert_eq!(hidden(&h, 3), [false, true, false], "nothing applied before Apply Filter");
    click(&mut h, "Apply Filter");
    assert_eq!(hidden(&h, 3), [false, true, true]);
    assert!(h.state().grid.filter_menu.is_none(), "Apply Filter closes the menu");
}

#[test]
fn clear_filter_shows_every_row_again() {
    let mut h = open(json!([["Fruit"], ["Apple"], ["Banana"], ["Cherry"]]));
    click(&mut h, "Apple");
    assert_eq!(hidden(&h, 3), [true, false, false]);
    click(&mut h, "Clear Filter");
    assert_eq!(hidden(&h, 3), [false, false, false]);
}

#[test]
fn greater_than_condition_filters_numbers() {
    let mut h = open(json!([["Amount"], [5], [20], [12]]));
    click(&mut h, "Choose One  ▾");
    click(&mut h, "Greater Than");
    // The condition's value box sits right below the condition button (the app has other text boxes).
    let below = h.get_by_label("Greater Than  ▾").rect().bottom();
    let value = h
        .query_all_by_role(Role::TextInput)
        .map(|n| n.rect())
        .filter(|r| r.top() >= below)
        .min_by(|a, b| a.top().total_cmp(&b.top()))
        .expect("a value box")
        .center();
    h.input_mut().events.push(Event::PointerMoved(value));
    h.input_mut().events.push(Event::PointerButton { pos: value, button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.input_mut().events.push(Event::PointerButton { pos: value, button: egui::PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    h.input_mut().events.push(Event::Text("10".into()));
    h.run_steps(2);
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(hidden(&h, 3), [true, false, false], "only values above 10 stay");
    assert!(h.state().grid.filter_menu.is_none());
}

#[test]
fn text_columns_offer_text_conditions_and_numbers_offer_top_10() {
    let mut h = open(json!([["Fruit"], ["Apple"], ["Banana"]]));
    click(&mut h, "Choose One  ▾");
    assert!(h.query_by_label("Contains").is_some());
    assert!(h.query_by_label("Top 10").is_none());
    let mut h = open(json!([["Amount"], [1], [2]]));
    click(&mut h, "Choose One  ▾");
    assert!(h.query_by_label("Top 10").is_some());
    assert!(h.query_by_label("Contains").is_none());
}

#[test]
fn a_date_column_lists_years_months_and_days() {
    let mut h = open(json!([["Day"], ["2025-03-04"], ["2025-03-20"], ["2026-01-02"]]));
    // Years, newest first, collapsed.
    assert!(h.query_by_label("2026").is_some());
    assert!(h.query_by_label("2025").is_some());
    assert!(h.query_by_label("March").is_none(), "years start collapsed");
    assert!(h.query_by_label("2025-03-04").is_none(), "no flat list of dates");

    click(&mut h, "Expand 2025");
    assert!(h.query_by_label("March").is_some());
    click(&mut h, "Expand March");
    assert!(h.query_by_label("04").is_some() && h.query_by_label("20").is_some());

    // Unticking one day (Auto Apply) hides that row only.
    click(&mut h, "20");
    assert_eq!(hidden(&h, 3), [false, true, false]);
    // Unticking a whole year hides every day in it.
    click(&mut h, "2026");
    assert_eq!(hidden(&h, 3), [false, true, true]);
}

/// `RC_FILTER_SCREENSHOT=dir cargo test … filter_popup_screenshot` saves the dropdown in dark mode
/// (a date column, expanded to days; then a text column) for a visual check. A no-op otherwise.
#[test]
fn filter_popup_screenshot() {
    let Ok(dir) = std::env::var("RC_FILTER_SCREENSHOT") else { return };
    let mut h = open(json!([["ORDER_DATE"], ["2025-03-04"], ["2025-03-20"], ["2025-07-11"], ["2026-01-02"], [""]]));
    h.state_mut().ui.dark = true;
    h.state_mut().ui.system_theme = false;
    h.run_steps(30);
    click(&mut h, "Expand 2025");
    click(&mut h, "Expand March");
    h.run_steps(30);
    h.render().unwrap().save(std::path::Path::new(&dir).join("filter-dates.png")).unwrap();
    click(&mut h, "Choose One  ▾");
    h.run_steps(30);
    h.render().unwrap().save(std::path::Path::new(&dir).join("filter-conditions.png")).unwrap();
}
