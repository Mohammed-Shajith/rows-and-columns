//! The pointer over an AutoFilter dropdown is a pointing hand, as in Excel; over the rest of the
//! header cell and over the grid it stays the cell cursor.

use egui::{CursorIcon, Event, vec2};
use gridcraft_engine::Session;
use gridcraft_engine::core::CellRef;
use gridcraft_ui_egui::SheetApp;
use gridcraft_ui_egui::grid::{Geo, filter_button_rect};
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn harness() -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    session.execute("range.setValues", json!({"range": "A1", "values": [["CITY", "TYPE"], ["Pune", "HCV"], ["Surat", "2W"]]})).unwrap();
    session.execute("selection.set", json!({"cell": "A1"})).unwrap();
    session.execute("data.filter", json!({})).unwrap();
    // Park the selection away from the header so its border and fill handle don't overlap the buttons.
    session.execute("selection.set", json!({"cell": "D10"})).unwrap();
    let mut h = crate::common::builder().with_step_dt(1.0 / 60.0).with_size(vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        crate::common::app(session),
    );
    h.run_steps(4);
    h
}

fn geo(h: &Harness) -> Geo {
    h.state().grid.geo.clone().unwrap()
}

fn cursor_at(h: &mut Harness, pos: egui::Pos2) -> CursorIcon {
    h.input_mut().events.push(Event::PointerMoved(pos));
    h.run_steps(2);
    h.output().platform_output.cursor_icon
}

#[test]
fn pointer_over_a_filter_button_is_a_pointing_hand() {
    let mut h = harness();
    let g = geo(&h);
    let (button_a, button_b, header_text, body) = {
        let sh = h.state().session.active().unwrap().wb.active().unwrap();
        (
            filter_button_rect(&g, sh, CellRef::new(0, 0)),
            filter_button_rect(&g, sh, CellRef::new(0, 1)),
            g.screen_cell_rect(sh, CellRef::new(0, 0)).left_center() + vec2(6.0, 0.0),
            g.screen_cell_rect(sh, CellRef::new(1, 1)).center(),
        )
    };
    assert_eq!(cursor_at(&mut h, button_a.center()), CursorIcon::PointingHand);
    assert_eq!(cursor_at(&mut h, header_text), CursorIcon::Cell, "the heading text is not the button");
    assert_eq!(cursor_at(&mut h, button_b.center()), CursorIcon::PointingHand);
    assert_eq!(cursor_at(&mut h, body), CursorIcon::Cell);
}
