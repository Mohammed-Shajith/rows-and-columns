//! Rows&Columns' own widgets (G05): an Excel-style checkbox, filled green with a white tick when
//! ticked. egui's checkbox can't fill by its ticked state, so this draws its own; it reports the
//! same role, label and state to assistive tech (and tests) as egui's.

use egui::{Color32, Rect, Response, Role, Sense, Stroke, StrokeKind, TextWrapMode, Ui, WidgetInfo, WidgetText, pos2, vec2};

use crate::theme::Tokens;

const BOX: f32 = 16.0;
const RADIUS: f32 = 4.0;

/// The ticked fill: Excel for Mac's checkbox green (measured), and the theme accent in light mode.
fn tick_fill(t: &Tokens) -> Color32 {
    if t.dark { Color32::from_rgb(0x3E, 0x87, 0x45) } else { t.accent }
}

/// A checkbox like Excel's: an outlined box, filled green with a white tick when on.
pub fn checkbox(ui: &mut Ui, checked: &mut bool, text: impl Into<WidgetText>) -> Response {
    tri_checkbox(ui, checked, text, false)
}

/// A checkbox that can also show a mixed state (a white dash on green), for groups such as
/// (Select All) or a year in the filter's date tree. Clicking toggles `checked`.
pub fn tri_checkbox(ui: &mut Ui, checked: &mut bool, text: impl Into<WidgetText>, mixed: bool) -> Response {
    let gap = ui.spacing().icon_spacing.max(6.0);
    let galley = text.into().into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, egui::TextStyle::Body);
    let height = ui.spacing().interact_size.y.max(galley.size().y);
    let width = if galley.size().x > 0.0 { BOX + gap + galley.size().x } else { BOX };
    let (rect, mut response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }
    let enabled = ui.is_enabled();
    let label = galley.text().to_string();
    response.widget_info(|| {
        if mixed { WidgetInfo::labeled(Role::CheckBox, enabled, &label) } else { WidgetInfo::selected(Role::CheckBox, enabled, *checked, &label) }
    });
    if ui.is_rect_visible(rect) {
        let t = Tokens::get(ui.ctx());
        let p = ui.painter();
        let b = Rect::from_center_size(pos2(rect.left() + BOX / 2.0, rect.center().y), vec2(BOX, BOX));
        let dim = |c: Color32| if enabled { c } else { c.gamma_multiply(0.45) };
        let on = *checked || mixed;
        if on {
            let fill = if response.hovered() { tick_fill(&t).gamma_multiply(0.85) } else { tick_fill(&t) };
            p.rect_filled(b, RADIUS, dim(fill));
            let white = Stroke::new(2.0, dim(Color32::WHITE));
            if mixed {
                p.line_segment([pos2(b.left() + 4.0, b.center().y), pos2(b.right() - 4.0, b.center().y)], white);
            } else {
                let at = |x: f32, y: f32| pos2(b.left() + x * BOX, b.top() + y * BOX);
                p.add(egui::Shape::line(vec![at(0.24, 0.53), at(0.43, 0.71), at(0.77, 0.31)], white));
            }
        } else {
            let border = if response.hovered() { tick_fill(&t) } else { t.input_border };
            p.rect(b, RADIUS, t.input_bg, Stroke::new(1.0, dim(border)), StrokeKind::Inside);
        }
        if response.has_focus() {
            p.rect_stroke(b.expand(2.0), RADIUS + 2.0, Stroke::new(1.5, tick_fill(&t)), StrokeKind::Outside);
        }
        let text_pos = pos2(b.right() + gap, rect.center().y - galley.size().y / 2.0);
        p.galley(text_pos, galley, dim(t.text));
    }
    response
}

/// A dialog's main button (OK, Solve) like Excel's: filled green with white text.
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    ui.add(egui::Button::new(egui::RichText::new(format!("   {text}   ")).color(Color32::WHITE)).fill(tick_fill(&t)).corner_radius(5.0))
}
