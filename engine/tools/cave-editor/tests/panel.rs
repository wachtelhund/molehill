//! Click-through of the panel and canvas logic (`Editor`), the way the window drives it: every row is
//! clicked at its y, every key pressed, strokes painted. This covers what each control does; how the
//! window *looks* (fonts, spacing, colours on screen) still needs a person at a real window.
use molehill_cave_editor::*;
use molehill_sim::mat::Mat;

fn click(e: &mut Editor, text_starts: &str) -> bool {
    let rows = e.rows(&[]);
    let i = rows.iter().position(|r| r.text.trim_start().starts_with(text_starts)).unwrap_or_else(|| panic!("no row {text_starts:?} in {:#?}", rows.iter().map(|r| &r.text).collect::<Vec<_>>()));
    e.click_panel(ROW_TOP + i as f32 * LINE + LINE / 2., &[])
}
fn temp_dir() -> std::path::PathBuf { let d = std::env::temp_dir().join(format!("cave-editor-panel-{}", std::process::id())); std::fs::create_dir_all(&d).unwrap(); d }

#[test]
fn every_row_with_a_command_does_it() {
    let mut e = Editor::new(0).unwrap();
    e.save_dir = temp_dir();
    // the panel fits the window's rows, whatever the message and slots
    assert!(panel_fits());
    // a click between rows or on a label does nothing
    let before = e.rows(&[]);
    assert!(!e.click_panel(ROW_TOP + 0.5 * LINE, &[]), "the title row has no command");
    assert!(!e.click_panel(-5., &[]));
    assert_eq!(e.rows(&[]), before);
    // tools
    for (row, tool) in [("L  paint lining", Tool::Lining), ("T  set things", Tool::Thing), ("C  move the chest", Tool::Chest), ("M  paint material", Tool::Material)] {
        assert!(click(&mut e, row)); assert_eq!(e.tool, tool, "{row}");
    }
    // every palette row of every tool picks its entry
    for tool in ["M  paint material", "L  paint lining", "T  set things"] {
        click(&mut e, tool);
        let n = e.pal_len();
        for k in 0..n {
            let rows = e.rows(&[]);
            let label = e.pal_label(k);
            if let Some(i) = rows.iter().position(|r| r.text.trim_start_matches(['>', ' ']) == label) { e.click_panel(ROW_TOP + i as f32 * LINE + 1., &[]); assert_eq!(e.pal_cur(), k, "{label}"); }
            else { e.run(Act::Pal(k)); } // off the visible window of the palette: the keys get there
            assert!(e.rows(&[]).iter().any(|r| r.on && r.text.trim_start_matches(['>', ' ']) == label), "the picked entry {label:?} is shown and lit");
        }
    }
    click(&mut e, "M  paint material");
    // brush, variant, side
    let b = e.brush; click(&mut e, "bigger"); assert!(e.brush > b); click(&mut e, "smaller"); assert_eq!(e.brush, b);
    click(&mut e, "+1"); click(&mut e, "+1"); assert_eq!(e.var, 2); click(&mut e, "-1"); assert_eq!(e.var, 1);
    click(&mut e, "N  side"); assert!(e.left); click(&mut e, "N  side"); assert!(!e.left);
    // checks: each lights its row and sets its overlay; Esc clears
    click(&mut e, "R  can the mole walk it"); assert!(matches!(e.overlay, Overlay::Reach(_)) && e.msg.starts_with("Walkable"), "{}", e.msg);
    assert!(e.rows(&[]).iter().any(|r| r.on && r.text.starts_with("R ")));
    click(&mut e, "P  will it stand"); assert!(matches!(e.overlay, Overlay::Stress(_)));
    click(&mut e, "Esc"); assert!(matches!(e.overlay, Overlay::None));
    // families: a clean document switches at once
    click(&mut e, "5  fossil"); assert_eq!(kind_label(e.kind()), "fossil");
    click(&mut e, "F  fit"); assert!(e.fit);
    // undo / redo / save
    click(&mut e, "Ctrl/Cmd+Z  undo"); assert_eq!(e.msg, "Nothing to undo.");
    e.press(100, 60, Press::Left); assert!(e.doc.dirty);
    assert!(e.rows(&[])[0].text.contains("*unsaved*"));
    click(&mut e, "Ctrl/Cmd+Z  undo"); assert_eq!(e.msg, "Undone.");
    click(&mut e, "Ctrl/Cmd+Shift+Z"); assert_eq!(e.msg, "Redone.");
    click(&mut e, "Ctrl/Cmd+S  save"); assert!(e.msg.starts_with("Saved"), "{}", e.msg); assert!(!e.doc.dirty);
    assert!(e.save_dir.join("fossil.json").exists());
    std::fs::remove_dir_all(&e.save_dir).ok();
}

#[test]
fn keys_do_what_the_panel_says() {
    let mut e = Editor::new(0).unwrap();
    for (k, check) in [("l", Tool::Lining), ("t", Tool::Thing), ("c", Tool::Chest), ("m", Tool::Material)] { e.key(k); assert_eq!(e.tool, check); }
    let i = e.pal_cur(); e.key("."); assert_eq!(e.pal_cur(), i + 1); e.key(","); assert_eq!(e.pal_cur(), i);
    e.key(","); e.key(","); let n = e.pal_len(); for _ in 0..n { e.key("."); } assert_eq!(e.pal_cur(), (i + n - 2) % n, "the palette wraps");
    let b = e.brush; e.key("]"); assert_eq!(e.brush, b + 0.5); e.key("[");
    e.key("b"); e.key("B"); assert_eq!(e.var, 17); e.key("V"); e.key("v"); assert_eq!(e.var, 0); e.key("v"); assert_eq!(e.var, 0, "var stops at 0");
    e.key("n"); assert!(e.left);
    e.key("p"); assert!(matches!(e.overlay, Overlay::Stress(_))); e.key("esc"); assert!(matches!(e.overlay, Overlay::None));
    e.key("f"); assert!(e.fit);
    e.key("3"); assert_eq!(kind_label(e.kind()), "pumpworks");
    e.key("q"); e.key("^q"); // unbound: nothing happens
}

#[test]
fn a_stroke_is_one_undo_and_unsaved_work_is_guarded() {
    let mut e = Editor::new(1).unwrap(); // holt
    e.run(Act::Pal(e.mats.iter().position(|&m| m == Some(Mat::Basalt)).unwrap()));
    let t0 = e.doc.template().unwrap();
    e.press(40, 50, Press::Left);
    for x in 41..80 { e.press(x, 50, Press::LeftHeld); }
    e.release();
    e.press(40, 70, Press::Left); e.release();
    let t = e.doc.template().unwrap();
    assert_eq!(t.cell(60, 50, 0, false), Some(Mat::Basalt));
    e.key("^z"); // the second, separate dab
    assert_eq!(e.doc.template().unwrap().cell(60, 50, 0, false), Some(Mat::Basalt));
    e.key("^z"); // the whole drag at once
    assert_eq!(e.doc.template().unwrap().cell(60, 50, 0, false), t0.cell(60, 50, 0, false));
    e.key("^Z"); assert_eq!(e.doc.template().unwrap().cell(60, 50, 0, false), Some(Mat::Basalt));
    // right mouse picks; switching family with unsaved work asks twice
    e.key("m"); e.run(Act::Pal(0));
    e.press(60, 50, Press::Right); assert_eq!(e.mats[e.pal_cur()], Some(Mat::Basalt));
    e.key("2"); // same family: nothing
    e.key("4"); assert_eq!(kind_label(e.kind()), "holt"); assert!(e.msg.contains("press again"));
    e.key("4"); assert_eq!(kind_label(e.kind()), "fissure");
    // things and the chest from the canvas
    e.key("t"); let n = e.doc.file.things.len(); e.press(70, 100, Press::Left); assert_eq!(e.doc.file.things.len(), n + 1);
    e.press(70, 100, Press::Right); assert_eq!(e.doc.file.things.len(), n);
    e.key("c"); e.press(70, 300, Press::Left); assert_eq!(e.doc.file.chest, Some([70, 300]));
    e.press(1, 1, Press::Right); assert_eq!(e.doc.file.chest, None);
}
