//! Found places can be walked. Each site is stamped alone in granite, and the mole's body positions are
//! flood-filled (`World::body_regions`: the sim's collision box, gravity ignored: there is no wall climbing,
//! but the early rope gun hooks the rock overhead and takes the mole up any face, and rope ladders down). Two checks per site:
//! * goal: from a way in (a region that reaches the site's edge: a mouth or a door) the body gets to the
//!   chest, which stands in open space so it opens; or for the fissure (which has none) to its floor;
//! * whole: the site's open space is one walk, with no big room cut off from the way in.
//! Layouts that fail today are listed below (see REFINE-movement.md and `examples/sitewalk.rs` for maps).
//! The test also fails when a listed site has been opened up, so take it off the list: they only shrink.
use molehill_sim::{world::World, mat::{Mat, CPM}, sites::Kind};

/// Sites whose goal no way in reaches (stamped layouts as of 2026-11-06, env-rework v2).
const GOAL_SEALED: &[&str] = &[];
/// Sites with a big room (4 m² of body positions or more) cut off from the main way in.
const CUT: &[&str] = &[];
const KINDS: [&str; 7] = ["holt", "mine", "pump", "fissure", "fossil", "crystal", "halt"];

struct Walk { goal: bool, cut: Vec<usize> }
fn walk(kind: Kind) -> Walk {
    let mut w = World::new(1024, 512, 1); w.disasters.enabled = false;
    let (sw, sh) = kind.size(); let m = 2 * CPM; let (x0, y0) = (64, 512 - sh - 3 * m);
    for y in y0 - m..y0 + sh + m { for x in x0 - m..x0 + sw + m { let i = w.g.idx(x, y); w.g.set(i, Mat::Granite); w.g.flags[i] = 0; } }
    w.tr.ladders.clear(); w.chests.clear(); w.stamp_site(kind, x0, y0);
    let (bx0, by0, bw, bh) = (x0 - m / 2, y0 - m / 2, sw + m, sh + m);
    let (reg, sizes) = w.body_regions(bx0, by0, bw, bh);
    let mut edge = vec![false; sizes.len()];
    let mut goal = vec![false; sizes.len()];
    // a chest opens only standing in open space (`World::check_chests`): one stamped inside a solid can't be had
    let chest = w.chests.first().map(|c| (c.x, c.y));
    let buried = chest.map_or(false, |(cx, cy)| w.g.is_solidish(w.g.idx(cx, cy)));
    for (i, &r) in reg.iter().enumerate() { if r == 0 { continue; } let (gx, gy) = (bx0 + i % bw, by0 + i / bw);
        if gx < x0 + CPM / 2 || gx + CPM / 2 >= x0 + sw || gy < y0 + CPM { edge[r as usize] = true; }
        let at_goal = match chest { Some((cx, cy)) => gx.abs_diff(cx) <= CPM && gy.abs_diff(cy) <= CPM, None => gy + 2 * CPM >= y0 + sh };
        if at_goal { goal[r as usize] = true; } }
    let main = (1..sizes.len()).filter(|&k| edge[k]).max_by_key(|&k| sizes[k]);
    let big = 4 * CPM * CPM;
    Walk { goal: !buried && (1..sizes.len()).any(|k| edge[k] && goal[k]), cut: (1..sizes.len()).filter(|&k| Some(k) != main && sizes[k] >= big).map(|k| sizes[k]).collect() }
}

#[test]
fn found_places_can_be_walked() {
    let mut bad = vec![];
    for k in KINDS {
        let r = walk(Kind::from_name(k).unwrap());
        let goal = if k == "fissure" { "its floor" } else { "its chest" };
        match (GOAL_SEALED.contains(&k), r.goal) {
            (false, false) => bad.push(format!("{k}: no way in reaches {goal}")),
            (true, true) => bad.push(format!("{k}: a way in reaches {goal} now: take it off GOAL_SEALED")),
            _ => {}
        }
        match (CUT.contains(&k), r.cut.is_empty()) {
            (false, false) => bad.push(format!("{k}: rooms cut off from the way in ({:?} body positions)", r.cut)),
            (true, true) => bad.push(format!("{k}: one walk now: take it off CUT")),
            _ => {}
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
