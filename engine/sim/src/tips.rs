//! Hover explanations for things in the world: what's under a cell, in plain Delver words.
//! Pure sim so it can be tested headless; the game shows `name` on the first line and `lines` under it.
use crate::craft::Item;
use crate::grid::F_SEEN;
use crate::logic::Dev;
use crate::mat::{Kind, Mat, S, TRACK, TUBE, CPM};
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
/// Order: station, furniture, clue, chest, mushroom plot, transport machine, device, copperline, material.
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
    if let Some(t) = transport_tip(w, x, y) { return Some(t); }
    let b = w.logic.block_of(x, y);
    let d = Dev::from_u8(w.logic.dev[b]);
    if d != Dev::None { return tip(d.name().into(), vec![d.desc().into(), if w.logic.lit(b) { "Burning: too late to pick it up.".into() } else { "E: pick up.".into() }]); }
    if w.torch_at(x, y).is_some() { return tip("Torch".into(), vec![Item::Torch.desc().into(), "E: pick up.".into()]); }
    if w.logic.wire[b] != 0 { return tip("Copperline".into(), vec![Item::CopperWire.desc().into(), if w.logic.net_lit(b) { "Carrying current.".into() } else { "Idle.".into() }, "E: take up (hold and sweep for a run).".into()]); }
    let m = w.g.m(i);
    if matches!(m.kind(), Kind::Empty | Kind::Gas) && m != Mat::Steam && m != Mat::Fire {
        let wl = w.walls.at(x, y);
        if wl != crate::walls::Wall::None {
            return tip(wl.name().into(), vec![wl.desc().into(), "A wall item in hand: left mouse panels, right mouse scrapes.".into()]);
        }
        return None;
    }
    let loose = w.g.loose(i) && m.kind() == Kind::Solid;
    let mut l = vec![tool_free(m.desc())];
    // digging: what the mole's own tools do to it, instead of the table's general note on tools
    if let Some(r) = w.dig_refusal(m) { if m != Mat::Bedrock { l.push(r); } }
    else if w.can_dig(m) { if let Some(it) = crate::craft::drop_of(m) { l.push(format!("Digging it gives {}.", it.name().to_lowercase())); } }
    if m.kind() == Kind::Solid && !loose {
        if let Some(s) = strain_word(w.st.util.get(i).copied().unwrap_or(0), w.st.pending.get(i).copied().unwrap_or(0) > 0) { l.push(s.into()); }
    }
    let p = w.g.paint(i); if p > 0 && m.kind() == Kind::Solid { l.push(format!("Painted {}.", pigment_name(p))); }
    // track and pipe: what makes them go
    if m.has(TRACK) && !loose { l.push(if w.rail_powered(x, y) { "Powered: carts on it are driven along.".into() } else { "Set a mine cart on it; copperline with current along it drives carts.".into() }); }
    if m.has(TUBE) && !loose { l.push("Put tube hoppers on the line: one with current sends, the others receive.".into()); }
    // production: what moves things along it
    if m.has(crate::mat::SCREEN) && !loose { l.push(if w.shaking(x, y) { "Shaking: lumps walk down it and off the end.".into() } else { "Lumps lie on it unless it's steep; copperline current along it shakes it.".into() }); }
    else if m.has(crate::mat::CHUTE) && !loose { l.push("Loose things slide down it where it slopes.".into()); }
    if m == Mat::Water { if let Some((v, _)) = crate::sluice::current(w, x as f32 + 0.5, y as f32 + 0.5) {
        l.push(format!("Running {}, {:.1} m/s: it carries sand, dirt, gravel and coal; ore and stone stay.", if v > 0. { "right" } else { "left" }, v.abs())); } }
    // linkage: pulled or not, and what pulls it
    if m.has(crate::mat::LINK) && !loose && w.gate_at(x, y).is_none() && w.scale_at(x, y).is_none() { let i = y * w.g.w + x;
        l.push(if w.pulled(i) { "Pulled: a plate, scale or float on its linkage is pressed.".into() } else { "Slack: nothing on its linkage is pressed.".into() }); }
    // machinery: how fast its train turns and on what
    if m.has(crate::mat::MECH) && !loose { let i = y * w.g.w + x; let p = &w.tr.power;
        l.push(match p.train.get(&i) { Some(&t) if p.speed.get(t as usize).copied().unwrap_or(0.) > 0.01 => format!("Turning at {:.0}%: {:.0} W to give, {:.0} W to do.", 100. * p.speed[t as usize], p.supply[t as usize], p.demand[t as usize]),
            _ => "Still: nothing turns it. A water wheel in a stream or a counterweight drives a train.".into() }); }
    if m.has(crate::mat::BUMPER) && !loose { l.push("A loaded cart that runs into it tips its load out over it.".into()); }
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

/// Transport machines under (x, y): what it is, its state, and the one or two inputs that matter.
pub fn transport_tip(w: &World, x: usize, y: usize) -> Option<Tip> {
    let tip = |n: &str, l: Vec<String>| Some(Tip { name: n.into(), lines: l });
    if let Some(k) = w.cart_at(x, y) {
        let c = &w.tr.carts[k]; let mut l = vec![];
        if w.tr.riding == Some(k) { l.push("You're in it. Space: jump out · A/D: lean on it.".into()); }
        else { l.push("Right mouse: get in · walk into it to push.".into()); }
        l.push(if c.load() > 0 { format!("Carrying {} items · E: unload them.", c.load()) } else if w.tr.riding == Some(k) { "Empty.".into() } else { "Empty: throw things in to load it · E: pick it up.".into() });
        if !c.on_rail { l.push("Off the rails: it won't roll until it's back on track.".into()); }
        return tip("Mine cart", l);
    }
    if let Some(k) = w.borer_at(x, y) {
        let b = &w.tr.borers[k];
        let on = w.tr.borer_ride == Some(k);
        return tip("Steam borer", vec![format!("Fuel {} lumps{} · water {:.0} of {:.0} · steam {:.0} kJ.", b.lumps(), if b.burning.is_some() { " (lit)" } else { "" }, b.water, crate::borer::TANK, b.steam),
            if b.jammed { "Jammed: no room behind it for its spoil. Clear the tunnel behind it.".into() } else if on { "A/D: drive and bore (it bores the way it faces) · Space: get off.".into() } else { "Coal in paw + right mouse: feed it · empty paw: get on · E: rake out its fuel, then pick it up.".into() }]);
    }
    if let Some(j) = w.wagon_at(x, y) {
        use crate::wagon::WagonKind; let wg = &w.tr.wagons[j]; let inside = w.tr.wagon_ride == Some(j);
        let mut l = vec![if w.in_train(j) { "Coupled: it follows the borer.".to_string() } else if wg.coupled { "Coupled, but nothing pulls it.".into() } else { "Loose: set or shove it up behind a borer to couple it on.".into() }];
        l.push(match wg.kind {
            WagonKind::Store => format!("Holds {} of {} items{}.", wg.load(), crate::wagon::STORE_CAP, if inside { " · right mouse: stow the stack in paw" } else { "" }),
            WagonKind::Coal => format!("{} of {} lumps of fuel; coupled, it stokes the borer while its steam is low.", wg.load(), crate::wagon::COAL_CAP),
            WagonKind::Water => format!("Water {:.0} of {:.0}; coupled, it tops up the borer's tank.", wg.water, crate::wagon::WATER_CAP),
            WagonKind::Bunk => "Climb in to sleep: you'll wake here, and the holt is saved.".into(),
            WagonKind::Workshop => if inside { "You're at its bench: craft as at a workbench.".into() } else { "Climb in to work at its bench.".into() },
        });
        let e = match (wg.load() > 0, wg.coupled) { (true, _) => "E: take its load out", (false, true) => "E: uncouple it", _ => "E: pick it up" };
        l.push(if inside { format!("Space: get out · {e}.") } else { format!("Right mouse: climb in · {e}.") });
        return tip(match wg.kind { WagonKind::Store => "Store wagon", WagonKind::Workshop => "Workshop wagon", WagonKind::Bunk => "Bunk wagon", WagonKind::Water => "Water wagon", WagonKind::Coal => "Coal wagon" }, l);
    }
    if let Some(k) = w.gate_at(x, y) {
        let g = &w.tr.links.gates[k];
        return tip("Sluice gate", vec![format!("{} · it {} when its pull-rod is pulled.", if g.open { "Lifted" } else { "Down" }, if g.invert { "shuts" } else { "lifts" }), "Right mouse: turn it round · E: take it up.".into()]);
    }
    if let Some(k) = w.scale_at(x, y) {
        let s = &w.tr.links.scales[k];
        return tip("Tipping scale", vec![format!("{} stones on its counter-pan: it tips at more than {:.0} kg{}.", s.ballast, s.ballast as f32 * crate::linkage::STONE_KG, if s.tipped { " (tipped)" } else { "" }), "Right mouse: a stone on (stone in paw) or off · E: take it up.".into()]);
    }
    if let Some(k) = w.works_at(x, y) {
        use crate::refine::WorkKind; let wk = &w.tr.works[k];
        let names = |v: &[(crate::craft::Item, u32)]| v.iter().map(|(it, n)| format!("{n} {}", it.name().to_lowercase())).collect::<Vec<_>>().join(", ");
        let held = if wk.input.is_empty() { "Empty".to_string() } else { format!("Holding {}", names(&wk.input)) };
        return match wk.kind {
            WorkKind::Stamp => tip("Stamp mill", vec![format!("{held}: drop ore, stone or slag on it to crush."), "Needs a turning drive train against it; crushed ore comes out as concentrate and sand.".into()]),
            WorkKind::Washer => tip("Washer", vec![format!("{held}: drop crushed ore in to wash it."), "Needs water against it and a turning train: heavy out left, light out right.".into()]),
            WorkKind::Smelter => tip("Smelter", vec![format!("{held} · {:.0} °C, air {:.0}%.", wk.temp, 100. * wk.air),
                if wk.burning.is_some() { "Lit: bellows against it on a turning train make it hot enough (copper and tin 1100 °C, iron 1250 °C).".into() } else { "Cold: drop coal or charcoal in with the ore.".into() },
                format!("Molten: {:.1} copper, {:.1} tin, {:.1} iron (an ingot mould against it is filled at 1).", wk.molten[0], wk.molten[1], wk.molten[2])]),
        };
    }
    if let Some(k) = w.boiler_at(x, y) {
        let b = &w.tr.boilers[k];
        let fire = if b.choked { "Ash door choked: haul the ash and slag away.".to_string() } else if let Some(it) = b.burning { format!("Burning {} ({} more lumps waiting).", it.name().to_lowercase(), b.lumps() - 1) } else { "Cold: drop coal, charcoal or wood on its top.".into() };
        return tip("Boiler", vec![fire, format!("Water {:.0} of {:.0} (it draws from water touching it) · steam {:.0} kJ.", b.water, crate::steam::TANK, b.steam),
            if b.engines.is_empty() { "No engine on it: build a steam engine against it, or pipe its steam to one.".into() } else { format!("Feeds {} engine cells.", b.engines.len()) }]);
    }
    if let Some(k) = w.weight_at(x, y) {
        let wt = &w.tr.power.weights[k]; let down = (wt.y - (wt.drum + 2) as f32) / CPM as f32; let left = (wt.bottom - wt.y) / CPM as f32;
        return tip("Counterweight", vec![format!("{:.0} kg, {:.1} m down its rope ({:.1} m more to fall): it falls to drive its train and is wound up by spare power.", crate::power::WEIGHT_KG, down, left), "E: take it down.".into()]);
    }
    if let Some(k) = w.bin_at(x, y) {
        let b = &w.tr.bins[k];
        let l = vec![format!("Holding {} of {} · fill it from above (items, sand, rubble) · E: empty it.", b.load(), crate::production::BIN_LOAD),
            if b.held { "Gate held open: it empties onto what's under it · right mouse: let go.".into() } else { "A cart under its spout trips the gate and is filled · right mouse: hold it open.".into() }];
        return tip("Ore hopper", l);
    }
    if let Some(k) = w.hopper_at(x, y) {
        let mut l = vec![if w.hopper_blowing(k) { "Blowing: drop items in to send them · right mouse (empty paw): ride.".into() } else { "No current: it only receives. Copperline current runs its blower.".into() }];
        match w.tr.hoppers[k].dest { Some(d) if d < w.tr.hoppers.len() => { let (dx, _) = w.tr.hoppers[d].centre(); let (hx, _) = w.tr.hoppers[k].centre();
                l.push(format!("Sends to the hopper {:.0} m {} · right mouse (paw full): change.", (dx - hx).abs() / CPM as f32, if dx < hx { "left" } else { "right" })); }
            _ => l.push("No other hopper on its line yet.".into()) }
        return tip("Tube hopper", l);
    }
    if let Some(k) = w.lift_at(x, y) { return Some(lift_tip(w, k)); }
    if w.ladder_at(x, y).is_some() {
        return tip("Rope ladder".into(), vec![if w.tr.climbing { "W / S: climb · Space or A / D: let go.".into() } else { "W / S by it: grab on and climb (let go of the keys to hang).".into() }, "E: take it down.".into()]);
    }
    None
}

/// A lift's tip: how it rides, where the cage is, and how it stands for balance.
pub fn lift_tip(w: &World, k: usize) -> Tip {
    let l = &w.tr.lifts[k]; let b = w.lift_balance(k); let n = l.stops.len();
    let mut lines = vec![];
    lines.push(if b.rider { "On the cage: W / S ride to the next stop (hold to run on).".to_string() } else { "Step on and W / S to ride · at a stop, W / S calls the cage.".to_string() });
    lines.push(match l.at_stop() { Some(s) => format!("At stop {} of {n} ({}).", s + 1, if s == 0 { "top" } else if s + 1 == n { "the shaft floor" } else { "a passage" }), None if l.target.is_some() => "On its way.".into(), None => "Held by the brake between stops.".into() });
    lines.push(if b.powered { "Current at the head: the winch hauls any load.".to_string() }
        else if b.over().abs() <= 5. { format!("Balanced: cage {:.0} kg, counterweight {:.0} kg.", b.cage, b.weight) }
        else if b.haulable() { format!("Cage {:.0} kg, counterweight {:.0} kg: light enough to haul.", b.cage, b.weight) }
        else if b.over() > 0. { format!("Too heavy to haul: cage {:.0} kg, {:.0} kg over · right mouse: balance it with stone.", b.cage, b.over()) }
        else { format!("Counterweight {:.0} kg too heavy · right mouse: take stone off.", -b.over()) });
    lines.push("Right mouse: balance · E on the head: take it down.".into());
    Tip { name: "Lift".into(), lines }
}

/// With a lift or a rope ladder in hand, what it would do at the cursor (x, y): where it goes, or why not.
pub fn place_tip(w: &World, it: Item, x: usize, y: usize) -> Option<Tip> {
    let reach = w.in_reach(x, y);
    match it {
        Item::Lift => Some(match w.lift_plan(x, y) {
            Ok(p) => Tip { name: "Lift here".into(), lines: vec![format!("{:.1} m shaft, cage {:.1} m wide, {} stop{}.", (p.bottom - p.head) as f32 / CPM as f32, p.w / CPM as f32, p.stops.len(), if p.stops.len() == 1 { "" } else { "s" }),
                if reach { "Left mouse: hang it under the ceiling.".into() } else { "Out of reach: step closer.".into() }] },
            Err(e) => Tip { name: "No lift here".into(), lines: vec![format!("{}{}.", e[..1].to_uppercase(), &e[1..])] } }),
        Item::RopeLadder => Some(match w.ladder_plan(x, y) {
            Ok(l) => Tip { name: "Rope ladder here".into(), lines: vec![format!("{} {:.1} m to the floor.", if l.ax != l.x { "Tied on the ledge, hangs" } else { "Hangs from the rock above," }, l.len as f32 / CPM as f32),
                if reach { "Left mouse: hang it.".into() } else { "Out of reach: step closer.".into() }] },
            Err(e) => Tip { name: "No ladder here".into(), lines: vec![format!("{}{}.", e[..1].to_uppercase(), &e[1..])] } }),
        _ => None,
    }
}
