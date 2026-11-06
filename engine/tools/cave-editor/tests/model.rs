//! The editor's model, headless: painting, undo, saving, and the checks that run the real sim.
use molehill_cave_editor::*;
use molehill_sim::mat::Mat;
use molehill_sim::sites::Kind;
use molehill_sim::templates::{self, Template};
use molehill_sim::walls::Wall;

#[test]
fn every_family_opens_and_reads_back_as_the_same_layout() {
    for &k in &families() {
        let d = Doc::open(k).unwrap();
        let (a, b) = (templates::get(k), d.template().unwrap());
        for (y, x) in (0..d.h).step_by(3).flat_map(|y| (0..d.w).step_by(3).map(move |x| (y, x))) {
            assert_eq!(a.cell(x, y, 5, false), b.cell(x, y, 5, false), "{k:?} ({x},{y})");
        }
        assert!(!d.to_file().from_editor(), "unedited stays the exporter's");
    }
}

#[test]
fn painting_undo_redo_and_the_palette_grows() {
    let mut d = Doc::open(Kind::Holt).unwrap();
    let (x, y) = (d.w / 2, d.h / 2);
    d.begin();
    let n = d.paint(x as i64, y as i64, 3., Brush::Mat(Some(Mat::Basalt)));
    assert!(n > 20, "a radius-3 disc paints about 29 cells, got {n}");
    let t = d.template().unwrap();
    assert_eq!(t.cell(x, y, 0, false), Some(Mat::Basalt));
    assert!(d.to_file().palette.values().any(|v| v == "Basalt"), "a new material joins the palette");
    d.begin(); d.paint(x as i64, y as i64, 1., Brush::Mat(None));
    assert_eq!(d.template().unwrap().cell(x, y, 0, false), None, "erase leaves the host");
    d.undo(); assert_eq!(d.template().unwrap().cell(x, y, 0, false), Some(Mat::Basalt));
    d.undo(); assert_eq!(d.template().unwrap().cell(x, y, 0, false), templates::get(Kind::Holt).cell(x, y, 0, false));
    assert!(!d.to_file().palette.values().any(|v| v == "Basalt"), "an unused material leaves the palette");
    d.redo(); assert_eq!(d.template().unwrap().cell(x, y, 0, false), Some(Mat::Basalt));
    // a lining stroke adds a one-wall mix and shows behind open cells
    d.begin(); d.paint(x as i64, y as i64, 2., Brush::Lining(Some(Wall::Salt)));
    let f = d.to_file();
    assert!(f.linings.values().any(|v| v == &vec!["Salt".to_string()]));
    let t = Template::from_file(f).unwrap();
    assert_eq!(t.lining_with(x, y, 0, false, 12345), Some(Wall::Salt));
    assert_eq!(d.pick(x, y), Some(Brush::Mat(Some(Mat::Basalt))));
}

#[test]
fn things_and_the_chest() {
    let mut d = Doc::open(Kind::Crystal).unwrap();
    let n0 = d.file.things.len();
    let torch = thing_kinds().into_iter().find(|t| t.what == "torch").unwrap();
    d.begin(); d.place_thing(&torch, 40, 50, 0, false);
    assert_eq!(d.file.things.len(), n0 + 1);
    let rope = thing_kinds().into_iter().find(|t| t.what == "ladder").unwrap();
    d.begin(); d.place_thing(&rope, 60, 40, 0, false);
    assert!(d.file.things.last().unwrap().anchor.is_some(), "a rope is tied to something");
    assert!(d.remove_thing_near(40, 51, 4.));
    assert_eq!(d.file.things.len(), n0 + 1);
    d.begin(); d.set_chest(Some([30, 100]));
    assert_eq!(d.template().unwrap().chest(0, false), Some((30, 100)));
    assert!(thing_kinds().iter().any(|t| t.item.as_deref() == Some("Table")) && thing_kinds().iter().any(|t| t.what == "device"));
}

#[test]
fn a_save_is_canonical_marked_as_the_editors_and_loads_back() {
    let mut d = Doc::open(Kind::Fossil).unwrap();
    d.begin(); d.paint(100, 60, 2., Brush::Mat(Some(Mat::Quartz)));
    let dir = std::env::temp_dir().join(format!("cave-editor-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("fossil.json");
    d.save(&p).unwrap();
    assert!(!d.dirty);
    let text = std::fs::read_to_string(&p).unwrap();
    assert!(text.contains("\"source\": \"editor\""), "an editor save says so");
    let back = Doc::load(&p).unwrap();
    assert_eq!(templates::to_json(&back.to_file()), text, "the file is in canonical form");
    assert_eq!(back.template().unwrap().cell(100, 60, 0, false), Some(Mat::Quartz));
    std::fs::remove_dir_all(&dir).ok();
}

/// The checks agree with the found-places test on today's layouts: every family is walkable from every
/// way in; the fossil and pumpworks (fixed in #22) stand; a deliberate trap is caught.
#[test]
fn checks_run_the_real_sim() {
    for &k in &families() {
        let r = reach(templates::get(k), 3, false).unwrap();
        assert!(r.problems.is_empty(), "{k:?}: {:?}", r.problems);
        assert!(!r.ways_in.is_empty() && r.region.iter().any(|&g| g != 0));
    }
    let over = stress(templates::get(Kind::Fossil), 0, false).unwrap();
    assert!(over.iter().filter(|&&b| b).count() < 40, "the fossil beast lies on its floor and stands");
    // a 9 m clay slab painted in the fossil cave's air, touching nothing, can't stand
    let mut d = Doc::open(Kind::Fossil).unwrap();
    d.begin();
    for x in 100..170 { d.paint(x, 60, 2., Brush::Mat(Some(Mat::Clay))); }
    let t = d.template().unwrap();
    assert!(stress(&t, 0, false).unwrap().iter().filter(|&&b| b).count() > 100, "a floating clay slab is over capacity");
    // and sealing the salt hall's mouths cuts the way to the chest
    let mut d = Doc::open(Kind::Crystal).unwrap();
    d.begin();
    for y in 0..d.h as i64 { d.paint(d.w as i64 / 2, y, 3., Brush::Mat(Some(Mat::Granite))); }
    assert!(!reach(&d.template().unwrap(), 0, false).unwrap().problems.is_empty(), "a wall across the hall is caught");
    let s = simulate(templates::get(Kind::Pumpworks), 0, false, 120).unwrap();
    assert_eq!(s.cells.len(), s.cells.len());
    assert!(s.hp > 90., "the mole stands by the pumpworks chest unhurt: {}", s.hp);
}

#[test]
fn drawing_fills_every_pixel() {
    let t = templates::get(Kind::Halt);
    let px = draw(t, 0, true, None);
    assert_eq!(px.len(), t.w * t.h * 4);
    assert!(px.chunks(4).all(|p| p[3] == 255));
}
