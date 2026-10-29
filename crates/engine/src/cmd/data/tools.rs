use super::{
    Cell, CellRef, EngineError, Json, RangeRef, Result, Session, bad, bool_param, cell_param_on, col_param, data_target, edit, guess_header, json,
    ok, str_param, target_range, target_sheet,
};

// ---------------------------------------------------------------- tools

pub(super) fn remove_duplicates(s: &mut Session, p: &Json) -> Result<Json> {
    let (sheet, r) = data_target(s, p, "data.removeDuplicates")?;
    let d = s.doc()?;
    let sh = d.wb.sheet(sheet).ok_or(EngineError::NoDocument)?;
    // Rows and columns past the data are blank: only the part with data is compared (whole
    // columns compared a million blank rows and reported them as removed duplicates).
    let r = sh.used_range().and_then(|u| r.intersection(&u)).unwrap_or(RangeRef::cell(r.start));
    let header = bool_param(p, "header").unwrap_or_else(|| guess_header(&d.wb, sh, r));
    let cols: Vec<u32> = p
        .get("columns")
        .and_then(Json::as_array)
        .map(|a| a.iter().filter_map(|c| col_param(Some(c), r.start.col)).collect())
        .unwrap_or_else(|| (r.start.col..=r.end.col).collect());
    // With no column to compare, every row key is empty: the first row would survive and every
    // other row would be deleted as a duplicate. A column outside the range compares data the
    // user didn't select. Excel refuses both.
    if cols.is_empty() || cols.iter().any(|c| !(r.start.col..=r.end.col).contains(c)) {
        return Err(bad("data.removeDuplicates", "`columns` must name at least one column of the range"));
    }
    let start = r.start.row + header as u32;
    let mut removed = 0;
    edit(s, |cx| {
        let Some(sh) = cx.wb.sheet(sheet) else { return Ok(()) };
        let mut seen = std::collections::HashSet::new();
        let mut keep: Vec<u32> = Vec::new();
        // Cells are compared by the text they show, as in Excel: the same date in two formats
        // isn't a duplicate, while 1.01 and 1.02 shown as 1.0 are.
        let shown = |c: CellRef| crate::display::format(&sh.value(c), cx.wb.styles.get(sh.style_id(c)).num_fmt.as_str(), &cx.wb).text;
        for row in start..=r.end.row {
            let key: Vec<String> = cols.iter().map(|c| shown(CellRef::new(row, *c)).to_lowercase()).collect();
            if seen.insert(key) {
                keep.push(row);
            } else {
                removed += 1;
            }
        }
        let pictures: Vec<_> = keep
            .iter()
            .enumerate()
            .flat_map(|(i, row)| {
                (r.start.col..=r.end.col)
                    .filter_map(move |col| sh.cell_pictures.get(&CellRef::new(*row, col)).map(|p| (CellRef::new(start + i as u32, col), p.clone())))
            })
            .collect();
        // Rows a filter keeps are compacted to `start + i`, so the notes and hyperlinks that
        // describe them move with them; the ones on removed rows are dropped with the row.
        let notes: Vec<_> = keep
            .iter()
            .enumerate()
            .flat_map(|(i, row)| {
                (r.start.col..=r.end.col)
                    .filter_map(move |col| sh.comments.get(&CellRef::new(*row, col)).map(|cm| (CellRef::new(start + i as u32, col), cm.clone())))
            })
            .collect();
        let links: Vec<_> = keep
            .iter()
            .enumerate()
            .flat_map(|(i, row)| {
                (r.start.col..=r.end.col)
                    .filter_map(move |col| sh.hyperlinks.get(&CellRef::new(*row, col)).map(|h| (CellRef::new(start + i as u32, col), h.clone())))
            })
            .collect();
        let rows: Vec<Vec<Option<Cell>>> =
            keep.iter().map(|row| (r.start.col..=r.end.col).map(|c| sh.cell(CellRef::new(*row, c)).cloned()).collect()).collect();
        let shm = cx.sheet_mut(sheet)?;
        for row in start..=r.end.row {
            for c in r.start.col..=r.end.col {
                shm.remove_cell(CellRef::new(row, c));
            }
        }
        for row in start..=r.end.row {
            for c in r.start.col..=r.end.col {
                let at = CellRef::new(row, c);
                shm.comments.remove(&at);
                shm.hyperlinks.remove(&at);
            }
        }
        for (i, cells) in rows.into_iter().enumerate() {
            for (j, cell) in cells.into_iter().enumerate() {
                if let Some(cell) = cell {
                    shm.set_cell(CellRef::new(start + i as u32, r.start.col + j as u32), cell);
                }
            }
        }
        shm.cell_pictures.extend(pictures);
        for (at, cm) in notes {
            shm.comments.insert(at, cm);
        }
        for (at, h) in links {
            shm.hyperlinks.insert(at, h);
        }
        for c in RangeRef::new(CellRef::new(start, r.start.col), r.end).iter() {
            cx.changed.push((sheet, c));
        }
        Ok(())
    })?;
    let remaining = (r.end.row + 1 - start) as usize - removed;
    Ok(json!({"removed": removed, "remaining": remaining}))
}

pub(super) fn text_to_columns(s: &mut Session, p: &Json) -> Result<Json> {
    let r = target_range(s, p)?;
    let sheet = target_sheet(s, p)?;
    let mut delims: Vec<char> = p
        .get("delimiters")
        .and_then(Json::as_array)
        .map(|a| a.iter().filter_map(|d| d.as_str().and_then(|x| x.chars().next())).collect())
        .unwrap_or_else(|| vec!['\t']);
    if let Some(o) = str_param(p, "other").and_then(|o| o.chars().next()) {
        delims.push(o);
    }
    let fixed: Option<Vec<usize>> =
        p.get("fixedWidths").and_then(Json::as_array).map(|a| a.iter().filter_map(|x| x.as_u64().map(|v| v as usize)).collect());
    let consecutive = bool_param(p, "treatConsecutive").unwrap_or(false);
    let quote = str_param(p, "textQualifier").and_then(|q| q.chars().next()).unwrap_or('"');
    let dest = cell_param_on(s, p, "destination", sheet)?.unwrap_or(r.start);
    // Excel's "Column data format": one for every column (`format`), or per column (`columnFormats`).
    let all = match str_param(p, "format") {
        Some(f) => ColFmt::parse(f).ok_or_else(|| bad("data.textToColumns", format!("unknown format `{f}`")))?,
        None => ColFmt::General,
    };
    let per_column: Vec<ColFmt> = match p.get("columnFormats").and_then(Json::as_array) {
        Some(a) => a
            .iter()
            .map(|f| f.as_str().and_then(ColFmt::parse).ok_or_else(|| bad("data.textToColumns", format!("unknown column format {f}"))))
            .collect::<Result<_>>()?,
        None => vec![],
    };
    let date_system = s.doc()?.wb.date_system;
    edit(s, |cx| {
        let Some(sh) = cx.wb.sheet(sheet) else { return Ok(()) };
        let mut out: Vec<(CellRef, String, ColFmt)> = Vec::new();
        for (i, row) in (r.start.row..=r.end.row).enumerate() {
            let src = CellRef::new(row, r.start.col);
            let v = sh.value(src);
            // A date is split and re-read as it is shown ("7/8/2024"), as Excel does: that is how a
            // column of dates read in the wrong order gets fixed.
            let shown_date = matches!(v, gridcraft_core::Value::Number(_))
                && crate::display::number_format(cx.wb.styles.get(sh.style_id(src)).num_fmt.as_str()).is_date();
            let text = if shown_date { crate::display::cell_text(&cx.wb, sh, src) } else { v.display() };
            let parts: Vec<String> = match &fixed {
                Some(w) => {
                    let chars: Vec<char> = text.chars().collect();
                    let mut cuts = vec![0];
                    let mut breaks: Vec<usize> = w.iter().copied().filter(|x| *x > 0 && *x < chars.len()).collect();
                    breaks.sort_unstable();
                    breaks.dedup();
                    cuts.extend(breaks);
                    cuts.push(chars.len());
                    cuts.windows(2)
                        .map(|p| chars.get(p[0]..p[1]).map(|s| s.iter().collect::<String>().trim().to_string()).unwrap_or_default())
                        .collect()
                }
                None => split_delimited(&text, &delims, consecutive, quote),
            };
            let mut j_out = 0u32;
            for (j, part) in parts.into_iter().enumerate() {
                let fmt = per_column.get(j).copied().unwrap_or(all);
                if fmt == ColFmt::Skip {
                    continue;
                }
                out.push((CellRef::new(dest.row + i as u32, dest.col.saturating_add(j_out)), part, fmt));
                j_out = j_out.saturating_add(1);
            }
        }
        for (c, t, fmt) in out {
            let cell = match fmt {
                ColFmt::Text if !t.is_empty() => crate::cmd::edit::input_to_cell(&format!("'{t}"), sheet, c, &mut cx.wb).unwrap_or(None),
                ColFmt::Date(order) => match date_in_order(&t, order, date_system) {
                    Some(serial) => crate::cmd::edit::input_to_cell(&serial.to_string(), sheet, c, &mut cx.wb).unwrap_or(None).map(|mut x| {
                        x.style = cx.wb.styles.derive(x.style, |st| st.num_fmt = gridcraft_model::NumFmt::new(date_code(order)));
                        x
                    }),
                    // Not a date in that order: kept as it reads, as Excel leaves it.
                    None => crate::cmd::edit::input_to_cell(&t, sheet, c, &mut cx.wb).unwrap_or(None),
                },
                _ => crate::cmd::edit::input_to_cell(&t, sheet, c, &mut cx.wb).unwrap_or(None),
            };
            let shm = cx.sheet_mut(sheet)?;
            match cell {
                Some(x) => shm.set_cell(c, x),
                None => {
                    shm.remove_cell(c);
                }
            }
            cx.changed.push((sheet, c));
        }
        Ok(())
    })?;
    ok()
}

/// A Text to Columns "Column data format".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColFmt {
    General,
    Text,
    Skip,
    /// A date written in this order of day, month and year (`b"DMY"`).
    Date([u8; 3]),
}

impl ColFmt {
    fn parse(s: &str) -> Option<ColFmt> {
        let u = s.trim().to_ascii_uppercase();
        match u.as_str() {
            "GENERAL" => Some(ColFmt::General),
            "TEXT" => Some(ColFmt::Text),
            "SKIP" => Some(ColFmt::Skip),
            "MDY" | "DMY" | "YMD" | "MYD" | "DYM" | "YDM" => {
                let b = u.as_bytes();
                Some(ColFmt::Date([*b.first()?, *b.get(1)?, *b.get(2)?]))
            }
            _ => None,
        }
    }
}

/// The number format a converted date is shown in: day first stays day first.
fn date_code(order: [u8; 3]) -> &'static str {
    match order.first() {
        Some(b'D') => "dd/mm/yyyy",
        Some(b'Y') => "yyyy-mm-dd",
        _ => "m/d/yyyy",
    }
}

/// `text` read as a date with its day, month and year in `order` ("16-08-2023" in DMY), as a serial.
/// The first three runs of digits are used (a time after them is ignored); a two-digit year is
/// 2000-2029 below 30 and 19xx otherwise, as Excel reads it. `None` when it isn't a real date.
fn date_in_order(text: &str, order: [u8; 3], sys: gridcraft_core::date::DateSystem) -> Option<f64> {
    let nums: Vec<&str> = text.split(|ch: char| !ch.is_ascii_digit()).filter(|t| !t.is_empty()).take(3).collect();
    if nums.len() < 3 {
        return None;
    }
    let (mut y, mut m, mut d) = (None, None, None);
    for (tok, which) in nums.iter().zip(order) {
        let n: i64 = tok.parse().ok()?;
        match which {
            b'Y' => y = Some(if tok.len() <= 2 { if n < 30 { 2000 + n } else { 1900 + n } } else { n }),
            b'M' => m = Some(n),
            _ => d = Some(n),
        }
    }
    let (y, m, d) = (y?, m?, d?);
    // serial_from_ymd rolls over like DATE() (month 16 → next April); a real date must read back as written.
    let serial = gridcraft_core::date::serial_from_ymd(sys, y, m, d)?;
    let back = gridcraft_core::date::datetime_from_serial(sys, serial)?;
    (i64::from(back.year) == y && i64::from(back.month) == m && i64::from(back.day) == d).then_some(serial)
}

pub(super) fn split_delimited(text: &str, delims: &[char], consecutive: bool, quote: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    let mut last_delim = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == quote {
            if in_q && chars.peek() == Some(&quote) {
                cur.push(quote);
                chars.next();
            } else {
                in_q = !in_q;
            }
            continue;
        }
        if !in_q && delims.contains(&c) {
            if consecutive && last_delim {
                continue;
            }
            out.push(std::mem::take(&mut cur));
            last_delim = true;
            continue;
        }
        last_delim = false;
        cur.push(c);
    }
    out.push(cur);
    out
}
