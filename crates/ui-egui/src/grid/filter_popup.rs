//! Rows&Columns' AutoFilter dropdown, laid out like Excel's: the column's name, sort, a condition
//! ("Choose One"), search, the values (a date column as Year › Month › Day), Auto Apply, Apply Filter
//! and Clear Filter (flaws G06, G12).
//!
//! The values are read once when the dropdown opens (a big column takes a moment to list); the
//! list is drawn a screenful at a time, so 300,000 distinct values scroll smoothly.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use egui::{Align, Button, Layout, RichText, ScrollArea, Sense, TextEdit, vec2};
use gridcraft_engine::cmd::data::{FilterItem, filter_items};
use gridcraft_engine::core::date::DateSystem;
use gridcraft_engine::core::{CellRef, Value, col_to_letters};
use serde_json::{Value as Json, json};

use super::editor::popup_dismissed;
use crate::SheetApp;

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const LIST_HEIGHT: f32 = 260.0;
const BLANKS: &str = "(Blanks)";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Kind {
    #[default]
    Text,
    Number,
    Date,
}

/// One entry of the "Choose One" list: label, `data.filterBy` operator and how many values it takes.
/// Special operators: `between`, `top10`, `above`, `below`.
struct Cond {
    label: &'static str,
    op: &'static str,
    values: u8,
}

const fn c(label: &'static str, op: &'static str, values: u8) -> Cond {
    Cond { label, op, values }
}

const TEXT_CONDS: [Cond; 7] = [
    c("Choose One", "", 0),
    c("Equals", "=", 1),
    c("Does Not Equal", "<>", 1),
    c("Begins With", "beginsWith", 1),
    c("Ends With", "endsWith", 1),
    c("Contains", "contains", 1),
    c("Does Not Contain", "notContains", 1),
];
const NUMBER_CONDS: [Cond; 11] = [
    c("Choose One", "", 0),
    c("Equals", "=", 1),
    c("Does Not Equal", "<>", 1),
    c("Greater Than", ">", 1),
    c("Greater Than or Equal To", ">=", 1),
    c("Less Than", "<", 1),
    c("Less Than or Equal To", "<=", 1),
    c("Between", "between", 2),
    c("Top 10", "top10", 1),
    c("Above Average", "above", 0),
    c("Below Average", "below", 0),
];
const DATE_CONDS: [Cond; 7] = [
    c("Choose One", "", 0),
    c("Equals", "=", 1),
    c("Before", "<", 1),
    c("On or Before", "<=", 1),
    c("After", ">", 1),
    c("On or After", ">=", 1),
    c("Between", "between", 2),
];

fn conditions(kind: Kind) -> &'static [Cond] {
    match kind {
        Kind::Text => &TEXT_CONDS,
        Kind::Number => &NUMBER_CONDS,
        Kind::Date => &DATE_CONDS,
    }
}

/// The dropdown's state while it is open: taken out of egui's memory each frame and put back.
#[derive(Clone, Default)]
struct State {
    items: Arc<Vec<FilterItem>>,
    /// Lower-cased item texts, for search.
    lower: Arc<Vec<String>>,
    kind: Kind,
    date_system: DateSystem,
    title: String,
    checks: Vec<bool>,
    search: String,
    /// Items matching `visible_for` (all of them when it's empty).
    visible: Vec<usize>,
    visible_mask: Vec<bool>,
    visible_for: Option<String>,
    /// Expanded date groups: `(year, 0)` for a year, `(year, month)` for a month.
    expanded: HashSet<(i32, u32)>,
    cond: usize,
    cond_open: bool,
    value: String,
    value2: String,
    /// The column has a filter now.
    filtered: bool,
}

impl State {
    fn open(app: &SheetApp, col: u32) -> Option<State> {
        let d = app.session.active()?;
        let wb = &d.wb;
        let si = wb.active_sheet();
        let sh = wb.sheet(si)?;
        let af = sh.autofilter.as_ref()?;
        let (items, dates) = filter_items(wb, si, col);
        let numbers = items.iter().filter(|i| i.text != BLANKS).all(|i| i.number) && items.iter().any(|i| i.text != BLANKS);
        let kind = if dates {
            Kind::Date
        } else if numbers {
            Kind::Number
        } else {
            Kind::Text
        };
        let heading = gridcraft_engine::display::cell_text(wb, sh, CellRef::new(af.range.start.row, col));
        let title = if heading.trim().is_empty() { format!("Column {}", col_to_letters(col)) } else { heading };
        let off = col.saturating_sub(af.range.start.col);
        let filtered = af.criteria.iter().any(|(o, _)| *o == off);
        let checks = items.iter().map(|i| i.on).collect();
        let lower = items.iter().map(|i| i.text.to_lowercase()).collect();
        Some(State { items: Arc::new(items), lower: Arc::new(lower), kind, date_system: wb.date_system, title, checks, filtered, ..State::default() })
    }

    fn refresh_visible(&mut self) {
        let needle = self.search.trim().to_lowercase();
        if self.visible_for.as_deref() == Some(needle.as_str()) {
            return;
        }
        self.visible = self.lower.iter().enumerate().filter(|(_, t)| needle.is_empty() || t.contains(&needle)).map(|(i, _)| i).collect();
        self.visible_mask = vec![false; self.items.len()];
        for &i in &self.visible {
            if let Some(m) = self.visible_mask.get_mut(i) {
                *m = true;
            }
        }
        self.visible_for = Some(needle);
    }

    fn searching(&self) -> bool {
        self.visible_for.as_deref().is_some_and(|n| !n.is_empty())
    }

    fn checked(&self, i: usize) -> bool {
        self.checks.get(i).copied().unwrap_or(false)
    }

    fn set(&mut self, idx: &[usize], on: bool) {
        for &i in idx {
            if let Some(c) = self.checks.get_mut(i) {
                *c = on;
            }
        }
    }

    /// `data.filterBy` params for the ticked values (only search matches count while searching).
    fn values_params(&self, col: u32) -> Json {
        let searching = self.searching();
        let shown = |i: usize| self.checked(i) && (!searching || self.visible_mask.get(i).copied().unwrap_or(false));
        let column = col_to_letters(col);
        if (0..self.items.len()).all(shown) {
            return json!({"column": column, "clear": true});
        }
        let mut values = Vec::new();
        let mut blanks = false;
        for (i, item) in self.items.iter().enumerate() {
            if shown(i) {
                if item.text == BLANKS {
                    blanks = true;
                } else {
                    values.push(item.text.clone());
                }
            }
        }
        json!({"column": column, "values": values, "blanks": blanks})
    }

    /// A typed comparison value; a date column takes a date ("1/5/2026") and compares serials.
    fn criterion(&self, text: &str) -> Option<String> {
        let t = text.trim();
        if t.is_empty() {
            return None;
        }
        if self.kind == Kind::Date {
            return match gridcraft_engine::core::parse::parse_input(t, self.date_system).value {
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            };
        }
        Some(t.to_string())
    }

    /// `data.filterBy` params for the chosen condition, or `None` until it's complete.
    fn condition_params(&self, col: u32) -> Option<Json> {
        let cond = conditions(self.kind).get(self.cond)?;
        let column = col_to_letters(col);
        match cond.op {
            "" => None,
            "above" => Some(json!({"column": column, "aboveAverage": true})),
            "below" => Some(json!({"column": column, "aboveAverage": false})),
            "top10" => {
                let count = self.value.trim().parse::<u32>().unwrap_or(10).clamp(1, 500);
                Some(json!({"column": column, "top": {"count": count}}))
            }
            "between" => {
                let (a, b) = (self.criterion(&self.value)?, self.criterion(&self.value2)?);
                Some(json!({"column": column, "custom": {"op": ">=", "value": a, "op2": "<=", "value2": b, "and": true}}))
            }
            op => Some(json!({"column": column, "custom": {"op": op, "value": self.criterion(&self.value)?}})),
        }
    }

    /// Something is ticked (and, while searching, matches the search).
    fn any_shown(&self) -> bool {
        let searching = self.searching();
        (0..self.items.len()).any(|i| self.checked(i) && (!searching || self.visible_mask.get(i).copied().unwrap_or(false)))
    }

    fn reset_checks(&mut self) {
        self.checks.iter_mut().for_each(|c| *c = true);
    }
}

/// A tri-state checkbox for a group (Select All, a year, a month). Returns the new state on click.
fn group_box(ui: &mut egui::Ui, on: usize, total: usize, label: impl Into<egui::WidgetText>) -> Option<bool> {
    let all = total > 0 && on == total;
    let mut v = all;
    let r = crate::rc_widgets::tri_checkbox(ui, &mut v, label, on > 0 && on < total);
    r.clicked().then_some(!all)
}

/// An expand/collapse arrow for group `name` ("Expand 2025" to assistive tech); true when clicked.
fn arrow(ui: &mut egui::Ui, open: bool, name: &str) -> bool {
    let (_, r) = ui.allocate_exact_size(vec2(14.0, 18.0), Sense::click());
    let label = format!("{} {name}", if open { "Collapse" } else { "Expand" });
    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label.clone()));
    // A small grey chevron, as in Excel: › closed, ⌄ open.
    let t = crate::theme::Tokens::get(ui.ctx());
    let c = r.rect.center();
    let color = if r.hovered() { t.text } else { t.text_dim };
    let pts = if open {
        vec![egui::pos2(c.x - 3.5, c.y - 1.5), egui::pos2(c.x, c.y + 2.0), egui::pos2(c.x + 3.5, c.y - 1.5)]
    } else {
        vec![egui::pos2(c.x - 1.5, c.y - 3.5), egui::pos2(c.x + 2.0, c.y), egui::pos2(c.x - 1.5, c.y + 3.5)]
    };
    ui.painter().add(egui::Shape::line(pts, egui::Stroke::new(1.5, color)));
    r.clicked()
}

/// The flat list (and the search results of a date column). Returns true when a tick changed.
fn flat_list(ui: &mut egui::Ui, st: &mut State) -> bool {
    let mut changed = false;
    let row_h = ui.spacing().interact_size.y;
    let visible = std::mem::take(&mut st.visible);
    ScrollArea::vertical().id_salt("rc_filter_values").max_height(LIST_HEIGHT).auto_shrink([false, true]).show_rows(
        ui,
        row_h,
        visible.len(),
        |ui, range| {
            for k in range {
                let Some(&i) = visible.get(k) else { continue };
                let label = st.items.get(i).map(|it| it.text.clone()).unwrap_or_default();
                if let Some(c) = st.checks.get_mut(i)
                    && crate::rc_widgets::checkbox(ui, c, label).changed()
                {
                    changed = true;
                }
            }
        },
    );
    st.visible = visible;
    changed
}

/// A date column as Year › Month › Day (newest year first), then (Blanks). Returns true when a tick changed.
fn date_tree(ui: &mut egui::Ui, st: &mut State) -> bool {
    let mut years: BTreeMap<i32, BTreeMap<u32, Vec<(u32, usize)>>> = BTreeMap::new();
    let mut others = Vec::new();
    for (i, item) in st.items.iter().enumerate() {
        match item.date {
            Some((y, m, d)) => years.entry(y).or_default().entry(m).or_default().push((d, i)),
            None => others.push(i),
        }
    }
    let mut changed = false;
    ScrollArea::vertical().id_salt("rc_filter_dates").max_height(LIST_HEIGHT).auto_shrink([false, true]).show(ui, |ui| {
        for (y, months) in years.iter().rev() {
            let year_idx: Vec<usize> = months.values().flatten().map(|(_, i)| *i).collect();
            let on = year_idx.iter().filter(|&&i| st.checked(i)).count();
            let open = st.expanded.contains(&(*y, 0));
            ui.horizontal(|ui| {
                if arrow(ui, open, &y.to_string()) {
                    if open {
                        st.expanded.remove(&(*y, 0));
                    } else {
                        st.expanded.insert((*y, 0));
                    }
                }
                if let Some(v) = group_box(ui, on, year_idx.len(), y.to_string()) {
                    st.set(&year_idx, v);
                    changed = true;
                }
            });
            if !open {
                continue;
            }
            for (m, days) in months {
                let month_idx: Vec<usize> = days.iter().map(|(_, i)| *i).collect();
                let on = month_idx.iter().filter(|&&i| st.checked(i)).count();
                let mopen = st.expanded.contains(&(*y, *m));
                let name = MONTHS.get((*m as usize).saturating_sub(1)).copied().unwrap_or("?");
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
                    if arrow(ui, mopen, name) {
                        if mopen {
                            st.expanded.remove(&(*y, *m));
                        } else {
                            st.expanded.insert((*y, *m));
                        }
                    }
                    if let Some(v) = group_box(ui, on, month_idx.len(), name) {
                        st.set(&month_idx, v);
                        changed = true;
                    }
                });
                if !mopen {
                    continue;
                }
                for (d, i) in days {
                    ui.horizontal(|ui| {
                        ui.add_space(50.0);
                        if let Some(c) = st.checks.get_mut(*i)
                            && crate::rc_widgets::checkbox(ui, c, format!("{d:02}")).changed()
                        {
                            changed = true;
                        }
                    });
                }
            }
        }
        for i in others {
            let label = st.items.get(i).map(|it| it.text.clone()).unwrap_or_default();
            if let Some(c) = st.checks.get_mut(i)
                && crate::rc_widgets::checkbox(ui, c, label).changed()
            {
                changed = true;
            }
        }
    });
    changed
}

/// Draws the dropdown for column `col`, anchored at `at`, and applies what the user picks.
pub(super) fn show(app: &mut SheetApp, ui: &mut egui::Ui, col: u32, at: egui::Pos2, was_open: bool) {
    let id = egui::Id::unique(("rc_filter_popup", col));
    let auto_id = egui::Id::unique("rc_filter_auto_apply");
    let kept = ui.ctx().data_mut(|d| d.remove_temp::<State>(id));
    let mut st = match kept {
        Some(s) if was_open && !s.items.is_empty() => s,
        _ => match State::open(app, col) {
            Some(s) => s,
            None => {
                app.grid.filter_menu = None;
                return;
            }
        },
    };
    st.refresh_visible();
    let mut auto = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(auto_id, || true));
    // Enter is the menu's OK, as in Excel: Apply Filter.
    let enter = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));

    let mut close = false;
    let mut sort: Option<&'static str> = None;
    let mut apply_values = false;
    let mut apply_cond = false;
    let mut clear = false;
    let area = egui::Area::new(egui::Id::unique("rc_filter_menu")).fixed_pos(at + vec2(-180.0, 10.0)).order(egui::Order::Foreground);
    let response = area.show(ui.ctx(), |ui| {
        egui::Frame::popup(ui.style()).inner_margin(12.0).show(ui, |ui| {
            ui.set_width(300.0);
            ui.vertical_centered(|ui| ui.label(RichText::new(&st.title).strong()));
            ui.add_space(4.0);
            ui.label(RichText::new("Sort").strong());
            ui.horizontal(|ui| {
                let w = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
                if ui.add_sized([w, 24.0], Button::new("A→Z  Ascending")).clicked() {
                    sort = Some("data.sortAscending");
                }
                if ui.add_sized([w, 24.0], Button::new("Z→A  Descending")).clicked() {
                    sort = Some("data.sortDescending");
                }
            });
            ui.separator();
            ui.label(RichText::new("Filter").strong());

            // "Choose One": the conditions open inline (a separate popup would close this one).
            let conds = conditions(st.kind);
            let label = conds.get(st.cond).map(|c| c.label).unwrap_or("Choose One");
            if ui.add_sized([ui.available_width(), 22.0], Button::new(format!("{label}  ▾"))).clicked() {
                st.cond_open = !st.cond_open;
            }
            if st.cond_open {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    for (i, c) in conds.iter().enumerate() {
                        if ui.selectable_label(st.cond == i, c.label).clicked() {
                            st.cond = i;
                            st.cond_open = false;
                        }
                    }
                });
            }
            let need = conds.get(st.cond).map(|c| c.values).unwrap_or(0);
            if need >= 1 {
                let hint = match (st.kind, conds.get(st.cond).map(|c| c.op)) {
                    (_, Some("top10")) => "10",
                    (Kind::Date, _) => "a date, e.g. 1/5/2026",
                    _ => "value",
                };
                ui.add(TextEdit::singleline(&mut st.value).hint_text(hint).desired_width(f32::INFINITY));
            }
            if need >= 2 {
                ui.horizontal(|ui| {
                    ui.label("and");
                    ui.add(TextEdit::singleline(&mut st.value2).desired_width(f32::INFINITY));
                });
            }
            ui.add_space(2.0);

            let field = ui.add(TextEdit::singleline(&mut st.search).hint_text("Search").desired_width(f32::INFINITY));
            if !was_open {
                field.request_focus();
            }
            st.refresh_visible();

            // (Select All) covers what's listed: everything, or the search results.
            let visible = std::mem::take(&mut st.visible);
            let on = visible.iter().filter(|&&i| st.checked(i)).count();
            let all_label = if st.searching() { "(Select All Search Results)" } else { "(Select All)" };
            let mut changed = false;
            if let Some(v) = group_box(ui, on, visible.len(), all_label) {
                st.set(&visible, v);
                changed = true;
            }
            st.visible = visible;
            changed |= if st.kind == Kind::Date && !st.searching() { date_tree(ui, &mut st) } else { flat_list(ui, &mut st) };
            if changed && auto {
                apply_values = true;
            }

            ui.add_space(4.0);
            crate::rc_widgets::checkbox(ui, &mut auto, "Auto Apply");
            ui.separator();
            let cond_ready = st.condition_params(col).is_some();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let filtered = st.filtered || st.checks.iter().any(|c| !c);
                if ui.add_enabled(filtered, Button::new("Clear Filter")).clicked() {
                    clear = true;
                }
                if ui.add_enabled(!auto || cond_ready, Button::new("Apply Filter")).clicked() {
                    if cond_ready {
                        apply_cond = true;
                    } else {
                        apply_values = true;
                    }
                    close = true;
                }
            });
        });
    });

    if enter {
        if st.condition_params(col).is_some() {
            apply_cond = true;
            close = true;
        } else if st.any_shown() {
            apply_values = true;
            close = true;
        }
    }
    if let Some(cmd) = sort {
        let _ = app.run(cmd, json!({"column": col_to_letters(col)}));
        close = true;
    }
    if clear {
        let _ = app.run("data.filterBy", json!({"column": col_to_letters(col), "clear": true}));
        st.reset_checks();
        st.cond = 0;
        st.value.clear();
        st.value2.clear();
        st.filtered = false;
    } else if apply_cond {
        if let Some(p) = st.condition_params(col) {
            let _ = app.run("data.filterBy", p);
            st.reset_checks();
            st.filtered = true;
        }
    } else if apply_values {
        let p = st.values_params(col);
        st.filtered = p.get("clear").is_none();
        let _ = app.run("data.filterBy", p);
    }

    ui.ctx().data_mut(|d| {
        d.insert_temp(auto_id, auto);
        d.insert_temp(egui::Id::unique(("filter_search", col)), st.search.clone());
    });
    if close || popup_dismissed(&response.response, was_open) {
        app.grid.filter_menu = None;
    } else {
        ui.ctx().data_mut(|d| d.insert_temp(id, st));
    }
}
