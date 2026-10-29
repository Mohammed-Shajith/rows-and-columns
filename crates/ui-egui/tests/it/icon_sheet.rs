//! `RC_ICON_SHEET=dir cargo test … icon_sheet` renders the ribbon's main icons at 30 px (big
//! ribbon buttons) and 16 px, in the dark and light themes, for design review (Rows&Columns G05).
//! A no-op otherwise.

use gridcraft_ui_egui::icons::{self, Icon};
use gridcraft_ui_egui::theme::{self, Tokens};

const ICONS: [Icon; 24] = [
    Icon::Paste,
    Icon::CondFormat,
    Icon::FormatTable,
    Icon::CellStyles,
    Icon::Insert,
    Icon::Delete,
    Icon::Format,
    Icon::Sum,
    Icon::SortFilter,
    Icon::Find,
    Icon::Table,
    Icon::PivotTable,
    Icon::Chart,
    Icon::ChartBar,
    Icon::Filter,
    Icon::SortAsc,
    Icon::Duplicates,
    Icon::TextColumns,
    Icon::Validation,
    Icon::Freeze,
    Icon::Picture,
    Icon::Comment,
    Icon::Function,
    Icon::Macro,
];

fn sheet(dark: bool, path: &std::path::Path) {
    let mut h = crate::common::builder::<()>().with_size(egui::vec2(1880.0, 620.0)).build_ui(move |ui| {
        theme::apply(ui.ctx(), dark);
        let t = Tokens::get(ui.ctx());
        ui.painter().rect_filled(ui.max_rect(), 0.0, t.ribbon);
        let (cols, cell) = (12usize, egui::vec2(76.0, 140.0));
        for (i, icon) in ICONS.iter().enumerate() {
            let o = ui.max_rect().min + egui::vec2((i % cols) as f32 * cell.x + 8.0, (i / cols) as f32 * cell.y + 10.0);
            icons::paint(ui.painter(), egui::Rect::from_min_size(o + egui::vec2(18.0, 0.0), egui::vec2(30.0, 30.0)), *icon, t.text);
            icons::paint(ui.painter(), egui::Rect::from_min_size(o + egui::vec2(25.0, 44.0), egui::vec2(16.0, 16.0)), *icon, t.text);
            ui.painter().text(
                o + egui::vec2(33.0, 74.0),
                egui::Align2::CENTER_TOP,
                format!("{icon:?}"),
                egui::FontId::proportional(10.0),
                t.text_dim,
            );
        }
    });
    h.set_pixels_per_point(2.0);
    h.run_steps(3);
    h.render().unwrap().save(path).unwrap();
}

#[test]
fn icon_sheet() {
    let Ok(dir) = std::env::var("RC_ICON_SHEET") else { return };
    let dir = std::path::Path::new(&dir);
    sheet(true, &dir.join("icons-dark.png"));
    sheet(false, &dir.join("icons-light.png"));
}
