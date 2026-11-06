//! Seeded variation of placed found places (`templates::Variation`): mirroring, depth remaps and tagged
//! random zones. Below `VARY_FROM` a site is exactly its template (scripted stamps, scenarios, the golden
//! range); above it every roll comes from the site's own `var`, so a site always comes out the same.
use molehill_sim::mat::Mat;
use molehill_sim::sites::{Kind, Site};
use molehill_sim::templates::{self, Placement, FAMILIES, VARY_FROM};

fn at(var: u32, depth: f32) -> Placement { Placement { var, left: false, depth } }

#[test]
fn below_vary_from_a_site_is_its_template() {
    for &(k, _) in &FAMILIES {
        let t = templates::get(k);
        for var in [0, 3, 95, VARY_FROM - 1] {
            for dy in (0..t.h).step_by(5) { for dx in (0..t.w).step_by(5) {
                assert_eq!(t.placed_cell(dx, dy, at(var, 200.)), t.cell(dx, dy, var, false), "{k:?} var {var}");
            } }
            assert_eq!(t.placed_chest(at(var, 0.)), t.chest(var, false));
            assert_eq!(t.placed_things(at(var, 0.)), t.things(var, false));
        }
    }
}

#[test]
fn mirroring_flips_cells_chest_and_things_and_about_half_are_flipped() {
    for &(k, name) in &FAMILIES {
        let t = templates::get(k);
        let vars: Vec<u32> = (1..400u32).map(|i| i << 8).collect();
        let n = vars.iter().filter(|&&v| t.mirrored(v)).count();
        if !t.variation.mirror { assert_eq!(n, 0, "{name} never flips"); continue; }
        assert!(n > 150 && n < 250, "{name}: {n} of 399 flipped");
        let v = *vars.iter().find(|&&v| t.mirrored(v) && t.variation.zones.iter().enumerate().all(|(z, _)| !t.zone_on(z, v) || t.variation.zones[z].tag != "may-collapse")).unwrap();
        for dy in (0..t.h).step_by(3) { for dx in (0..t.w).step_by(3) {
            assert_eq!(t.placed_cell(dx, dy, at(v, 0.)), t.cell(t.w - 1 - dx, dy, v, false), "{name} ({dx},{dy})");
        } }
        if let (Some((x, y)), Some((mx, my))) = (t.chest(v, false), t.placed_chest(at(v, 0.))) { assert_eq!((mx, my), (t.w - 1 - x, y), "{name} chest"); }
        for th in t.placed_things(at(v, 0.)) { let s = format!("{th:?}"); assert!(!s.contains("18446744"), "{name}: a thing left the box: {s}"); }
    }
}

#[test]
fn depth_remaps_follow_the_layers() {
    // deeper is hotter and harder: a deep fossil cave's mound and floor are basalt, a shallow one's rock
    let t = templates::get(Kind::Fossil);
    let v = (1..400u32).map(|i| i << 8).find(|&v| !t.mirrored(v)).unwrap();
    let rock: Vec<(usize, usize)> = (0..t.h).flat_map(|y| (0..t.w).map(move |x| (x, y))).filter(|&(x, y)| t.cell(x, y, v, false) == Some(Mat::Rock)).collect();
    assert!(rock.len() > 100);
    assert!(rock.iter().all(|&(x, y)| t.placed_cell(x, y, at(v, 30.)) == Some(Mat::Rock)), "shallow: rock");
    assert!(rock.iter().all(|&(x, y)| t.placed_cell(x, y, at(v, 70.)) == Some(Mat::Basalt)), "deep: basalt");
    // a placed site takes the depth of its middle
    let deep = Site::new(1, Kind::Fossil, 0, 70 * 8, v, false, None);
    let (x, y) = rock[0];
    assert_eq!(deep.cell(x, deep.y0 + y as i64), Some(Mat::Basalt));
}

#[test]
fn random_zones_fire_at_their_chance() {
    let vars: Vec<u32> = (1..2000u32).map(|i| i << 8).collect();
    // halts keep their chest half the time, salt halls three times in four; mines and holts always (the first-hour find)
    for (k, lo, hi) in [(Kind::Halt, 0.42, 0.58), (Kind::Crystal, 0.68, 0.82), (Kind::Mine, 1., 1.), (Kind::Holt, 1., 1.)] {
        let t = templates::get(k);
        let n = vars.iter().filter(|&&v| t.placed_chest(at(v, 0.)).is_some()).count() as f32 / vars.len() as f32;
        assert!(n >= lo && n <= hi, "{k:?}: chest {n}");
    }
    // the mine's worked face caves in about a third of the time, with loose gravel only where it was open
    let t = templates::get(Kind::Mine);
    let z = t.variation.zones.iter().position(|z| z.tag == "may-collapse").unwrap();
    let on = vars.iter().filter(|&&v| t.zone_on(z, v)).count() as f32 / vars.len() as f32;
    assert!((0.28..0.42).contains(&on), "collapse {on}");
    let v = *vars.iter().find(|&&v| t.zone_on(z, v) && !t.mirrored(v)).unwrap();
    let [x, y, w, h] = t.variation.zones[z].rect.unwrap();
    let mut filled = 0;
    for dy in y..y + h { for dx in x..x + w {
        let (before, after) = (t.cell(dx, dy, v, false), t.placed_cell(dx, dy, at(v, 0.)));
        if before == Some(Mat::Empty) { assert_eq!(after, Some(Mat::Gravel)); filled += 1; } else { assert_eq!(after, before, "only open cells fill"); }
    } }
    assert!(filled > 100, "{filled}");
    // a holt keeps one set of banners: two or one, never none or all three
    let t = templates::get(Kind::Holt);
    let banners = |v: u32| t.placed_things(at(v, 0.)).iter().filter(|th| format!("{th:?}").contains("Banner")).count();
    let counts: std::collections::BTreeSet<usize> = vars.iter().map(|&v| banners(v)).collect();
    assert_eq!(counts.into_iter().collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(banners(0), 3, "unvaried: all three");
}

/// Generated worlds use it: of the sites over many seeds, about half the flippable ones are flipped.
#[test]
fn generated_sites_vary() {
    use molehill_sim::gen::Gen;
    let _ = Gen::new; // (the generator places sites with 20 random bits of var)
    let mut flipped = 0; let mut n = 0;
    for seed in 1..60u32 {
        let g = Gen::new(seed, 2048);
        for s in g.sites_in_rows(-40 * 8, 120 * 8) {
            if s.var < VARY_FROM { continue; }
            let t = templates::get(s.kind);
            if !t.variation.mirror { continue; }
            n += 1; if t.mirrored(s.var) { flipped += 1; }
        }
    }
    assert!(n > 20, "{n} sites");
    let f = flipped as f32 / n as f32;
    assert!((0.25..0.75).contains(&f), "{flipped} of {n} flipped");
}
