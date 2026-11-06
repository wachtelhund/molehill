//! Golden check: the site templates compiled in from `assets/sites/` stamp exactly what the procedural
//! reference layouts in `sites.rs` do, cell for cell, for every variant a site can be (var 0..96 covers
//! each layout knob's period, plus a few large vars, both halt sides): materials, linings, the chest and
//! the things it owns. If this fails after a layout change in `sites.rs`, re-export:
//! `cargo run --release -p molehill-sim --example export_sites` (from `engine/`).
use molehill_sim::sites::{Kind, Knobs, Site};
use molehill_sim::templates::{self, Template, FAMILIES};

#[test]
fn templates_match_the_procedural_sites() {
    let mut bad = vec![];
    for &(kind, name) in &FAMILIES {
        let t = templates::get(kind);
        // painted in the cave editor: sites.rs is no longer its source
        if t.file.from_editor() { continue; }
        let (w, h) = kind.size();
        assert_eq!((t.w, t.h), (w, h), "{name}: size");
        let lefts: &[bool] = if kind == Kind::Halt { &[false, true] } else { &[false] };
        for &left in lefts {
            for var in (0..96u32).chain([1 << 19, 123_457, (1 << 20) - 1]) {
                let k = Knobs::of(var);
                let s = Site { id: 0, kind, x0: 0, y0: 0, w, h, var, left, chest: None, link: None };
                let mut diff = 0; let mut first = None;
                for dy in 0..h { for dx in 0..w {
                    let (a, b) = (t.cell(dx, dy, var, left), s.cell_reference(dx, dy as i64, &k));
                    let lining_ok = b.is_none() || [0u32, 3, 9, 0x7777_7777].iter().all(|&hh| t.lining_with(dx, dy, var, left, hh) == Some(s.lining_reference(dx as i64, dy as i64, hh)));
                    if a != b || !lining_ok { diff += 1; first.get_or_insert((dx, dy, a, b)); }
                } }
                if diff > 0 { bad.push(format!("{name} var={var} left={left}: {diff} cells differ, first {first:?}")); }
                if t.chest(var, left) != Site::chest_reference(kind, left) { bad.push(format!("{name} var={var} left={left}: chest")); }
                if t.things(var, left) != Site::things_reference(&s, 0, 0) { bad.push(format!("{name} left={left}: things")); }
            }
            // things are laid out from the box corner, so the box-relative list moved is the placed list
            let s = Site { id: 0, kind, x0: 0, y0: 0, w, h, var: 0, left, chest: None, link: None };
            let moved: Vec<_> = Site::things_reference(&s, 0, 0).into_iter().map(|t| t.offset(96, 40)).collect();
            if moved != Site::things_reference(&s, 96, 40) { bad.push(format!("{name}: things aren't relative to the box")); }
        }
    }
    assert!(bad.is_empty(), "the templates in assets/sites/ don't match sites.rs (re-run the export_sites example):\n{}", bad.join("\n"));
}

/// A site placed in the world takes its chest from the template, at the box offset.
#[test]
fn site_chest_comes_from_the_template() {
    for &(kind, _) in &FAMILIES {
        for left in [false, true] {
            let s = Site::new(7, kind, 100, -50, 3, left, None);
            let want = Site::chest_reference(kind, left).map(|(x, y)| (100 + x, -50 + y as i64));
            assert_eq!(s.chest, want, "{kind:?} left={left}");
        }
    }
}

/// The file format round-trips through the writer, and a broken file says what is wrong.
#[test]
fn template_files_round_trip_and_explain_errors() {
    for &(kind, _) in &FAMILIES {
        let f = &templates::get(kind).file;
        let json = templates::to_json(f);
        let back = Template::parse(&json).expect("re-parse");
        assert_eq!(&back.file, f);
    }
    let mut f = templates::get(Kind::Holt).file.clone();
    f.cells[3] = "5.".into();
    let e = Template::from_file(f).err().unwrap();
    assert!(e.contains("row 3"), "{e}");
    let mut f = templates::get(Kind::Holt).file.clone();
    f.cpm = 4;
    assert!(Template::from_file(f).err().unwrap().contains("cells/m"));
    let mut f = templates::get(Kind::Holt).file.clone();
    f.palette.insert("Z".into(), "Unobtainium".into());
    assert!(Template::from_file(f).err().unwrap().contains("unknown material"));
    assert_eq!(templates::rle_decode("3.2#o").unwrap().iter().collect::<String>(), "...##o");
    assert_eq!(templates::rle_encode("...##o".chars()), "3.2#o");
}

/// Each file in assets/sites/ is exactly what the writer makes of its data (no hand-formatting drift,
/// so an editor save or a re-export only shows real changes in a diff), and parsing all seven is quick.
#[test]
fn template_files_are_canonical_and_parse_fast() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/sites");
    let t0 = std::time::Instant::now();
    for &(_, name) in &FAMILIES {
        let text = std::fs::read_to_string(dir.join(format!("{name}.json"))).unwrap();
        let t = Template::parse(&text).unwrap();
        assert!(templates::to_json(&t.file) == text, "{name}.json isn't in canonical form: re-run the export_sites example");
    }
    let ms = t0.elapsed().as_secs_f32() * 1000.;
    eprintln!("parsed and re-wrote seven templates in {ms:.1} ms");
    assert!(ms < 250., "templates took {ms:.0} ms");
}

/// `World::stamp_template` (the cave editor's checks) stamps what the game's own site stamping does, with
/// and without seeded variation.
#[test]
fn stamp_template_matches_the_game_stamp() {
    use molehill_sim::mat::Mat;
    use molehill_sim::world::World;
    for (&(kind, name), var) in FAMILIES.iter().flat_map(|f| [5u32, (77 << 8) | 5, (1234 << 8) | 2].map(move |v| (f, v))) {
        let mk = || { let mut w = World::new(1024, 512, 1); w.disasters.enabled = false; w.chests.clear(); w.torches.clear(); w.furniture.clear(); w.tr.ladders.clear(); w };
        let (mut a, mut b) = (mk(), mk());
        let (x0, y0) = (64, 512 - kind.size().1 - 48);
        a.stamp_site_var(kind, x0, y0, var, true);
        let (id, p) = (a.sites.last().unwrap().id, a.sites.last().unwrap().placement());
        b.stamp_template(templates::get(kind), x0, y0, p, id);
        assert!(a.g.mat == b.g.mat, "{name}: cells");
        assert!(a.walls.cell == b.walls.cell, "{name}: linings");
        assert_eq!(a.chests.iter().map(|c| (c.x, c.y)).collect::<Vec<_>>(), b.chests.iter().map(|c| (c.x, c.y)).collect::<Vec<_>>(), "{name}: chest");
        assert_eq!(a.torches, b.torches, "{name}: torches");
        assert_eq!(a.tr.ladders.len(), b.tr.ladders.len(), "{name}: ropes");
        let _ = Mat::Empty;
    }
}
