//! Big files open on a worker thread (Rows&Columns G07): the window keeps running frames while
//! the file loads, the workbook arrives as with File › Open, and Esc cancels.

use gridcraft_ui_egui::SheetApp;

type Harness = egui_kittest::Harness<'static, SheetApp>;

/// A CSV just over the background threshold (~2.5 MB): 60,000 rows.
fn big_csv(tag: &str) -> String {
    let dir = std::env::temp_dir().join(format!("rc-bg-open-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("orders.csv");
    let mut body = String::from("ORDER_ID,CITY,VEHICLE,CUSTOMER,REVENUE,NOTE\n");
    for i in 0..60_000 {
        body.push_str(&format!("{i},Ahmedabad,OUTSTATION,Retail,{},Some longer free text here\n", i * 7 % 9000));
    }
    std::fs::write(&path, body).unwrap();
    path.to_string_lossy().into_owned()
}

fn harness() -> Harness {
    crate::support::blank_harness(4)
}

fn wait_until_open(h: &mut Harness) {
    for _ in 0..600 {
        if h.state().opening_in_background().is_none() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.run_steps(1);
    }
    panic!("the background open never finished");
}

#[test]
fn a_big_file_opens_in_the_background() {
    let path = big_csv("open");
    let mut h = harness();
    h.state_mut().open_path(&path);
    assert_eq!(h.state().opening_in_background(), Some(path.as_str()), "a big file goes to the worker");
    h.run_steps(2); // frames keep running while it loads
    wait_until_open(&mut h);
    let d = h.state().session.active().unwrap();
    assert_eq!(d.path.as_deref(), Some(path.as_str()));
    let sh = d.wb.active().unwrap();
    assert_eq!(sh.used_range().unwrap().end.row, 60_000, "every row arrived");
    assert_eq!(h.state().session.documents().len(), 1, "the blank Book1 was replaced");
}

#[test]
fn esc_cancels_a_background_open() {
    let path = big_csv("cancel");
    let mut h = harness();
    h.state_mut().open_path(&path);
    assert!(h.state().opening_in_background().is_some());
    h.input_mut().events.push(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(2);
    assert!(h.state().opening_in_background().is_none());
    std::thread::sleep(std::time::Duration::from_millis(300));
    h.run_steps(3);
    assert!(h.state().session.documents().iter().all(|d| d.path.as_deref() != Some(path.as_str())), "a cancelled open never arrives");
}

#[test]
fn small_files_still_open_at_once() {
    let dir = std::env::temp_dir().join(format!("rc-bg-open-{}-small", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("small.csv");
    std::fs::write(&path, "A,B\n1,2\n").unwrap();
    let path = path.to_string_lossy().into_owned();
    let mut h = harness();
    h.state_mut().open_path(&path);
    assert!(h.state().opening_in_background().is_none());
    assert_eq!(h.state().session.active().unwrap().path.as_deref(), Some(path.as_str()));
}

/// `RC_OPEN_SCREENSHOT=dir cargo test … background_open_screenshot` saves the "Opening…" overlay.
#[test]
fn background_open_screenshot() {
    let Ok(dir) = std::env::var("RC_OPEN_SCREENSHOT") else { return };
    let path = big_csv("shot");
    let mut h = harness();
    h.state_mut().ui.dark = true;
    h.state_mut().ui.system_theme = false;
    h.run_steps(10);
    h.state_mut().open_path(&path);
    h.run_steps(3);
    h.render().unwrap().save(std::path::Path::new(&dir).join("opening.png")).unwrap();
    wait_until_open(&mut h);
}
