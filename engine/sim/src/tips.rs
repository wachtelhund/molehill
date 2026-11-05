//! Hover explanations for things in the world: what's under a cell, in plain Delver words.
//! Pure sim so it can be tested headless; the game shows `name` on the first line and `lines` under it.
use crate::craft::Item;
use crate::grid::F_SEEN;
use crate::logic::Dev;
use crate::mat::{Kind, Mat, S};
use crate::world::World;
use crate::craft::Station;

pub struct Tip { pub name: String, pub lines: Vec<String> }

/// How hard a standing block is working, from the stress solver's utilisation (×128 = at capacity).
/// 0 idle, 1 bearing, 2 straining, 3 overloaded, 4 cracking.
pub fn strain_level(util: u8, cracking: bool) -> u8 { if cracking { 4 } else { match util { 0..=31 => 0, 32..=79 => 1, 80..=114 => 2, _ => 3 } } }
pub fn strain_word(util: u8, cracking: bool) -> Option<&'static str> {
    [None, Some("Bearing weight."), Some("Straining under its load."), Some("Overloaded: it will crack soon."), Some("Cracking! It will give way.")][strain_level(util, cracking) as usize]
}
/// Strain level of a standing solid at window cell i (0 for anything else).
pub fn strain_at(w: &World, i: usize) -> u8 {
    let m = w.g.m(i); if m.kind() != Kind::Solid || w.g.loose(i) { return 0; }
    strain_level(w.st.util.get(i).copied().unwrap_or(0), w.st.pending.get(i).copied().unwrap_or(0) > 0)
}

/// Describe what's at window cell (x, y), or None for open air or rock the colony hasn't seen.
/// Order: station, chest, mushroom plot, device, copperline, material.
pub fn describe(w: &World, x: usize, y: usize) -> Option<Tip> {
    if x >= w.g.w || y >= w.g.h { return None; }
    let i = y * w.g.w + x;
    if w.g.flags[i] & F_SEEN == 0 { return None; }
    let tip = |n: String, l: Vec<String>| Some(Tip { name: n, lines: l });
    for b in &w.buildings {
        if x >= b.x && x < b.x + Station::W && y + Station::H > b.y && y <= b.y {
            let makes: Vec<&str> = crate::craft::RECIPES.iter().filter(|r| r.station == b.kind).map(|r| r.name).take(4).collect();
            if b.built {
                // a station that can't work says why, rather than inviting a craft that will be refused
                let how = match w.station_ready(b) { Ok(()) => "Stand here and open the pack (I) to craft.".to_string(), Err(e) => format!("{}{}.", e[..1].to_uppercase(), &e[1..]) };
                return tip(b.kind.name().into(), vec![b.kind.desc().into(), format!("Makes {}.", makes.join(", ").to_lowercase()), how]);
            }
            // planned: what it still needs, against what's in the pack
            let need: Vec<String> = b.kind.cost().iter().map(|&(it, n)| { let have = w.inv[it as usize];
                if have >= n { format!("{n} {} (have)", it.name().to_lowercase()) } else { format!("{n} {} (have {have})", it.name().to_lowercase()) } }).collect();
            return tip(format!("{} (planned)", b.kind.name()), vec![b.kind.desc().into(), format!("Needs {}.", need.join(", "))]);
        }
    }
    if let Some(k) = w.furniture_at(x, y) {
        let f = &w.furniture[k]; let mut l = vec![f.item.desc().to_string()];
        if f.tint > 0 { l.push(format!("Dyed {}.", pigment_name(f.tint))); }
        l.push("E: pick up.".into());
        return tip(f.item.name().into(), l);
    }
    for c in &w.clues {
        let r = S + 1; if x + r < c.x || x > c.x + r || y + 2 * r < c.y || y > c.y + 1 || w.g.is_solidish(c.y * w.g.w + c.x) { continue; }
        let say = match c.kind { crate::lore::ClueKind::Carving => "Marks cut into the rock by an older Delver.", crate::lore::ClueKind::Note => "A scrap of bark, scratched with old writing.",
            crate::lore::ClueKind::Draft => "A cool draft, falling from somewhere above.", crate::lore::ClueKind::Relic => "Something that doesn't belong down here." };
        return tip(c.kind.name().into(), vec![say.into(), if c.found { "In your journal (J).".into() } else { "Walk up to it to read it.".into() }]);
    }
    for c in &w.chests {
        let k = crate::mat::S;
        if x + 2 * k >= c.x && x <= c.x + 2 * k - 1 && y + 3 * k >= c.y && y <= c.y && !w.g.is_solidish(c.y * w.g.w + c.x) {
            return tip("Old Delver chest".into(), vec![if c.opened { "Empty now.".into() } else { "Walk up to it to open it.".into() }]);
        }
    }
    for f in &w.farms {
        let k = crate::mat::S;
        if x + k >= f.x && x <= f.x + 2 * k - 1 && y + 2 * k >= f.y && y <= f.y {
            let mut l = vec![format!("Growing: {}%. Drops mushrooms when grown.", (f.growth.clamp(0., 1.) * 100.).round())];
            if f.why & 1 != 0 { l.push("Too bright here: mushrooms want the dark.".into()); }
            if f.why & 2 != 0 { l.push("Too dry: it needs water nearby.".into()); }
            return tip("Mushroom".into(), l);
        }
    }
    let b = w.logic.block_of(x, y);
    let d = Dev::from_u8(w.logic.dev[b]);
    if d != Dev::None { return tip(d.name().into(), vec![d.desc().into(), if w.logic.lit(b) { "Burning: too late to pick it up.".into() } else { "E: pick up.".into() }]); }
    if w.torch_at(x, y).is_some() { return tip("Torch".into(), vec![Item::Torch.desc().into(), "E: pick up.".into()]); }
    if w.logic.wire[b] != 0 { return tip("Copperline".into(), vec![Item::CopperWire.desc().into(), if w.logic.net_lit(b) { "Carrying current.".into() } else { "Idle.".into() }, "E: take up (hold and sweep for a run).".into()]); }
    let m = w.g.m(i);
    if m == Mat::Empty { return None; }
    let loose = w.g.loose(i) && m.kind() == Kind::Solid;
    let mut l = vec![tool_free(m.desc())];
    // digging: what the mole's own tools do to it, instead of the table's general note on tools
    if let Some(r) = w.dig_refusal(m) { if m != Mat::Bedrock { l.push(r); } }
    else if w.can_dig(m) { if let Some(it) = crate::craft::drop_of(m) { l.push(format!("Digging it gives {}.", it.name().to_lowercase())); } }
    if m.kind() == Kind::Solid && !loose {
        if let Some(s) = strain_word(w.st.util.get(i).copied().unwrap_or(0), w.st.pending.get(i).copied().unwrap_or(0) > 0) { l.push(s.into()); }
    }
    let p = w.g.paint(i); if p > 0 && m.kind() == Kind::Solid { l.push(format!("Painted {}.", pigment_name(p))); }
    tip(if loose { format!("Loose {}", m.p().name.to_lowercase()) } else { m.p().name.to_string() }, l)
}

/// A material's description without its sentences about dig tools ("Needs a drill.", "A pickaxe cuts it."):
/// the tip says what the mole's own tools do instead, so a drill owner isn't told granite needs one.
pub fn tool_free(desc: &str) -> String {
    let keep: Vec<&str> = desc.split_inclusive(". ").map(str::trim).filter(|t| { let t = t.to_lowercase(); !["drill", "pickaxe", "bore"].iter().any(|k| t.contains(k)) }).collect();
    if keep.is_empty() { desc.to_string() } else { keep.join(" ") }
}

/// A pigment's name by paint index (1..6), lower case.
pub fn pigment_name(p: u8) -> String { crate::craft::ITEMS.iter().find(|it| it.pigment() == Some(p)).map(|it| it.name().to_lowercase()).unwrap_or_default() }
