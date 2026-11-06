//! Molehill - Bevy front-end. The simulation lives in `molehill-sim`; this crate composites the
//! world into a GPU texture (albedo x per-cell light, fog of war, designations, cracks), draws
//! parallax background layers behind open cells, and syncs sprites for moles, items, chests,
//! stations and torches.
mod art;
mod audio;
mod music;
mod devart;
mod decorart;
mod journal;
mod hotbar; mod itemart; mod props;
mod vitals;
mod wiring;
mod ui;
mod menu;
mod loading;
use molehill_sim::logic::{Dev, DEVS, B as LB};
use molehill_sim::disasters::{Phase, Kind2};
use bevy::prelude::*;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::input::keyboard::Key;
use bevy::window::PrimaryWindow;
use molehill_sim::view as sv;
use molehill_sim::survival::Mode;
use molehill_sim::craft::{self, Item, Station, ITEMS, NITEM, QUESTS, RECIPES, STATIONS};
use molehill_sim::grid::{hash3, F_SEEN};
use molehill_sim::mat::{Kind, Mat, PROPS, CPM, S, SF};
use molehill_sim::sites::Kind as SiteKind;
use molehill_sim::walls::Wall;
use molehill_sim::world::{self as sw, World};

// The window is 256 × 128 m at any resolution; a cell is CPX world units (4 per legacy 0.25 m tile), so
// sprites, zoom and the UI keep their size in metres.
/// Seconds of holding `Q` for a full-power throw.
const THROW_CHARGE: f32 = 0.8;
const W: usize = 1024 * S; const H: usize = 512 * S; const CPX: f32 = 4.0 / SF; // world units per cell
const PAD: usize = 512 * S;
const CH: usize = 64 * S; // texture chunk size in cells (16 m)
/// The parallax layers are drawn at legacy-tile resolution (one pixel per 0.25 m): they're blurred far rock.
const BG_RES: usize = S;
const FAR: usize = 4; // chunk rows drawn beyond each end of the simulated window (the camera roams there; nothing simulates)

#[derive(Resource)] pub struct Game { pub w: World, /// a holt is in play (false over the empty window before the first one)
    pub live: bool, chunks: Vec<Handle<Image>>, buf: Vec<u8>, pub uploads: usize, acc: f32, pub paused: bool, speed: u32, pub frame: u64, last_light: u64, msg: String, pub help: bool, ev: Vec<sw::Event>, shake_off: Vec2, autosave: f32, last_oy: i64, far: Vec<Handle<Image>>, far_done: Vec<bool>, far_cells: std::collections::HashMap<i64, Option<(Vec<u8>, Vec<u8>)>>, pub inv_open: bool, pub aim: Option<(usize, usize, bool)>,
    pub slots: Vec<Option<Thing>>, pub held: Option<Thing>, /** lay the hotbar out afresh for this holt (`hotbar.rs`) */ pub relayout: bool, pub hot: usize, pub sel_t: f32, pub toasts: Vec<(String, f32)>, log_seen: u64, pub show_dev: bool, disp: Vec2, pub seed: u32,
    /// brush size per brush item and grid snap (`view::BrushSizes`)
    pub sizes: sv::BrushSizes,
    /// grid snap: where the current drag started (a held left mouse lays a straight line from here)
    drag: Option<(usize, usize)>,
    /// the size written beside the brush outline, and the outline's radius on screen (px)
    pub brush_tag: Option<(String, f32)>,
    /// dev only (`--shot-walk`): hold right as if `D` were down
    pub demo_right: bool,
    /// wheel → notches (see `view::Wheel`: an idle wheel is still, a trackpad flick is one notch)
    wheel: molehill_sim::view::Wheel,
    /// mouse buttons held since a menu closed: ignored until released (the click that closed the menu must not dig or place)
    pub mouse_latch: bool,
    /// a jump pressed since the last sim step (kept for that step even if already released, so a quick tap
    /// between two 60 Hz steps on a faster display isn't lost); `jump_key` = the jump keys' held state
    jump_tap: bool, jump_key: bool,
    /// `W` is acting as jump: pressed fresh while not clinging, and still held
    w_jump: bool,
    /// `Q` held: seconds of throw charge so far, and the aim (cells, from the mole's chest) for the arc preview
    charge: Option<f32>, throw_aim: (f32, f32),
    /// E closed the pack: it doesn't also pick up until released
    e_latch: bool,
    /// in-world feedback pops: (kind 0 bite / 1 placed block, x, y, radius in cells, start time s)
    pub pops: Vec<(u8, f32, f32, f32, f32)>,
    /// smoothed camera lookahead (cells)
    look: Vec2 }
/// What an inventory slot holds. Counts come from the shared pack (`World::inv`).
#[derive(Clone, Copy, PartialEq, Debug)] pub enum Thing { Pick(u8), Item(Item), Dev(usize), Station(usize) }
pub const NSLOT: usize = 100; // 10 hotbar + 90 pack: room for the six back-wall items after devices and stations
/// The round brush an inventory thing uses, if any (dig tools and placeable blocks).
pub fn brush_of(t: Option<Thing>) -> Option<sw::Brush> { match t { Some(Thing::Pick(n)) => Some(sw::Brush::Dig(n)), Some(Thing::Item(it)) if sw::block_code(it).is_some() || it.pigment().is_some() || it.is_wall() => Some(sw::Brush::Place(it)), _ => None } }
/// The radius the brush of this thing will actually use (the player's choice, clamped to the item's range).
pub fn brush_r(g: &Game, b: sw::Brush) -> f32 { g.sizes.radius(b) }
pub fn thing_name(g: &Game, t: Thing) -> String { match t { Thing::Pick(_) if !thing_usable(g, t) => "Claws".into(), Thing::Pick(n) => ["Pickaxe", "Drill", "Bore"][(n as usize - 1).min(2)].into(), Thing::Item(it) => it.name().into(), Thing::Dev(d) => DEVS[d].name().into(), Thing::Station(s) => STATIONS[s].name().into() } }
/// Tooltip line for an inventory thing.
pub fn thing_desc(t: Thing) -> &'static str { match t { Thing::Pick(1) => "Breaks soil and soft stone in a mole-sized bite.", Thing::Pick(2) => "Copperline-driven auger. Cuts granite, basalt and ore.", Thing::Pick(_) => "Heavy heat-bore. The widest bite; cuts the hardest stone.", Thing::Item(it) => it.desc(), Thing::Dev(d) => DEVS[d].desc(), Thing::Station(s) => STATIONS[s].desc() } }
/// The inputs for a thing in hand, one line (slot tooltip and the hint over the hotbar).
pub fn thing_controls(t: Thing) -> &'static str { match t {
    Thing::Pick(_) => "Left mouse: dig · S: dig down",
    Thing::Item(it) => it.controls(),
    Thing::Dev(d) if DEVS[d] == Dev::Tnt => Item::Tnt.controls(),
    Thing::Dev(_) => "Left/right mouse: set down · right mouse on it: use · E: pick up",
    Thing::Station(_) => "Left mouse: plan it here, then bring the materials",
} }
/// Crafting list (pack open): for each station the mole stands at (Creative: every station, free), a header
/// with the station's state, then the recipes known so far with their inputs. Clicking a bright row crafts it.
pub struct CraftRow { pub text: String, pub recipe: Option<usize>, pub ok: bool, pub head: bool }
pub const CRAFT_ROWS: usize = 30;
const CRAFT_X: f32 = 18.; const CRAFT_Y: f32 = 96.; const CRAFT_H: f32 = 19.; const CRAFT_W: f32 = 380.;
pub fn craft_row_pos(k: usize) -> Vec2 { Vec2::new(CRAFT_X, CRAFT_Y + k as f32 * CRAFT_H) }
pub fn craft_rows(g: &Game) -> Vec<CraftRow> {
    let w = &g.w; let mut v = vec![]; if !g.inv_open { return v; } let free = !w.rules.crafting;
    for st in STATIONS {
        let here = w.station_here(st); if !free && here.is_none() { continue; }
        let ready = if free { Ok(()) } else { here.map_or(Ok(()), |b| w.station_ready(b)) };
        let burns = !free && st == Station::Forge && here.map_or(false, |b| !w.forge_hot(b));
        v.push(CraftRow { text: match ready { Ok(()) if burns => "Forge: burns a coal per job (lava or fire near keeps it hot)".into(), Ok(()) => st.name().to_string(), Err(e) => format!("{}: {e}", st.name()) }, recipe: None, ok: ready.is_ok(), head: true });
        for (r, rec) in RECIPES.iter().enumerate() {
            if rec.station != st || !w.recipe_known(r) { continue; }
            let has = rec.inputs.iter().all(|&(it, n)| w.inv[it as usize] >= n);
            let ins: Vec<String> = rec.inputs.iter().map(|&(it, n)| format!("{n} {}", it.name().to_lowercase())).collect();
            v.push(CraftRow { text: format!("  {}   {}", rec.name, ins.join(", ")), recipe: Some(r), ok: has && ready.is_ok(), head: false });
        }
    }
    if v.is_empty() { v.push(CraftRow { text: "Stand at a workbench, forge or workshop to craft".into(), recipe: None, ok: false, head: true }); }
    v.truncate(CRAFT_ROWS); v
}
pub fn craft_row_at(g: &Game, c: Vec2) -> Option<usize> {
    if !g.inv_open || c.x < CRAFT_X || c.x > CRAFT_X + CRAFT_W || c.y < CRAFT_Y { return None; }
    let k = ((c.y - CRAFT_Y) / CRAFT_H) as usize; (k < craft_rows(g).len()).then_some(k)
}
/// In Survival a dig tool is only in the hotbar once you own it (Creative: always).
pub fn thing_usable(g: &Game, t: Thing) -> bool { match t { Thing::Pick(n) => !g.w.rules.tools || g.w.inv[[Item::Pickaxe, Item::Drill, Item::Bore][(n as usize).clamp(1, 3) - 1] as usize] > 0, _ => true } }
pub fn thing_count(g: &Game, t: Thing) -> Option<u32> { match t { Thing::Item(it) => Some(g.w.inv[it as usize]), Thing::Dev(d) => dev_cost(DEVS[d]).map(|it| g.w.inv[it as usize]), _ => None } }
pub(crate) fn default_slots() -> Vec<Option<Thing>> {
    let mut v: Vec<Thing> = vec![Thing::Pick(1), Thing::Pick(2), Thing::Pick(3), Thing::Item(Item::WoodPillar), Thing::Item(Item::Brick), Thing::Item(Item::IronBeam), Thing::Item(Item::Torch), Thing::Item(Item::Tnt), Thing::Item(Item::CopperWire), Thing::Dev(DEVS.iter().position(|&d| d == Dev::Lever).unwrap_or(0))];
    for it in [Item::RockWall, Item::TimberWall, Item::MasonryWall, Item::BeamWall, Item::PipeWall, Item::CarvedWall] { v.push(Thing::Item(it)); }
    v.push(Thing::Item(Item::Spore));
    for d in 0..DEVS.len() { if !v.contains(&Thing::Dev(d)) { v.push(Thing::Dev(d)); } }
    for k in 0..STATIONS.len() { v.push(Thing::Station(k)); }
    for &it in ITEMS.iter() { if !v.contains(&Thing::Item(it)) && !matches!(it, Item::Pickaxe | Item::Drill | Item::Bore) { v.push(Thing::Item(it)); } }
    let mut out: Vec<Option<Thing>> = v.into_iter().map(Some).collect(); out.resize(NSLOT, None); out.truncate(NSLOT); out
}
/// Parallax background: three deep-earth layers (pixels, parallax factor). They are drawn as GPU sprites
/// (`BgLayer`) behind the world and slide smoothly with the camera; open cells in the world texture are a
/// translucent light/air overlay on top of them. Replacing the resource re-uploads the layer images.
#[derive(Resource)] struct Bg { layers: Vec<(Vec<u8>, f32)> }
#[derive(Component)] pub struct BgLayer(usize);
const BG_BASE: [f32; 3] = [44., 34., 30.];
#[inline] fn lin(c: f32) -> f32 { (c / 255.).max(0.).powf(2.2) }
#[inline] fn srgb(l: f32) -> f32 { l.max(0.).powf(1. / 2.2) * 255. }
#[derive(Component)] pub struct WorldSprite;
#[derive(Component)] struct Pooled(u8, usize);
#[derive(Resource)] pub struct Sprites { pub rope: [Handle<Image>; 2], pub props: Vec<Handle<Image>>, pub cart: Handle<Image>, pub bin: Vec<Handle<Image>>, pub clue: Vec<Handle<Image>>, pub furn: Vec<Option<(Handle<Image>, Option<Handle<Image>>)>>, pub tools: [Handle<Image>; 3], pub dev: Vec<Handle<Image>>, pub ring: Handle<Image>, pub mole: Vec<Handle<Image>>, pub mole_out: Vec<Handle<Image>>, pub held: Vec<Handle<Image>>, pub icons: Vec<Handle<Image>>, pub chest: [Handle<Image>; 2], pub station: Vec<[Handle<Image>; 2]>, pub torch: Handle<Image>, pub torch_f: Vec<Handle<Image>>, pub glow: Handle<Image>, pub shroom: [Handle<Image>; 3], pub px: Handle<Image> }

/// `?seed=N` (web) / `--seed=N` (native) was given: the game opens straight into that holt.
fn seed_pinned() -> bool {
    #[cfg(target_arch = "wasm32")] { web_sys::window().and_then(|w| w.location().search().ok()).is_some_and(|q| q.split(['?', '&']).any(|kv| kv.starts_with("seed="))) }
    #[cfg(not(target_arch = "wasm32"))] { std::env::args().any(|a| a.starts_with("--seed=")) }
}
pub fn seed_now() -> u32 {
    // ?seed=N (web) / --seed=N (native) pins the first world
    #[cfg(target_arch = "wasm32")] { if let Some(v) = web_sys::window().and_then(|w| w.location().search().ok()).and_then(|q| q.split(['?', '&']).find_map(|kv| kv.strip_prefix("seed=").and_then(|v| v.parse::<u32>().ok()))) { return v; } }
    #[cfg(not(target_arch = "wasm32"))] { if let Some(v) = std::env::args().find_map(|a| a.strip_prefix("--seed=").and_then(|v| v.parse::<u32>().ok())) { return v; } }
    #[cfg(target_arch = "wasm32")] { (js_sys::Date::now() as u64 % 1_000_000_007) as u32 }
    #[cfg(not(target_arch = "wasm32"))] { (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() % 1_000_000_007) as u32 }
}
fn fresh_requested() -> bool {
    #[cfg(target_arch = "wasm32")] { web_sys::window().and_then(|w| w.location().search().ok()).map(|q| q.contains("fresh")).unwrap_or(false) }
    #[cfg(not(target_arch = "wasm32"))] { std::env::args().any(|a| a == "--fresh") }
}
/// Save slots: localStorage (base64) on the web, files next to the binary natively.
pub fn storage_get(slot: &str) -> Option<Vec<u8>> {
    #[cfg(target_arch = "wasm32")] { let st = web_sys::window()?.local_storage().ok()??; let s = st.get_item(slot).ok()??; molehill_sim::save::from_b64(&s).ok() }
    #[cfg(not(target_arch = "wasm32"))] { std::fs::read(format!("{slot}.save")).ok() }
}
pub fn storage_put(slot: &str, b: &[u8]) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")] { let st = web_sys::window().and_then(|w| w.local_storage().ok().flatten()).ok_or("no localStorage")?; st.set_item(slot, &molehill_sim::save::to_b64(b)).map_err(|_| "storage full".to_string()) }
    #[cfg(not(target_arch = "wasm32"))] { let f = format!("{slot}.save"); if let Some(d) = std::path::Path::new(&f).parent() { let _ = std::fs::create_dir_all(d); } std::fs::write(f, b).map_err(|e| e.to_string()) }
}
fn storage_del(key: &str) {
    #[cfg(target_arch = "wasm32")] { if let Some(st) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) { let _ = st.remove_item(key); } }
    #[cfg(not(target_arch = "wasm32"))] { let _ = std::fs::remove_file(format!("{key}.save")); }
}
/// The band keys stored under a slot (`slot/<band key>`).
fn storage_bands(slot: &str) -> Vec<String> {
    #[cfg(target_arch = "wasm32")] { let Some(st) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else { return vec![] };
        let pre = format!("{slot}/"); (0..st.length().unwrap_or(0)).filter_map(|i| st.key(i).ok().flatten()).filter(|k| k.starts_with(&pre)).collect() }
    #[cfg(not(target_arch = "wasm32"))] { std::fs::read_dir(slot).map(|d| d.filter_map(|e| e.ok()?.file_name().into_string().ok()?.strip_suffix(".save").map(|k| format!("{slot}/{k}"))).collect()).unwrap_or_default() }
}
/// Save the world into a slot as bands (see `sim::band`): only bands not stored yet are written, then
/// the manifest, then bands no longer named are removed. A failed write leaves the previous save whole.
pub fn save_slot(slot: &str, w: &World) -> Result<(), String> {
    let p = molehill_sim::save::save_parts(w);
    let have: std::collections::HashSet<String> = storage_bands(slot).into_iter().collect(); let mut live = std::collections::HashSet::new();
    for (k, b) in &p.bands { let key = format!("{slot}/{k}"); if !have.contains(&key) { storage_put(&key, b)?; } live.insert(key); }
    let mut m = p.manifest; m.push(0); storage_put(slot, &m)?;
    for k in have.difference(&live) { storage_del(k); }
    Ok(())
}
pub fn load_slot(slot: &str) -> Option<Result<World, String>> {
    let m = storage_get(slot)?; Some(molehill_sim::save::load_with(&m, &mut |k| storage_get(&format!("{slot}/{k}"))))
}
fn cell_to_world(x: f32, y: f32) -> Vec2 { Vec2::new(x * CPX - W as f32 * CPX / 2., H as f32 * CPX / 2. - y * CPX) }
fn world_to_cell(p: Vec2) -> (i32, i32) { (((p.x + W as f32 * CPX / 2.) / CPX).floor() as i32, ((H as f32 * CPX / 2. - p.y) / CPX).floor() as i32) }

fn main() {
    #[cfg(target_arch = "wasm32")] std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(Window {
            title: "Molehill".into(), canvas: Some("#bevy".into()), fit_canvas_to_parent: true, prevent_default_event_handling: true,
            // --shot: a fixed frame so each family fills the still; play keeps the host default
            resolution: if cfg!(not(target_arch = "wasm32")) && std::env::args().any(|a| a.starts_with("--shot=")) { (1440u32, 900u32).into() } else { Default::default() },
            ..default()
        }), ..default() }))
        .insert_resource(ClearColor(Color::srgb(0.04, 0.035, 0.05)))
        .add_systems(Startup, setup)
        .insert_non_send(audio::Audio::new())
        .insert_resource(loading::Loading::default())
        .add_systems(Update, (dev_shot, loading::update, menu::update, input, gear, simulate, render_world, dev_dump, sync_sprites, wiring::update, journal::update, hotbar::update, ui::update, sound).chain())
        .run();
}

/// Dev visual check (native): `--shot=FILE` starts a holt straight away, saves one frame after
/// `--shot-at=N` frames (default 150) and quits. `--shot-ui=pause|inv|help|dev` opens that panel first (`loading`: the still is of a new holt's loading line),
/// `--shot-walk` holds right, `--shot-blast=K` sets off a charge beside the mole K frames before the shot.
/// One still frame only: no videos.
fn dev_shot(mut n: Local<u32>, mut g: ResMut<Game>, mut m: ResMut<menu::Menu>, mut commands: Commands, mut exit: MessageWriter<AppExit>) {
    #[cfg(not(target_arch = "wasm32"))] {
        let arg = |k: &str| std::env::args().find_map(|a| a.strip_prefix(k).map(|v| v.to_string()));
        let Some(path) = arg("--shot=") else { return };
        let at: u32 = arg("--shot-at=").and_then(|v| v.parse().ok()).unwrap_or(150); let ui = arg("--shot-ui=").unwrap_or_default();
        *n += 1;
        if *n == 2 { m.go(menu::Screen::Off); g.paused = false; g.demo_right = std::env::args().any(|a| a == "--shot-walk"); }
        if *n == at.saturating_sub(5) { match ui.as_str() { "pause" => { m.go(menu::Screen::Pause); g.paused = true; } "inv" => g.inv_open = true, "help" => g.help = true, "dev" => g.show_dev = true, _ => {} } }
        // `--shot-blast=K`: a charge goes off 3 m beside the mole K frames before the shot (to see the front and dust)
        if let Some(k) = arg("--shot-blast=").and_then(|v| v.parse::<u32>().ok()) { if *n == at.saturating_sub(k) { if let Some(mo) = g.w.moles.first().cloned() {
            let c = molehill_sim::mat::CPM; let x = (mo.x + 3 * c).min(W - 1); let y = mo.y.saturating_sub(c / 2); g.w.blast(x, y, molehill_sim::blast::TNT_KG); } } }
        if *n == at { commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(move |c: On<bevy::render::view::screenshot::ScreenshotCaptured>| write_bmp(&path, &c.image)); }
        if *n == at + 40 { exit.write(AppExit::Success); }
    }
    #[cfg(target_arch = "wasm32")] { let _ = (&mut n, &mut g, &mut m, &mut commands, &mut exit); }
}
/// 24-bit BMP writer for the dev flags (no image codecs are compiled in). `bgra` swaps channels; pixels are
/// composited over a dark grey by alpha and scaled up `k`× with nearest sampling.
#[cfg(not(target_arch = "wasm32"))]
fn bmp(path: &str, w: usize, h: usize, d: &[u8], bgra: bool, k: usize) {
    let (ow, oh) = (w * k, h * k); let row = (ow * 3 + 3) & !3; let size = 54 + row * oh; let mut b = Vec::with_capacity(size);
    let le32 = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
    b.extend_from_slice(b"BM"); le32(&mut b, size as u32); le32(&mut b, 0); le32(&mut b, 54); le32(&mut b, 40); le32(&mut b, ow as u32); le32(&mut b, oh as u32);
    b.extend_from_slice(&1u16.to_le_bytes()); b.extend_from_slice(&24u16.to_le_bytes()); for _ in 0..6 { le32(&mut b, 0); }
    for oy in (0..oh).rev() { for ox in 0..ow { let o = ((oy / k) * w + ox / k) * 4; let a = d[o + 3] as u32;
        let (r, g, bl) = if bgra { (d[o + 2], d[o + 1], d[o]) } else { (d[o], d[o + 1], d[o + 2]) };
        let m = |c: u8| ((c as u32 * a + 40 * (255 - a)) / 255) as u8; b.extend_from_slice(&[m(bl), m(g), m(r)]); } for _ in 0..row - ow * 3 { b.push(0); } }
    let _ = std::fs::write(path, b);
}
#[cfg(not(target_arch = "wasm32"))]
fn write_bmp(path: &str, im: &Image) {
    let Some(d) = im.data.as_ref() else { return };
    let bgra = matches!(im.texture_descriptor.format, bevy::render::render_resource::TextureFormat::Bgra8UnormSrgb | bevy::render::render_resource::TextureFormat::Bgra8Unorm);
    bmp(path, im.width() as usize, im.height() as usize, d, bgra, 1);
}
/// Dev (native): with `--shot`, `--dump-art=DIR` also writes the composited world around the mole
/// with every visible world sprite composited on (`view.bmp`; `view_cells.bmp` is the cells alone) and every sprite (`sprite-*.bmp`, 8×).
fn dev_dump(n: Local<u32>, g: Res<Game>, sp: Res<Sprites>, images: Res<Assets<Image>>, mut done: Local<bool>, bg: Res<Bg>, cam: Query<&Transform, With<Camera2d>>, wires: Query<(&Transform, &Sprite, &Visibility), Or<(With<wiring::WireSpr>, With<Pooled>)>>) {
    #[cfg(not(target_arch = "wasm32"))] {
        let _ = &n; if *done { return; }
        let arg = |k: &str| std::env::args().find_map(|a| a.strip_prefix(k).map(|v| v.to_string()));
        let Some(dir) = arg("--dump-art=") else { return }; let at: u64 = arg("--shot-at=").and_then(|v| v.parse().ok()).unwrap_or(150);
        if g.frame < at { return; } *done = true; let _ = std::fs::create_dir_all(&dir);
        let (mx, my) = (g.w.moles[0].x, g.w.moles[0].y);
        // site / wall-edit stamps are larger than the start holt; dump a 60 × 40 m crop so the room fits
        let site_dump = scene_name().is_some_and(|s| matches!(s.as_str(),
            "mine" | "holt" | "pump" | "pumpworks" | "fissure" | "fossil" | "crystal" | "salt" | "halt"
            | "site-mine" | "site-holt" | "site-pump" | "site-fissure" | "site-fossil" | "site-crystal" | "site-halt"
            | "walls" | "wall"));
        let (cw, chh, x0, y0) = if let Some(st) = g.w.sites.last().filter(|_| site_dump) { // only a stamped scene crops to its site; a plain holt crops round the mole
            let pad = 8 * CPM;
            let yw0 = (st.y0 - g.w.oy).max(0) as usize;
            let cw = (st.w + 2 * pad).min(W);
            let chh = (st.h + 2 * pad).min(H);
            let x0 = st.x0.saturating_sub(pad).min(W.saturating_sub(cw));
            let y0 = yw0.saturating_sub(pad).min(H.saturating_sub(chh));
            (cw, chh, x0, y0)
        } else {
            let (cw, chh) = if site_dump { (240 * S, 160 * S) } else { (160 * S, 90 * S) };
            (cw, chh, mx.saturating_sub(cw / 2).min(W - cw), my.saturating_sub(chh / 2).min(H - chh))
        };
        let mut v = vec![0u8; cw * chh * 4]; for y in 0..chh { let s = ((y0 + y) * W + x0) * 4; v[y * cw * 4..(y + 1) * cw * 4].copy_from_slice(&g.buf[s..s + cw * 4]); }
        // composite the translucent open cells over the parallax layers like the GPU does (linear blend)
        let camx = cam.single().map(|t| t.translation.x / CPX).unwrap_or(0.); let bw = W + 2 * PAD;
        for y in 0..chh { for x in 0..cw { let o = (y * cw + x) * 4; let a = v[o + 3] as f32 / 255.; if a >= 1. { continue; }
            let mut c = BG_BASE;
            for (ly, f) in &bg.layers { let u = (((x0 + x) as f32 - camx * f + PAD as f32) / BG_RES as f32).floor().clamp(0., (bw / BG_RES) as f32 - 1.) as usize; let q = (((y0 + y) / BG_RES) * (bw / BG_RES) + u) * 4; let la = ly[q + 3] as f32 / 255.;
                for j in 0..3 { c[j] = c[j] + (ly[q + j] as f32 - c[j]) * la; } }
            for j in 0..3 { v[o + j] = srgb(lin(c[j]) * (1. - a) + lin(v[o + j] as f32) * a).min(255.) as u8; } v[o + 3] = 255; } }
        // mark the mole's body cells so the crop can be read without sprites
        for dy in 0..3 * S { let (x, y) = (mx - x0, my - dy - y0); let o = (y * cw + x) * 4; v[o..o + 4].copy_from_slice(&[255, 0, 255, 255]); }
        bmp(&format!("{dir}/view_cells.bmp"), cw, chh, &v, false, 4 / S.min(4));
        // view.bmp: the same at one pixel per world unit with every visible world sprite blitted on in depth
        // order (furniture, found-place props, chests, torches, carts, the mole, copperline, ropes); rotated ones are sampled back through their rotation
        { let k = (1. / CPX).recip() as usize; let (ww, wh) = (cw * k, chh * k); let mut u = vec![0u8; ww * wh * 4];
            for y in 0..wh { for x in 0..ww { let s = ((y / k) * cw + x / k) * 4; u[(y * ww + x) * 4..(y * ww + x) * 4 + 4].copy_from_slice(&v[s..s + 4]); } }
            let mut list: Vec<(&Transform, &Sprite)> = wires.iter().filter(|(_, _, v)| **v != Visibility::Hidden).map(|(t, s, _)| (t, s)).collect(); list.sort_by(|a, b| a.0.translation.z.partial_cmp(&b.0.translation.z).unwrap());
            for (t, spr) in list { let Some(im) = images.get(&spr.image) else { continue }; let Some(d) = im.data.as_ref() else { continue };
                let (iw, ih) = (im.width() as usize, im.height() as usize); let size = spr.custom_size.unwrap_or(Vec2::new(iw as f32, ih as f32));
                let c = spr.color.to_srgba(); let cell = |wx: f32, wy: f32| (((wx + W as f32 * CPX / 2.) / CPX - x0 as f32) * k as f32, ((H as f32 * CPX / 2. - wy) / CPX - y0 as f32) * k as f32);
                // walk the output pixels over the sprite's (rotated) bounds and sample it back through the inverse rotation
                let inv = t.rotation.inverse(); let r = size.length() / 2. + 1.; let (cx, cy) = cell(t.translation.x, t.translation.y);
                for oy in (cy - r).floor() as i64..=(cy + r).ceil() as i64 { for ox in (cx - r).floor() as i64..=(cx + r).ceil() as i64 { if ox < 0 || oy < 0 || ox >= ww as i64 || oy >= wh as i64 { continue; }
                    let l = inv * Vec3::new(ox as f32 + 0.5 - cx, cy - (oy as f32 + 0.5), 0.); let (lx, ly) = (l.x + size.x / 2., size.y / 2. - l.y);
                    if lx < 0. || ly < 0. || lx >= size.x || ly >= size.y { continue; }
                    let (sx, sy) = (((lx / size.x) * iw as f32) as usize, ((ly / size.y) * ih as f32) as usize); let (sx, sy) = (sx.min(iw - 1), sy.min(ih - 1)); let sx = if spr.flip_x { iw - 1 - sx } else { sx }; let q = (sy * iw + sx) * 4; let a = d[q + 3] as f32 / 255. * c.alpha; if a <= 0. { continue; }
                    let o = (oy as usize * ww + ox as usize) * 4; let tint = [c.red, c.green, c.blue];
                    for j in 0..3 { u[o + j] = (u[o + j] as f32 * (1. - a) + d[q + j] as f32 * tint[j] * a).min(255.) as u8; } } } }
            bmp(&format!("{dir}/view.bmp"), ww, wh, &u, false, 1); }
        let mut list: Vec<(String, Handle<Image>)> = vec![("torch".into(), sp.torch_f[0].clone()), ("torch1".into(), sp.torch_f[1].clone()), ("glow".into(), sp.glow.clone()), ("ring".into(), sp.ring.clone())];
        for (k, h) in sp.dev.iter().enumerate() { list.push((format!("dev-{:?}", DEVS[k]), h.clone())); }
        for (k, h) in sp.mole.iter().enumerate() { list.push((format!("mole{k}"), h.clone())); }
        for (k, h) in sp.mole_out.iter().enumerate() { list.push((format!("moleout{k}"), h.clone())); }
        for (k, h) in sp.held.iter().enumerate() { list.push((format!("held{k}"), h.clone())); }
        for (k, f) in sp.furn.iter().enumerate() { if let Some((b, c)) = f { list.push((format!("furn-{:?}", ITEMS[k]), b.clone())); if let Some(c) = c { list.push((format!("cloth-{:?}", ITEMS[k]), c.clone())); } } }
        for (k, h) in sp.tools.iter().enumerate() { list.push((format!("tool{k}"), h.clone())); }
        list.push(("cart".into(), sp.cart.clone())); list.push(("rope".into(), sp.rope[0].clone())); list.push(("hook".into(), sp.rope[1].clone()));
        for (k, h) in sp.bin.iter().enumerate() { list.push((format!("orebin{k}"), h.clone())); }
        for (k, h) in sp.props.iter().enumerate() { list.push((format!("prop-{:?}", props::PROPS[k]), h.clone())); }
        for (k, h) in sp.icons.iter().enumerate() { list.push((format!("icon-{:?}", ITEMS[k]), h.clone())); }
        for (k, h) in sp.station.iter().enumerate() { list.push((format!("station{k}"), h[1].clone())); }
        for (k, h) in sp.shroom.iter().enumerate() { list.push((format!("shroom{k}"), h.clone())); }
        for (k, h) in sp.chest.iter().enumerate() { list.push((format!("chest{k}"), h.clone())); }
        for (name, h) in list { if let Some(im) = images.get(&h) { if let Some(d) = im.data.as_ref() { bmp(&format!("{dir}/sprite-{name}.bmp"), im.width() as usize, im.height() as usize, d, false, 8); } } }
        info!("dumped art to {dir}");
    }
}
/// Dev (native): `--dump-art=` was given.
fn dumping() -> bool {
    #[cfg(not(target_arch = "wasm32"))] { static D: std::sync::OnceLock<bool> = std::sync::OnceLock::new(); *D.get_or_init(|| std::env::args().any(|a| a.starts_with("--dump-art="))) }
    #[cfg(target_arch = "wasm32")] { false }
}
fn make_world(seed: u32, mode: Mode) -> World { let mut w = World::new(W, H, seed); w.disasters.enabled = false; w.set_mode(scene_mode(mode)); stage_scene(&mut w); w }
fn scene_name() -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))] { std::env::args().find_map(|a| a.strip_prefix("--scene=").map(|s| s.to_string())) }
    #[cfg(target_arch = "wasm32")] { None }
}
/// Wall-edit inspection needs every lining in the pack; the other scenes keep the mode they were started in.
fn scene_mode(mode: Mode) -> Mode {
    match scene_name().as_deref() { Some("walls") | Some("wall") => Mode::Creative, _ => mode }
}
/// `--shot-zoom=`; site stamps pull the camera back so a chunk-scale halt or fissure fits the still.
fn shot_or_menu_zoom() -> f32 {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(z) = std::env::args().find_map(|a| a.strip_prefix("--shot-zoom=").and_then(|v| v.parse().ok())) {
        return molehill_sim::view::clamp_zoom(z);
    }
    molehill_sim::view::clamp_zoom(match scene_name().as_deref() {
        Some("fissure") | Some("site-fissure") => 0.52,
        Some("halt") | Some("site-halt") => 0.62,
        Some("mine") | Some("site-mine") => 0.58,
        Some("pump") | Some("pumpworks") | Some("site-pump") => 0.50,
        Some("holt") | Some("site-holt") => 0.36,
        Some("fossil") | Some("site-fossil") => 0.44,
        Some("crystal") | Some("salt") | Some("site-crystal") => 0.40,
        Some("walls") | Some("wall") => 0.32,
        _ => crate::menu::Menu::load().zoom,
    })
}
/// --scene=water|tnt|lore|decor|wiring, or a found-place / wall-edit stamp in host rock (native). Play is unchanged.
fn stage_scene(w: &mut World) {
    #[cfg(not(target_arch = "wasm32"))] {
        let Some(sc) = scene_name() else { return };
        let (mx, my) = (w.moles[0].x, w.moles[0].y); let set = |w: &mut World, x: usize, y: usize, m: Mat| { let i = w.g.idx(x, y); w.g.set(i, m); };
        w.disasters.enabled = false;
        match sc.as_str() {
            "water" => { // an open tunnel east out of the burrow ending in clay, with a water pocket behind it
                for x in mx + 4 * S..mx + 30 * S { for y in my - 2 * S..=my { set(w, x, y, Mat::Empty); } set(w, x, my + 1 * S, Mat::Clay); set(w, x, my - 3 * S, Mat::Limestone); }
                for x in mx + 30 * S..mx + 34 * S { for y in my - 5 * S..=my + 1 * S { set(w, x, y, Mat::Clay); } }
                for x in mx + 34 * S..mx + 50 * S { for y in my - 12 * S..=my + 1 * S { set(w, x, y, Mat::Water); } set(w, x, my + 2 * S, Mat::Clay); }
                molehill_sim::light::reveal_box(&mut w.g, mx, my - 20 * S, mx + 60 * S, my + 10 * S);
            }
            "tnt" => { // a long tunnel under a sand bed
                for x in mx + 4 * S..mx + 60 * S { for y in my - 2 * S..=my { set(w, x, y, Mat::Empty); } set(w, x, my + 1 * S, Mat::Limestone); for y in my - 6 * S..my - 2 * S { set(w, x, y, Mat::Dirt); } for y in my - 16 * S..my - 6 * S { set(w, x, y, Mat::Sand); } }
                w.inv[Item::Tnt as usize] = 3; w.inv[Item::Torch as usize] = 12;
                molehill_sim::light::reveal_box(&mut w.g, mx, my - 24 * S, mx + 70 * S, my + 10 * S);
                for k in 0..4 { w.torches.push((mx + 22 * S + k * 10 * S, my - 2 * S)); }
            }
            "lore" => { // a tunnel east of the burrow with one of every Lid clue to walk past
                use molehill_sim::lore::{Clue, ClueKind};
                let (x0, x1) = (mx + 4 * S, mx + 60 * S);
                for x in x0..x1 { for y in my - 6 * S..=my { set(w, x, y, Mat::Empty); } }
                for x in x0 + 44 * S..x0 + 47 * S { for y in my - 30 * S..my - 6 * S { set(w, x, y, Mat::Empty); } } // a shaft going up: the draft
                molehill_sim::light::reveal_box(&mut w.g, mx, my - 40 * S, x1 + 10 * S, my + 10 * S);
                let oy = w.oy; let mut put = |w: &mut World, kind: ClueKind, x: usize, y: usize, look: u8| w.clues.push(Clue { kind, x, y, wy: oy + y as i64, found: false, look });
                put(w, ClueKind::Carving, x1 - 1, my - 3 * S, 0); put(w, ClueKind::Carving, x0 + 30 * S, my - 7 * S + 1, 1);
                put(w, ClueKind::Note, x0 + 8 * S, my, 0);
                for k in 0..6u8 { put(w, ClueKind::Relic, x0 + 14 * S + k as usize * 4 * S, my, k); }
                put(w, ClueKind::Draft, x0 + 45 * S, my, 0);
                for k in 0..3 { w.torches.push((x0 + (6 + k * 20) * S, my - 4 * S)); }
            }
            "rope" => { // a sealed hall east of the burrow: a hook rope bitten into the ceiling, hanging in a slack curve,
                // and a hook in flight up toward the far ceiling with its rope paying out behind it
                use molehill_sim::{mat::CPM, ropes::{Rope, Hook, NSEG, SEG}};
                let (x0, x1, y0) = (mx + 4 * S, mx + 64 * S, my - 30 * S);
                for x in x0 - 2 * S..x1 + 2 * S { for y in y0 - 4 * S..my + 1 + 2 * S { set(w, x, y, Mat::Limestone); } }
                for x in x0..x1 { for y in y0..=my { set(w, x, y, Mat::Empty); } }
                let (ax, ay) = (x0 + 12 * CPM / 4 * 4, y0 - 1); // the anchor cell in the ceiling
                let pts: Vec<(f32, f32)> = (0..=NSEG).map(|i| { let t = i as f32 * SEG; let y = (ay as f32 + 0.5 + t).min(my as f32 - 0.5); let lie = (ay as f32 + 0.5 + t - y).max(0.);
                    (ax as f32 + 0.5 + (t / (NSEG as f32 * SEG) * 1.5).sin() * 2. * CPM as f32 + lie, y) }).collect();
                w.tr.ropes.push(Rope { prev: pts.clone(), pts, ax, ay });
                let (ox, oy) = ((x0 + 4 * S) as f32, (my - CPM) as f32); let (hx, hy) = (ox + 7. * CPM as f32, oy - 3.5 * CPM as f32); let (vx, vy) = (10. * CPM as f32, -9. * CPM as f32); // fired from inside the hall (its shell walls off the burrow)
                let paid = ((hx - ox).powi(2) + (hy - oy).powi(2)).sqrt(); let out = ((paid / SEG).ceil() as usize + 1).min(NSEG);
                let hp: Vec<(f32, f32)> = (0..=NSEG).map(|i| { if i >= out { return (ox, oy); } let f = i as f32 / out as f32; (hx + (ox - hx) * f, hy + (oy - hy) * f + (f * std::f32::consts::PI).sin() * 0.6 * CPM as f32) }).collect();
                w.tr.hook = Some(Hook { x: hx, y: hy, vx, vy, ox, oy, t: 10, prev: hp.clone(), pts: hp });
                molehill_sim::light::reveal_box(&mut w.g, mx, y0 - 10 * S, x1 + 10 * S, my + 10 * S);
                for k in 0..4 { w.torches.push((x0 + (6 + k * 16) * S, y0 + 4 * S)); }
            }
            "production" => { // a works east of the burrow: a chute sloping down onto a shaker screen, an ore hopper
                // heaped with copper ore over a rail, and a bumper at the end of the track
                use molehill_sim::mat::CPM;
                let (x0, x1, y0) = (mx + 4 * S, mx + 64 * S, my - 22 * S);
                for x in x0 - 2 * S..x1 + 2 * S { for y in y0 - 4 * S..my + 1 + 2 * S { set(w, x, y, Mat::Limestone); } } // a sealed stone shell, so no pocket floods it
                for x in x0..x1 { for y in y0..=my { set(w, x, y, Mat::Empty); } set(w, x, my, Mat::Rail); set(w, x, my - 1, Mat::Rail); }
                // the chute runs out of the west wall; the screen it feeds rests on a timber post at its far end
                for k in 0..10 * CPM / 4 { let (x, y) = (x0 + k, y0 + 4 * S + k / 2); for d in 0..2 { set(w, x, y + d, Mat::Chute); } }
                let (sx, sy) = (x0 + 10 * CPM / 4, y0 + 4 * S + 5 * CPM / 4); for x in sx..sx + 2 * CPM { for d in 0..2 { set(w, x, sy + d, Mat::Screen); } }
                for x in sx + 2 * CPM - 2..sx + 2 * CPM { for y in sy + 2..my - 1 { set(w, x, y, Mat::WoodBeam); } }
                for x in x1 - CPM / 2..x1 { for y in my - CPM..my - 1 { set(w, x, y, Mat::Bumper); } }
                let bx = x0 + 40 * S; let floor = my - 1; let by = floor - molehill_sim::production::BIN_LEGS;
                w.tr.bins.push(molehill_sim::production::OreHopper { x: bx, y: by, floor, contents: vec![(Item::CopperOre, 40)], held: false, open: false, feed: 0, grit: vec![] });
                molehill_sim::light::reveal_box(&mut w.g, mx, y0 - 10 * S, x1 + 10 * S, my + 10 * S);
                for k in 0..4 { w.torches.push((x0 + (6 + k * 16) * S, y0 + 2 * S)); }
            }
            "decor" => { // a furnished hall east of the burrow: plank floor, tiled and polished walls painted, carved pillars,
                // a table with stools and a rug, a nest, a shelf, banners and planters
                let (x0, x1, y0) = (mx + 4 * S, mx + 64 * S, my - 18 * S);
                for x in x0..x1 { for y in y0..=my { set(w, x, y, Mat::Empty); } for y in my + 1..my + 1 + 2 * S { set(w, x, y, Mat::Planks); } for y in y0 - 2 * S..y0 { set(w, x, y, Mat::Polished); } }
                for y in y0..=my { for x in x1..x1 + 2 * S { set(w, x, y, Mat::Tiles); } }
                for &px in &[x0 + 20 * S, x0 + 44 * S] { for y in y0..=my { for x in px..px + 2 * S { set(w, x, y, Mat::Carved); } } }
                for x in x0..x1 { for y in y0 - 2 * S..y0 { let i = w.g.idx(x, y); w.g.set_paint(i, 6); } } // soot-black ceiling band
                for y in y0..=my { for x in x1..x1 + 2 * S { let i = w.g.idx(x, y); w.g.set_paint(i, if (y / (4 * S)) % 2 == 0 { 5 } else { 3 }); } } // woad and chalk tiles
                molehill_sim::light::reveal_box(&mut w.g, mx, y0 - 10 * S, x1 + 10 * S, my + 10 * S);
                let fl = my; let put = |w: &mut World, it: Item, x: usize, tint: u8| { if let Some(mut f) = w.furniture_spot(it, x, fl - S) { f.tint = tint; w.furniture.push(f); } };
                put(w, Item::Rug, x0 + 10 * S, 2); put(w, Item::Table, x0 + 10 * S, 0); put(w, Item::Stool, x0 + 4 * S, 0); put(w, Item::Stool, x0 + 16 * S, 0);
                put(w, Item::Nest, x0 + 30 * S, 4); put(w, Item::Shelf, x0 + 38 * S, 0); put(w, Item::Planter, x0 + 52 * S, 0); put(w, Item::Planter, x0 + 58 * S, 0);
                for (k, t) in [(26usize, 1u8), (34, 5)] { if let Some(mut f) = w.furniture_spot(Item::Banner, x0 + k * S, y0 + 2 * S) { f.tint = t; w.furniture.push(f); } }
                for k in 0..3 { w.torches.push((x0 + (6 + k * 20) * S, my - 8 * S)); }
            }
            "wiring" => { // a tall open hall east of the burrow with a little circuit: levers -> both-knot -> lamp,
                // a ticker, a thrummer and a listener, and one of most devices, on copperline
                use molehill_sim::logic::{Dev, B as LB};
                let (x0, x1, y0, y1) = (mx + 4 * S, mx + 60 * S, my - 18 * S, my);
                for x in x0..x1 { for y in y0..=y1 { set(w, x, y, Mat::Empty); } set(w, x, my + S, Mat::Limestone); }
                molehill_sim::light::reveal_box(&mut w.g, mx, y0 - 10 * S, x1 + 10 * S, my + 10 * S);
                let bw = w.logic.bw; let (bx0, by0) = (x0 / LB + 1, my / LB);
                let mut dev = |w: &mut World, bx: usize, by: usize, d: Dev| { let b = by * bw + bx; w.logic.place(&mut w.g, d, b); };
                let wire = |w: &mut World, bx: usize, by: usize| { let b = by * bw + bx; w.logic.place_wire(b); };
                // lever A on the floor west of the knot, lever B above it feeding the knot's north input
                dev(w, bx0, by0, Dev::Lever); wire(w, bx0 + 1, by0); dev(w, bx0 + 2, by0, Dev::And);
                dev(w, bx0 + 2, by0 - 2, Dev::Lever); wire(w, bx0 + 2, by0 - 1);
                for k in 3..6 { wire(w, bx0 + k, by0); } dev(w, bx0 + 6, by0, Dev::Lamp); wire(w, bx0 + 6, by0 - 1); wire(w, bx0 + 6, by0 - 2); wire(w, bx0 + 5, by0 - 2);
                // flip both levers: the knot fires, the lamp lights
                for bx in [bx0, bx0 + 2] { let by = if bx == bx0 { by0 } else { by0 - 2 }; w.logic.dev_s[by * bw + bx] = 1; }
                // a ticker blinking a second lamp
                dev(w, bx0 + 8, by0, Dev::Timer); wire(w, bx0 + 9, by0); dev(w, bx0 + 10, by0, Dev::Lamp);
                // thrummer on a lever, listener further on driving a door
                dev(w, bx0 + 12, by0, Dev::Lever); w.logic.dev_s[by0 * bw + bx0 + 12] = 1; wire(w, bx0 + 13, by0); dev(w, bx0 + 14, by0, Dev::Tx);
                dev(w, bx0 + 20, by0, Dev::Rx); wire(w, bx0 + 21, by0); dev(w, bx0 + 22, by0 - 1, Dev::Door);
                // a row of the rest, up on a ledge line
                let rest = [Dev::Plate, Dev::Sensor, Dev::Pump, Dev::Tnt, Dev::Fuse, Dev::Or, Dev::Not, Dev::Xor, Dev::Latch, Dev::Rep, Dev::Vent, Dev::Heat, Dev::Wheel, Dev::Floodgate];
                for (k, d) in rest.into_iter().enumerate() { dev(w, bx0 + k, by0 - 5, d); }
                for k in 0..6 { wire(w, bx0 + k, by0 - 7); } wire(w, bx0 + 5, by0 - 8); wire(w, bx0 + 3, by0 - 6);
            }
            "mine" | "site-mine" => stage_site(w, SiteKind::Mine),
            "holt" | "site-holt" => stage_site(w, SiteKind::Holt),
            "pump" | "pumpworks" | "site-pump" => stage_site(w, SiteKind::Pumpworks),
            "fissure" | "site-fissure" => stage_site(w, SiteKind::Fissure),
            "fossil" | "site-fossil" => stage_site(w, SiteKind::Fossil),
            "crystal" | "salt" | "site-crystal" => stage_site(w, SiteKind::Crystal),
            "halt" | "site-halt" => stage_site(w, SiteKind::Halt),
            "walls" | "wall" => stage_wall_edit(w, mx, my, set),
            _ => {}
        }
        let f = w.st.solve(&w.g); w.st.crack(&w.g, &f);
    }
}

/// Stamp one found place just east of the burrow: only open cells become host, so the room sits in the
/// existing rock instead of a cut-out cube. The mole stands inside, on a lower floor or ledge.
fn stage_site(w: &mut World, kind: SiteKind) {
    let (sw, sh) = kind.size();
    let host = match kind {
        SiteKind::Mine | SiteKind::Holt | SiteKind::Pumpworks => Mat::Limestone,
        SiteKind::Crystal => Mat::Basalt,
        _ => Mat::Granite,
    };
    let (mx, my) = (w.moles[0].x, w.moles[0].y);
    let pad = CPM;
    let x0 = (mx + (molehill_sim::gen::START_SAFE as usize + 2) * CPM).min(w.g.w.saturating_sub(sw + pad + 8)).max(8);
    let mut y0 = my.saturating_sub(sh.saturating_sub(CPM + 4)).max(8);
    if y0 + sh + pad + 8 >= w.g.h { y0 = w.g.h.saturating_sub(sh + pad + 8); }
    let (hx0, hy0) = (x0.saturating_sub(pad), y0.saturating_sub(pad));
    let (hx1, hy1) = ((x0 + sw + pad).min(w.g.w - 2), (y0 + sh + pad).min(w.g.h - 2));
    for y in hy0..hy1 { for x in hx0..hx1 {
        let i = w.g.idx(x, y);
        if !w.g.m(i).passable() { continue; }
        w.g.set(i, host); w.walls.set_at(x, y, Wall::None);
    } }
    w.furniture.retain(|f| f.x < hx0 || f.x >= hx1 || f.y < hy0 || f.y >= hy1);
    w.farms.retain(|f| f.x < hx0 || f.x >= hx1 || f.y < hy0 || f.y >= hy1);
    w.torches.retain(|&(x, y)| x < hx0 || x >= hx1 || y < hy0 || y >= hy1);
    w.stamp_site(kind, x0, y0);
    let n = (sw / (5 * CPM)).max(3);
    for k in 0..n {
        let x = x0 + 2 * CPM + k * sw.saturating_sub(4 * CPM) / n.max(1);
        for &y in &[y0 + 2 * CPM, y0 + sh / 2, y0 + sh.saturating_sub(3 * CPM)] {
            if x >= w.g.w || y >= w.g.h || w.g.is_solidish(y * w.g.w + x) { continue; }
            if w.torches.iter().any(|&(tx, ty)| tx.abs_diff(x) < 2 * S && ty.abs_diff(y) < 2 * S) { continue; }
            w.torches.push((x, y));
        }
    }
    let (gw, gh) = (w.g.w, w.g.h);
    molehill_sim::light::reveal_box(&mut w.g, hx0.saturating_sub(16 * S), hy0.saturating_sub(16 * S), (hx1 + 16 * S).min(gw - 1), (hy1 + 16 * S).min(gh - 1));
    force_stand(w, x0, y0, sw, sh);
}

fn site_stand(w: &World, x0: usize, y0: usize, sw: usize, sh: usize) -> Option<(usize, usize)> {
    let want_x = x0 + sw / 3; let want_y = y0 + sh.saturating_sub(3 * CPM); let mut best = None;
    for y in y0 + 3..y0 + sh.saturating_sub(1) {
        for x in x0 + 2..x0 + sw.saturating_sub(2) {
            if !w.fits(x, y) { continue; }
            if y + 1 >= w.g.h || !w.g.is_solidish((y + 1) * w.g.w + x) { continue; }
            if w.g.m(y * w.g.w + x) == Mat::Water { continue; }
            let floor = w.g.m((y + 1) * w.g.w + x);
            let ledge = matches!(floor, Mat::WoodBeam | Mat::IronBeam | Mat::Rail | Mat::StoneBrick | Mat::Carved | Mat::Quartz);
            let score = (x as i32 - want_x as i32).unsigned_abs() + (y as i32 - want_y as i32).unsigned_abs() * 2 + if ledge { 0 } else { 24 };
            if best.map_or(true, |(_, _, s)| score < s) { best = Some((x, y, score)); }
        }
    }
    best.map(|(x, y, _)| (x, y))
}

/// Always put the mole in the stamp: carve a short limestone shelf if the room has no floor yet.
fn force_stand(w: &mut World, x0: usize, y0: usize, sw: usize, sh: usize) {
    let (px, py) = site_stand(w, x0, y0, sw, sh).unwrap_or_else(|| {
        ((x0 + sw / 2).min(w.g.w - 4).max(4), (y0 + sh.saturating_sub(3 * CPM)).min(w.g.h - 8).max(8))
    });
    for x in px.saturating_sub(CPM)..=(px + CPM).min(w.g.w - 1) {
        for y in py.saturating_sub(3 * S)..=py { if y < w.g.h { w.g.set(w.g.idx(x, y), Mat::Empty); } }
        for dy in 1..=CPM { if py + dy < w.g.h { w.g.set(w.g.idx(x, py + dy), Mat::Limestone); } }
    }
    w.moles[0].x = px; w.moles[0].y = py; w.pl.sync = None; w.pl.fall_from = py;
}

/// A lined room with scraped / timber / decorative-beam / masonry / carved bands, and real props in front.
fn stage_wall_edit(w: &mut World, mx: usize, my: usize, set: impl Fn(&mut World, usize, usize, Mat)) {
    let (x0, x1, y0) = (mx.saturating_sub(8 * S).max(8), (mx + 72 * S).min(w.g.w - 8), my.saturating_sub(28 * S).max(8));
    for x in x0.saturating_sub(4 * S)..(x1 + 4 * S).min(w.g.w - 1) {
        for y in y0.saturating_sub(4 * S)..(my + 6 * S).min(w.g.h - 1) { set(w, x, y, Mat::Limestone); }
    }
    for x in x0..x1 {
        for y in y0..=my { set(w, x, y, Mat::Empty); }
        for y in my + 1..my + 1 + 2 * S { set(w, x, y, Mat::Planks); }
        for y in y0.saturating_sub(2 * S)..y0 { set(w, x, y, Mat::Limestone); }
    }
    for &px in &[x0 + 16 * S, x0 + 40 * S] {
        for y in y0..=my { for x in px..px + S { set(w, x, y, Mat::WoodBeam); } }
    }
    for y in y0..=my { for x in x0 + 56 * S..x0 + 56 * S + S { set(w, x, y, Mat::IronBeam); } }
    for x in x0..x1 {
        for y in y0..=my {
            let t = (x - x0) as f32 / (x1 - x0).max(1) as f32;
            let wl = if t < 0.18 { Wall::None } else if t < 0.42 { Wall::Timber } else if t < 0.62 { Wall::Beams } else if t < 0.80 { Wall::Masonry } else { Wall::Carving };
            w.walls.set_at(x, y, wl);
        }
    }
    for k in 0..4 { w.torches.push((x0 + (8 + k * 16) * S, my.saturating_sub(10 * S))); }
    let (gw, gh) = (w.g.w, w.g.h);
    molehill_sim::light::reveal_box(&mut w.g, x0.saturating_sub(10 * S), y0.saturating_sub(10 * S), (x1 + 10 * S).min(gw - 1), (my + 10 * S).min(gh - 1));
    let px = x0 + (x1 - x0) * 35 / 100;
    if let Some((sx, sy)) = site_stand(w, x0, y0, x1 - x0, my.saturating_sub(y0) + 2) {
        w.moles[0].x = sx; w.moles[0].y = sy; w.pl.sync = None;
    } else if w.fits(px, my) { w.moles[0].x = px; w.moles[0].y = my; w.pl.sync = None; }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut fonts: ResMut<Assets<Font>>) {
    let seed = seed_now() % 100000;
    // dev stills and scenes start in a holt at once; play opens the main menu straight away over an
    // empty window (a new holt is built behind the loading line, a save read the same way)
    let live = dev_start();
    let w = if live { make_world(seed, Mode::Survival) } else { World::blank(W, H, seed) };
    // The world is split into CH×CH-cell chunk textures; only chunks whose composited pixels changed
    // are touched (Assets::get_mut marks an image for re-upload), so a quiet frame uploads nothing.
    let oy0 = w.oy;
    let mut chunks = vec![];
    for cy in 0..H / CH { for cx in 0..W / CH {
        let img = images.add(art::image(CH as u32, CH as u32, vec![0; CH * CH * 4], true));
        let p = cell_to_world((cx * CH) as f32 + CH as f32 / 2., (cy * CH) as f32 + CH as f32 / 2.);
        commands.spawn((Sprite { image: img.clone(), custom_size: Some(Vec2::splat(CH as f32 * CPX)), ..default() }, Transform::from_xyz(p.x, p.y, 0.), WorldSprite));
        chunks.push(img);
    } }
    // far bands: FAR chunk rows above (band rows 0..FAR) and below (FAR..2·FAR) the window
    let mut far = vec![];
    for b in 0..2 * FAR { for cx in 0..W / CH {
        let cy = if b < FAR { b as f32 - FAR as f32 } else { (H / CH + b - FAR) as f32 };
        let img = images.add(art::image(CH as u32, CH as u32, vec![0; CH * CH * 4], true));
        let p = cell_to_world((cx * CH) as f32 + CH as f32 / 2., cy * CH as f32 + CH as f32 / 2.);
        commands.spawn((Sprite { image: img.clone(), custom_size: Some(Vec2::splat(CH as f32 * CPX)), ..default() }, Transform::from_xyz(p.x, p.y, 0.), WorldSprite));
        far.push(img);
    } }
    let far_done = vec![false; far.len()];
    commands.insert_resource(make_bg(&w));
    // parallax: a flat base colour, then the three layers (far to near), all behind the world chunks
    let bw = (W + 2 * PAD) as u32;
    commands.spawn((Sprite { image: images.add(art::image(1, 1, vec![255; 4], true)), custom_size: Some(Vec2::new((W + 4 * PAD) as f32 * CPX, (H + 2 * FAR * CH) as f32 * CPX)), color: Color::srgb_u8(BG_BASE[0] as u8, BG_BASE[1] as u8, BG_BASE[2] as u8), ..default() }, Transform::from_xyz(0., 0., -4.)));
    let (lw, lh) = (bw / BG_RES as u32, (H / BG_RES) as u32);
    for k in 0..3 { let img = images.add(art::image(lw, lh, vec![0; (lw * lh) as usize * 4], true));
        commands.spawn((Sprite { image: img, custom_size: Some(Vec2::new(bw as f32 * CPX, H as f32 * CPX)), ..default() }, Transform::from_xyz(0., 0., -3. + k as f32 * 0.1), BgLayer(k))); }
    let (sx, sy) = (w.moles[0].x as f32, w.moles[0].y as f32 - 1.5 * SF);
    let p = cell_to_world(sx, sy);
    let zoom = shot_or_menu_zoom();
    commands.spawn((Camera2d, Transform::from_xyz(p.x, p.y, 100.).with_scale(Vec3::splat(zoom))));
    let font: Handle<Font> = fonts.add(Font::from_bytes(include_bytes!("../assets/PixelifySans.ttf").to_vec()));
    menu::setup(&mut commands, &font); let mut mset = menu::Menu::load(); mset.zoom = zoom; commands.insert_resource(mset);
    ui::setup(&mut commands, &font); journal::setup(&mut commands, &font); loading::setup(&mut commands, &font);
    { let ring = images.add(art::ring()); let px = images.add(art::image(1, 1, vec![255; 4], true)); wiring::setup(&mut commands, &mut images, &font, ring, px); }
    commands.insert_resource(ui::UiFont(font));
    // a pinned seed opens that holt (built behind the loading line) instead of the main menu
    if !live && (seed_pinned() || shot_loading()) { let mut ld = loading::Loading::default(); ld.new_holt(seed, Mode::Survival, false); commands.insert_resource(ld); }
    let sp = Sprites { tools: [images.add(art::tool_icon(1)), images.add(art::tool_icon(2)), images.add(art::tool_icon(3))], dev: DEVS.iter().map(|&d| images.add(devart::hotbar_icon(d))).collect(), ring: images.add(art::ring()),
        mole: art::mole_sheet(true).into_iter().map(|i| images.add(i)).collect(),
        mole_out: art::mole_sheet(false).into_iter().map(|i| images.add(i)).collect(),
        held: [art::Held::Pick, art::Held::Drill, art::Held::Bore, art::Held::Charge, art::Held::Block(Item::WoodPillar.color()), art::Held::Block(Item::Brick.color()), art::Held::Block(Item::IronBeam.color()), art::Held::RopeGun(true), art::Held::RopeGun(false)]
            .into_iter().map(|h| images.add(art::held_sprite(h))).collect(),
        clue: (0..9).map(|k| images.add(decorart::clue(k))).collect(), props: props::PROPS.iter().map(|&p| images.add(props::image(p))).collect(), cart: images.add(art::cart()), rope: [images.add(art::rope_seg()), images.add(art::rope_hook())], bin: (0..5).map(|k| images.add(art::ore_bin(k))).collect(),
        furn: ITEMS.iter().map(|&it| it.furn().map(|_| { let (b, c) = decorart::furniture(it); (images.add(decorart::finish(b)), c.map(|c| images.add(decorart::cloth(c)))) })).collect(),
        icons: ITEMS.iter().map(|&it| images.add(itemart::icon(it))).collect(),
        chest: [images.add(art::chest(false)), images.add(art::chest(true))],
        station: (0..4u8).map(|k| [images.add(art::station(k, false)), images.add(art::station(k, true))]).collect(),
        torch: images.add(itemart::icon(Item::Torch)), torch_f: art::torch_frames().into_iter().map(|i| images.add(i)).collect(), glow: images.add(art::glow()),
        shroom: [images.add(art::mushroom(0)), images.add(art::mushroom(1)), images.add(art::mushroom(2))],
        px: images.add(art::image(1, 1, vec![255; 4], true)),
    };
    commands.insert_resource(sp);
    let disp = Vec2::new(sx, sy + 1.5 * SF);
    let mut slots = default_slots(); let mut hot = 0usize; let mut relayout = true;
    if matches!(scene_name().as_deref(), Some("walls") | Some("wall")) {
        if let Some(k) = slots.iter().position(|s| *s == Some(Thing::Item(Item::TimberWall))) { slots.swap(0, k); hot = 0; }
        relayout = false;
    }
    commands.insert_resource(Game { w, chunks, buf: vec![1; W * H * 4], uploads: 0, acc: 0., paused: true, speed: 1, frame: 0, last_light: 0, msg: String::new(), help: false, ev: vec![], shake_off: Vec2::ZERO, autosave: 0., last_oy: oy0, far, far_done, far_cells: Default::default(), inv_open: false, aim: None, slots, held: None, relayout, hot, sel_t: 0., toasts: vec![], log_seen: 0, show_dev: false, disp, seed, sizes: Default::default(), drag: None, brush_tag: None, demo_right: false, wheel: Default::default(), mouse_latch: false, jump_tap: false, jump_key: false, w_jump: false, charge: None, throw_aim: (1., -1.), e_latch: false, pops: vec![], look: Vec2::ZERO, live });
}

/// Saves only write a holt in play: Quit on the main menu before Continue never puts the empty window over the autosave.
pub fn save_now(g: &mut Game) { if !g.live { return; } g.msg = match save_slot("molehill-manual", &g.w) { Ok(()) => "Game saved".into(), Err(e) => format!("Save failed: {e}") }; }
pub fn save_auto(g: &mut Game) { if !g.live { return; } if let Err(e) = save_slot("molehill-auto", &g.w) { g.msg = format!("Autosave failed: {e}"); } }
fn reset_view(g: &mut Game, commands: &mut Commands) {
    g.last_oy = g.w.oy; g.far_done.iter_mut().for_each(|v| *v = false); g.far_cells.clear(); commands.insert_resource(make_bg(&g.w)); g.buf.iter_mut().for_each(|v| *v = 1);
    g.w.pl.sync = None; g.inv_open = false; g.help = false; g.acc = 0.;
}
/// Dev runs that need a holt from the first frame (`--shot`, `--scene`): generated at once in `setup`.
fn dev_start() -> bool {
    #[cfg(not(target_arch = "wasm32"))] { !shot_loading() && std::env::args().any(|a| a.starts_with("--shot") || a.starts_with("--scene=")) }
    #[cfg(target_arch = "wasm32")] { false }
}
/// Dev (native): `--shot-ui=loading` stills the loading line of a new holt instead.
fn shot_loading() -> bool {
    #[cfg(not(target_arch = "wasm32"))] { std::env::args().any(|a| a == "--shot-ui=loading") }
    #[cfg(target_arch = "wasm32")] { false }
}
/// Finish a new holt's world (built by `loading`) and start playing it.
pub fn install(g: &mut Game, commands: &mut Commands, mut w: World, seed: u32, mode: Mode) {
    w.disasters.enabled = false; w.set_mode(scene_mode(mode)); stage_scene(&mut w);
    g.w = w; g.seed = seed; g.live = true; reset_view(g, commands); g.msg = format!("Holt {seed} ({})", mode.name()); g.relayout = true;
}
/// Load the manual save (or the autosave). Returns false if there is none or it failed.
pub fn load_saved(g: &mut Game, commands: &mut Commands) -> bool {
    match load_slot("molehill-manual").or_else(|| load_slot("molehill-auto")) {
        Some(Ok(w)) if w.g.w != W || w.g.h != H => { g.msg = format!("Load failed: save is {}x{}, this build is {W}x{H}", w.g.w, w.g.h); false }
        // a save from before modes was a Creative holt (everything in the pack)
        Some(Ok(w)) => { g.w = w; g.live = true; if g.w.mode == Mode::Plain { g.w.mode = Mode::Creative; g.w.rules = Mode::Creative.rules(); } if g.w.mode == Mode::Creative { for v in g.w.inv.iter_mut() { *v = (*v).max(999); } } reset_view(g, commands); g.msg = "Game loaded".into(); g.relayout = true; true }
        Some(Err(e)) => { g.msg = format!("Load failed: {e}"); false } None => { g.msg = "No save yet".into(); false } }
}
fn make_bg(w: &World) -> Bg {
    let mk = |k, f| (art::layer(k, w.oy, &w.info.biome, W, H, PAD, BG_RES, w.info.seed).0, f);
    Bg { layers: vec![mk(art::Layer::Strata, 0.8), mk(art::Layer::Caves, 0.6), mk(art::Layer::Veins, 0.35)] }
}
fn dev_cost(d: Dev) -> Option<Item> { d.cost() }
fn input(mut g: ResMut<Game>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, scroll: Res<AccumulatedMouseScroll>,
         win: Query<&Window, With<PrimaryWindow>>, mut cam: Query<&mut Transform, With<Camera2d>>, time: Res<Time>,
         mut menu: ResMut<menu::Menu>, lkeys: Res<ButtonInput<Key>>, mut ld: ResMut<loading::Loading>) {
    let Ok(win) = win.single() else { return }; let Ok(mut ct) = cam.single_mut() else { return };
    if g.paused { g.w.input = Default::default(); g.aim = None; g.jump_tap = false; g.charge = None; return; } // a menu is open: no input, no aim ring
    let sc = ct.scale.x; let now = time.elapsed_secs();
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    // toasts: short game messages and new world log lines (deduplicated, at most 3 live)
    let fresh: Vec<String> = g.w.log.iter().filter(|(t, _)| *t > g.log_seen).map(|(_, l)| l.clone()).collect();
    if let Some((t, _)) = g.w.log.last() { g.log_seen = *t; }
    let msg = std::mem::take(&mut g.msg);
    for m in fresh.into_iter().chain((!msg.is_empty()).then_some(msg)) { if !g.toasts.iter().any(|(s, t)| *s == m && now - t < 4.) { g.toasts.push((m, now)); } }
    g.toasts.retain(|(_, t)| now - t < 4.); while g.toasts.len() > 3 { g.toasts.remove(0); }
    // movement → the sim's input (applied every tick)
    let dead = !g.w.moles.first().map_or(false, |m| m.alive);
    g.w.input.left = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
    g.w.input.right = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) || g.demo_right;
    let upk = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp);
    g.w.input.up = upk; g.w.input.down = keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown);
    // Space jumps; W jumps too on a fresh press (held, it doesn't hop again on landing), except on a rope ladder
    // or by a rope-gun rope: there W grabs and climbs (`World::ladder_takes_up`, `World::rope_takes_up`)
    let upj = (keys.just_pressed(KeyCode::KeyW) || keys.just_pressed(KeyCode::ArrowUp)) && !g.w.ladder_takes_up() && !g.w.rope_takes_up();
    g.w_jump = upk && (g.w_jump || upj);
    let jk = keys.pressed(KeyCode::Space) || g.w_jump;
    if keys.just_pressed(KeyCode::Space) || upj { g.jump_tap = true; }
    // Shift held while moving: sprint (Shift-click stays free for other uses)
    g.w.input.sprint = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    g.jump_key = jk; g.w.input.jump = jk || g.jump_tap;
    g.w.input.mine = None;
    let sel0 = g.hot;
    // wheel: hotbar (Shift is sprint and changes nothing); Ctrl/Cmd+wheel: zoom; Alt/Option+wheel: brush size.
    // + / - zoom too; [ / ] resize the brush. Zoom is shared with Settings (clamped, saved). See `view::wheel_use`.
    let mods = sv::Mods { shift: keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight), ctrl,
        cmd: keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight), alt: keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight) };
    let held_brush = brush_of(g.slots[g.hot]);
    let notch = g.wheel.feed(sv::wheel_delta(scroll.delta.x, scroll.delta.y, mods), scroll.unit == MouseScrollUnit::Pixel, time.delta_secs()) as f32;
    // [ / ]: by the character typed (Option+8 / Option+9 on a Swedish Mac) or the US key position
    let typed = |c: &str| lkeys.just_pressed(Key::Character(c.into()));
    let mut resize = sv::brush_key_steps(typed("["), typed("]"), keys.just_pressed(KeyCode::BracketLeft), keys.just_pressed(KeyCode::BracketRight)) as f32;
    if notch != 0. {
        match sv::wheel_use(mods, held_brush.is_some()) {
            sv::WheelUse::Brush => resize += notch,
            sv::WheelUse::Zoom => menu.zoom_by(notch, &mut ct),
            sv::WheelUse::Hotbar => g.hot = (g.hot as i32 - notch as i32).rem_euclid(10) as usize,
        }
    }
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) { menu.zoom_by(1., &mut ct); }
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) { menu.zoom_by(-1., &mut ct); }
    let _ = sc;
    if let (Some(b), true) = (held_brush, resize != 0.) { g.sizes.step(b, resize as i32); g.sel_t = now; }
    // X: grid snap for building (square brush on a 0.25 / 0.5 / 1 m grid, a drag lays a straight line)
    if keys.just_pressed(KeyCode::KeyX) { g.sizes.snap = !g.sizes.snap; g.msg = if g.sizes.snap { format!("Grid snap on ({}): drag lays a straight line", g.sizes.label(sw::Brush::Place(Item::Brick))) } else { "Grid snap off".into() }; }
    let digits = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9, KeyCode::Digit0];
    if sv::digit_selects_slot(mods) { for (k, d) in digits.iter().enumerate() { if keys.just_pressed(*d) { g.hot = k; } } }
    if keys.just_pressed(KeyCode::Escape) { g.inv_open = false; g.help = false; }
    if keys.just_pressed(KeyCode::KeyF) { g.speed = if g.speed >= 4 { 1 } else { g.speed * 2 }; }
    if keys.just_pressed(KeyCode::KeyE) && g.inv_open { g.e_latch = true; }
    if !keys.pressed(KeyCode::KeyE) { g.e_latch = false; }
    if keys.just_pressed(KeyCode::KeyI) || keys.just_pressed(KeyCode::KeyE) && g.inv_open { g.inv_open = !g.inv_open; }
    if !g.inv_open { if let Some(t) = g.held.take() { if let Some(k) = g.slots.iter().position(|s| s.is_none()) { g.slots[k] = Some(t); } } }
    if keys.just_pressed(KeyCode::KeyH) { g.help = !g.help; }
    if keys.just_pressed(KeyCode::F3) { g.show_dev = !g.show_dev; }
    if keys.just_pressed(KeyCode::KeyK) { save_now(&mut g); }
    if keys.just_pressed(KeyCode::KeyL) { ld.load(); }
    if keys.just_pressed(KeyCode::KeyB) && g.show_dev { let kind = [Kind2::Flood, Kind2::Lava, Kind2::Quake][(g.frame % 3) as usize]; let mut d = std::mem::replace(&mut g.w.disasters, molehill_sim::disasters::Disasters::new(0)); let _ = d.schedule(&mut g.w, kind); g.w.disasters = d; }
    // UI clicks: slots swallow the mouse. Hotbar click selects; in the pack, click picks up / drops / swaps.
    if g.mouse_latch && !mouse.pressed(MouseButton::Left) && !mouse.pressed(MouseButton::Right) { g.mouse_latch = false; }
    if !mouse.pressed(MouseButton::Left) { g.drag = None; }
    g.brush_tag = None;
    let latched = g.mouse_latch;
    let cur = win.cursor_position();
    let over = cur.and_then(|c| ui::slot_at(c, win.width(), win.height(), g.inv_open));
    // the crafting list (pack open): a click on a recipe row crafts it
    if let (Some(c), true) = (cur, mouse.just_pressed(MouseButton::Left) && !latched) { if let Some(k) = craft_row_at(&g, c) {
        if let Some(r) = craft_rows(&g)[k].recipe { g.msg = match g.w.craft_now(r) { Ok(()) => format!("Made {}", RECIPES[r].name), Err(e) => format!("Can't craft: {e}") }; }
        g.mouse_latch = true; } }
    let latched = g.mouse_latch;
    if let Some(k) = over {
    if mouse.just_pressed(MouseButton::Left) && !latched {
            if g.inv_open { let h = g.held.take(); g.held = g.slots[k].take(); g.slots[k] = h; } else { g.hot = k; }
        }
    }
    if g.hot != sel0 { g.sel_t = now; }
    let sel = g.slots[g.hot];
    let pick = match sel { Some(Thing::Pick(n)) => n, _ => 1 };
    g.w.input.tool = pick; g.w.input.brush = brush_r(&g, sw::Brush::Dig(pick));
    if dead { g.aim = None; return; }
    // S: dig straight down (with the pick in hand, else the pickaxe)
    let (mx, my) = (g.w.moles[0].x, g.w.moles[0].y);
    // S digs down, except where it climbs down a rope ladder (at its head, or with rope below the feet)
    if (keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)) && g.w.pl.grounded && !g.w.ladder_takes_down() { g.w.input.mine = Some((mx, my + 2 * S)); }
    let on_panel = over.is_some() || cur.map_or(false, |c| craft_row_at(&g, c).is_some()) || g.inv_open && cur.map_or(false, |c| { let a = ui::slot_pos(10, win.width(), win.height()); c.y > a.y - 10. && (c.x - win.width() / 2.).abs() < 5. * (ui::S + ui::GAP) + 10. });
    let Some(cur) = cur else { g.aim = None; return };
    if on_panel { g.aim = None; return; }
    let wp = Vec2::new(ct.translation.x + (cur.x - win.width() / 2.) * sc, ct.translation.y - (cur.y - win.height() / 2.) * sc);
    let (cx, cy) = world_to_cell(wp); if cx < 0 || cy < 0 || cx as usize >= W || cy as usize >= H { g.aim = None; return; }
    let (cx, cy) = (cx as usize, cy as usize);
    let reach = g.w.in_reach(cx, cy); g.aim = Some((cx, cy, reach));
    if let Some(b) = brush_of(g.slots[g.hot]) { let cells = if g.sizes.snapping(b) { g.sizes.snap_cells() as f32 * 0.75 } else { brush_r(&g, b) }; g.brush_tag = Some((g.sizes.label(b), cells * CPX / sc)); }
    // Q: hold to charge a throw of the item in hand (aimed at the cursor), release to throw it. A TNT
    // charge goes out lit. Charging takes THROW_CHARGE s to full power.
    if let Some(Thing::Item(it)) = sel.filter(|t| matches!(t, Thing::Item(i) if i.uses().throw)) {
        let fc = Vec2::new((wp.x + W as f32 * CPX / 2.) / CPX, (H as f32 * CPX / 2. - wp.y) / CPX);
        g.throw_aim = (fc.x - g.w.pl.px, fc.y - (g.w.pl.py - sw::BODY_H * 0.7));
        if keys.pressed(KeyCode::KeyQ) && g.w.inv[it as usize] > 0 { g.charge = Some((g.charge.unwrap_or(0.) + time.delta_secs()).min(THROW_CHARGE)); }
        else if let Some(c) = g.charge.take() { let aim = g.throw_aim; if let Err(e) = g.w.throw_item(it, aim, c / THROW_CHARGE) { g.msg = format!("Can't throw: {e}"); } }
    } else { g.charge = None; if keys.just_pressed(KeyCode::KeyQ) { if let Some(th) = sel { g.msg = format!("{} can't be thrown", thing_name(&g, th)); } } }
    // the rope gun throws its hook at the cursor, however far (the rope runs out at 12 m)
    if sel == Some(Thing::Item(Item::RopeGun)) && mouse.just_pressed(MouseButton::Left) && !latched {
        let fc = ((wp.x + W as f32 * CPX / 2.) / CPX, (H as f32 * CPX / 2. - wp.y) / CPX);
        if let Err(e) = g.w.fire_rope_gun(fc) { g.msg = format!("Can't fire: {e}"); } }
    let blk = g.w.logic.block_of(cx, cy);
    // a dig aimed past reach still goes to the sim, which answers it at once ("Too far to dig")
    if !reach && matches!(sel, Some(Thing::Pick(_))) && mouse.pressed(MouseButton::Left) && !latched { g.w.input.mine = Some((cx, cy)); }
    if !reach || latched && mouse.pressed(MouseButton::Right) { return; }
    // E (hold and sweep): pick up the device, unlit TNT, torch or copperline under the cursor; no digging needed
    if keys.pressed(KeyCode::KeyE) && !g.inv_open && !g.e_latch {
        if g.w.pick_up(cx, cy).is_none() && keys.just_pressed(KeyCode::KeyE) && g.w.logic.lit(blk) { g.msg = "It's burning: too late to pick it up".into(); }
    }
    // right mouse: interact (lever, thrum channel, ticker, light a fuse or TNT); on an empty spot it sets down
    // the device (or TNT, unlit) in hand
    // (a glowmite under the cursor goes in a jar first)
    let wall_hand = matches!(sel, Some(Thing::Item(it)) if it.is_wall());
    if wall_hand && mouse.pressed(MouseButton::Right) && !latched {
        if mouse.just_pressed(MouseButton::Right) && (g.w.catch_glowmite(cx, cy) || g.w.use_device(cx, cy)) {}
        else if let Some(Thing::Item(it)) = sel {
            let r = brush_r(&g, sw::Brush::Place(it));
            if let Err(e) = g.w.scrape_walls(cx, cy, r) { if mouse.just_pressed(MouseButton::Right) { g.msg = format!("Can't scrape: {e}"); } }
        }
    } else if mouse.just_pressed(MouseButton::Right) && !g.w.catch_glowmite(cx, cy) && !g.w.use_device(cx, cy) {
        let d = match sel { Some(Thing::Dev(d)) => Some(DEVS[d]), Some(Thing::Item(Item::Tnt)) => Some(Dev::Tnt), Some(Thing::Item(Item::Lamp)) => Some(Dev::Lamp), Some(Thing::Item(Item::Vent)) => Some(Dev::Vent), _ => None };
        if let Some(d) = d { if let Err(e) = g.w.place_device(d, cx, cy) { g.w.refuse_place(d.name(), e); } }
    }
    // left mouse: use the item in hand
    let (lp, lj) = (mouse.pressed(MouseButton::Left) && !latched, mouse.just_pressed(MouseButton::Left) && !latched);
    match sel {
        Some(Thing::Pick(_)) if lp => g.w.input.mine = Some((cx, cy)),
        Some(Thing::Item(it)) if lp && it.is_wall() => {
            let r = if g.sizes.snapping(sw::Brush::Place(it)) { let a = *g.drag.get_or_insert((cx, cy)); let n = g.sizes.snap_cells(); g.w.place_wall_line(a, (cx, cy), n, it) }
                else { let r = brush_r(&g, sw::Brush::Place(it)); g.w.place_walls(cx, cy, it, r) };
            if let Err(e) = r { if lj { g.w.refuse_place(it.name(), e); } } }
        Some(Thing::Item(it)) if lp && sw::block_code(it).is_some() => {
            let r = if g.sizes.snapping(sw::Brush::Place(it)) { let a = *g.drag.get_or_insert((cx, cy)); let n = g.sizes.snap_cells(); g.w.place_line(a, (cx, cy), n, it) }
                else { let r = brush_r(&g, sw::Brush::Place(it)); g.w.place_brush(cx, cy, it, r) };
            if let Err(e) = r { if lj { g.w.refuse_place(it.name(), e); } } }
        Some(Thing::Item(Item::CopperWire)) if lp => { g.w.lay_wire(cx, cy); }
        // a pigment paints a brush of blocks, or dyes the rug / banner / nest under the cursor
        Some(Thing::Item(it)) if lp && it.pigment().is_some() => { let r = brush_r(&g, sw::Brush::Place(it)); if let Err(e) = g.w.paint(cx, cy, it, r) { if lj { g.msg = format!("Can't paint: {e}"); } } }
        Some(Thing::Item(it)) if lj && it.furn().is_some() => { if let Err(e) = g.w.place_furniture(it, cx, cy) { g.w.refuse_place(it.name(), e); } }
        Some(Thing::Item(it)) if lj && molehill_sim::transport::is_machine(it) => { if let Err(e) = g.w.place_transport(it, cx, cy) { g.w.refuse_place(it.name(), e); } }
        Some(Thing::Item(Item::Torch)) if lj => { if let Err(e) = g.w.try_place_torch(cx, cy) { g.w.refuse_place("torch", e); } }
        // TNT goes down inert; right mouse lights it, or wire it to a lever / knot / listener
        Some(Thing::Item(Item::Tnt)) if lj => { if g.w.logic.dev[blk] != 0 { g.w.use_device(cx, cy); } else if let Err(e) = g.w.place_device(Dev::Tnt, cx, cy) { g.w.refuse_place("TNT", e); } }
        // pack items that are exactly one device place it (knots and thrummers come in several kinds: use their device slots)
        Some(Thing::Item(it @ (Item::Lamp | Item::Vent))) if lj => { let d = if it == Item::Lamp { Dev::Lamp } else { Dev::Vent };
            if g.w.logic.dev[blk] != 0 { g.w.use_device(cx, cy); } else if let Err(e) = g.w.place_device(d, cx, cy) { g.w.refuse_place(it.name(), e); } }
        Some(Thing::Item(Item::Spore)) if lj => { if let Err(e) = g.w.plant(cx, cy) { g.msg = format!("Can't plant: {e}"); } }
        Some(Thing::Dev(d)) if lj => {
            if g.w.logic.dev[blk] != 0 { g.w.use_device(cx, cy); }
            else if let Err(e) = g.w.place_device(DEVS[d], cx, cy) { g.w.refuse_place(DEVS[d].name(), e); }
        }
        Some(Thing::Station(k)) if lj => { let s = STATIONS[k]; if let Err(e) = g.w.place_building(s, cx.saturating_sub(Station::W / 2), cy) { g.msg = format!("Can't build: {e}"); } }
        _ => {}
    }
}

/// Render interpolation between the last two fixed sim steps, and a tight camera with a little lookahead.
/// Runs after this frame's steps, so the mole is drawn where this frame's input put it (no frame of lag).
fn follow(g: &mut Game, ct: &mut Transform, dt: f32) {
    { let a = g.acc.clamp(0., 1.); let pl = &g.w.pl; g.disp = Vec2::new(pl.ppx + (pl.px - pl.ppx) * a, pl.ppy + (pl.py - pl.ppy) * a - SF);
      // the lookahead eases (a turn doesn't whip the view); the follow itself stays tight
      let want = Vec2::new(g.w.pl.vx * 0.14, if g.w.pl.vy > 20. * SF { (g.w.pl.vy - 20. * SF) * 0.08 } else { 0. });
      let kl = 1. - (-7. * dt).exp(); let look = g.look + (want - g.look) * kl; g.look = look;
      let p = cell_to_world(g.disp.x + look.x, g.disp.y - 1.5 * SF + look.y); let k = 1. - (-30. * dt).exp();
      let d = Vec2::new(p.x - ct.translation.x, p.y - ct.translation.y);
      if d.length() > 200. { ct.translation.x = p.x; ct.translation.y = p.y; } else { ct.translation.x += d.x * k; ct.translation.y += d.y * k; } }
    let half = Vec2::new(W as f32 * CPX / 2., H as f32 * CPX / 2.);
    ct.translation.x = ct.translation.x.clamp(-half.x, half.x); { let fy = half.y + (FAR * CH) as f32 * CPX; ct.translation.y = ct.translation.y.clamp(-fy, fy); }
}

fn simulate(mut commands: Commands, mut g: ResMut<Game>, time: Res<Time>, mut cam: Query<(&mut Transform, &Projection), With<Camera2d>>, win: Query<&Window, With<PrimaryWindow>>) {
    g.frame += 1;
    if !g.paused {
        g.acc += time.delta_secs().min(0.1) * 60. * g.speed as f32;
        let now = time.elapsed_secs();
        let mut n = 0; while g.acc >= 1. && n < 3 * g.speed {
            let pre = std::mem::take(&mut g.w.events); let pm = std::mem::take(&mut g.w.marks); g.w.step();
            let mut e = std::mem::take(&mut g.w.events); let m = std::mem::take(&mut g.w.marks); let gg = &mut *g; gg.ev.extend(pre); gg.ev.append(&mut e);
            gg.pops.extend(pm.into_iter().chain(m).map(|(k, x, y, r)| (k, x, y, r, now))); g.acc -= 1.; n += 1;
            if g.jump_tap { g.jump_tap = false; g.w.input.jump = g.jump_key; } } // the tap was seen: back to the key's state
        g.pops.retain(|p| now - p.4 < 0.25);
        g.autosave += time.delta_secs();
        if g.autosave > 30. { g.autosave = 0.; save_auto(&mut g); }
        if g.acc > 4. { g.acc = 0.; }
    }
    let (Ok((mut ct, _)), Ok(win)) = (cam.single_mut(), win.single()) else { return };
    if g.w.oy != g.last_oy { // window slid: keep the camera on the same rock, redraw everything
        let d = (g.w.oy - g.last_oy) as f32; ct.translation.y += d * CPX; g.last_oy = g.w.oy;
        commands.insert_resource(make_bg(&g.w)); g.buf.iter_mut().for_each(|v| *v = 1); g.far_done.iter_mut().for_each(|v| *v = false); g.far_cells.clear();
    }
    // camera shake (quakes, explosions): remove last frame's offset, add a new one
    let off = g.shake_off; ct.translation.x -= off.x; ct.translation.y -= off.y;
    if !g.paused { follow(&mut g, &mut ct, time.delta_secs()); }
    let sh = g.w.shake; let f = g.frame as f32;
    g.shake_off = if sh > 0.05 { Vec2::new((f * 1.7).sin(), (f * 2.3).cos()) * sh * 3. * ct.scale.x.max(0.3) } else { Vec2::ZERO };
    ct.translation.x += g.shake_off.x; ct.translation.y += g.shake_off.y;
    let ct = &*ct;
    if g.frame - g.last_light >= 2 {
        g.last_light = g.frame;
        let r = view_rect(ct, win, 24 * S as i32);
        g.w.compute_light(r);
    }
}
fn view_rect(ct: &Transform, win: &Window, margin: i32) -> (usize, usize, usize, usize) {
    let sc = ct.scale.x; let hw = win.width() / 2. * sc; let hh = win.height() / 2. * sc;
    let (x0, y0) = world_to_cell(Vec2::new(ct.translation.x - hw, ct.translation.y + hh));
    let (x1, y1) = world_to_cell(Vec2::new(ct.translation.x + hw, ct.translation.y - hh));
    ((x0 - margin).clamp(0, W as i32 - 1) as usize, (y0 - margin).clamp(0, H as i32 - 1) as usize, (x1 + margin).clamp(0, W as i32 - 1) as usize, (y1 + margin).clamp(0, H as i32 - 1) as usize)
}

fn render_world(mut gm: ResMut<Game>, bg: Res<Bg>, mut images: ResMut<Assets<Image>>, cam: Query<&Transform, With<Camera2d>>, mut layers: Query<(&BgLayer, &mut Transform, &Sprite), Without<Camera2d>>, win: Query<&Window, With<PrimaryWindow>>) {
    let (Ok(ct), Ok(win)) = (cam.single(), win.single()) else { return };
    // parallax layers: re-upload when regenerated; slide continuously (a layer at factor f moves with f of the camera)
    for (l, mut t, spr) in &mut layers {
        let f = bg.layers[l.0].1; t.translation.x = ct.translation.x * f;
        if bg.is_changed() { if let Some(mut im) = images.get_mut(&spr.image) { if let Some(d) = im.data.as_mut() { if d.len() == bg.layers[l.0].0.len() { d.copy_from_slice(&bg.layers[l.0].0); } } } }
    }
    // `--dump-art` composites the whole window, so a site crop wider than the screen isn't half unrendered
    let (x0, y0, x1, y1) = if dumping() { (0, 0, W - 1, H - 1) } else { view_rect(ct, win, 2) };
    // snap the composited rect to whole chunks
    let (x0, y0) = (x0 / CH * CH, y0 / CH * CH); let (x1, y1) = (((x1 / CH) + 1) * CH - 1, ((y1 / CH) + 1) * CH - 1);
    let (x1, y1) = (x1.min(W - 1), y1.min(H - 1));
    let mut buf = std::mem::take(&mut gm.buf);
    let mut changed = vec![false; (W / CH) * (H / CH)];
    let g = &*gm;
    let lr = g.w.light_rect;
    let wd = &g.w; let gr = &wd.g;
    // premultiplied linear "over": out = bg·(1-a) + cp
    let over = |(cp, a): ([f32; 3], f32), h: [f32; 3], ha: f32| -> ([f32; 3], f32) { ([cp[0] * (1. - ha) + lin(h[0]) * ha, cp[1] * (1. - ha) + lin(h[1]) * ha, cp[2] * (1. - ha) + lin(h[2]) * ha], 1. - (1. - a) * (1. - ha)) };
    for y in y0..=y1 { for x in x0..=x1 {
        let i = y * W + x; let o = i * 4;
        let m = gr.m(i); let p = &PROPS[m as usize];
        let seen = gr.flags[i] & F_SEEN != 0;
        let px: [u8; 4];
        if !seen {
            let n = (hash3(x as u32 / (2 * S) as u32, y as u32 / (2 * S) as u32, 5) & 7) as u8;
            px = [14 + n, 12 + n, 17 + n, 255];
        } else {
            let lit = x >= lr.0 && x <= lr.2 && y >= lr.1 && y <= lr.3;
            let l = if lit { wd.light.at(i) } else { [0.05; 3] };
            // remembered rock stays faintly visible (a cool, dim memory); everything else comes from the sim's light
            let l = [l[0].max(0.15), l[1].max(0.15), l[2].max(0.18)];
            match p.kind {
                Kind::Empty | Kind::Gas => {
                    // the parallax rock shows through: darkened by the light here (never pure black), with a faint
                    // wash of coloured light, stale-air haze and steam laid over it. A background wall sits
                    // in front of that far rock and behind the player: darker, flatter, same light.
                    let lum = (l[0] * 0.3 + l[1] * 0.5 + l[2] * 0.2).min(1.3);
                    let k = (0.16 + 0.84 * (lum * 0.8).min(1.)) * 0.72;
                    let mut o = ([0.; 3], 1. - k.powf(2.2));
                    let wl = wd.walls.get(i);
                    if wl != molehill_sim::walls::Wall::None {
                        let wc = wl.color(); let sh = wl.shade(x as u32, (y as i64 + wd.oy) as u32);
                        // one darker/flatter scale, same light as the open cell: no extra 0.55 multiply
                        let lit = |c: u8, li: f32| {
                            let face = molehill_sim::walls::WALL_AMIN + (1. - molehill_sim::walls::WALL_AMIN) * li.min(1.15);
                            c as f32 * face * molehill_sim::walls::WALL_K * sh
                        };
                        o = over(o, [lit(wc[0], l[0]), lit(wc[1], l[1]), lit(wc[2], l[2])], 1. - wl.see_through());
                    }
                    let tw = (lum * 0.6).min(0.5) * 0.35;
                    if tw > 0.01 { let mx = l[0].max(l[1]).max(l[2]).max(1e-3); o = over(o, [l[0] / mx * 150. * k, l[1] / mx * 150. * k, l[2] / mx * 150. * k], tw); }
                    let co2 = wd.air.at(x, y); if co2 > 0.2 { o = over(o, [96. * k + 20., 104. * k + 20., 52. * k + 10.], ((co2 - 0.2) * 0.9).min(0.55)); }
                    if m == Mat::Steam { o = over(o, [200., 200., 210.], 0.4); }
                    if m == Mat::Fire { o = ([lin(255.), lin(150.), lin(50.)], 1.); }
                    let (cp, a) = o; let a = a.clamp(0., 1.);
                    px = if a < 0.004 { [0, 0, 0, 0] } else { [srgb(cp[0] / a).min(255.) as u8, srgb(cp[1] / a).min(255.) as u8, srgb(cp[2] / a).min(255.) as u8, (a * 255.).round() as u8] };
                }
                _ => {
                    let v = (hash3(x as u32, y as u32, m as u32) & 255) as f32 / 255.;
                    let v2 = (hash3(x as u32 / (3 * S) as u32, y as u32 / (2 * S) as u32, m as u32 + 77) & 255) as f32 / 255.;
                    let mut k = 0.86 + v * 0.14 + (v2 - 0.5) * 0.12;
                    if m == Mat::Grass { k *= 0.9 + 0.2 * v; }
                    // soft shading: lit top faces, shadowed undersides
                    let wy = (y as i64 + wd.oy) as u32;
                    if p.kind == Kind::Solid && !gr.loose(i) { k *= art::mat_tex(m, x as u32, wy, wd.tick); }
                    // rims: a bright lip on faces open to the top, a soft second row under it, shadowed undersides
                    // and slightly darker side walls, so every tunnel edge reads as a shaped edge
                    // (each band is one legacy tile, S cells, thick)
                    let open = |j: usize| !gr.is_solidish(j);
                    let up = |d: usize| y >= d && open(i - d * W);
                    if (1..=S).any(up) { k *= 1.2; }
                    else if (S + 1..=2 * S).any(up) { k *= 1.07; }
                    else if (1..=S).any(|d| y + d < H && open(i + d * W)) { k *= 0.74; }
                    else if (1..=S).any(|d| (x >= d && open(i - d)) || (x + d < W && open(i + d))) { k *= 0.9; }
                    if p.kind == Kind::Powder || gr.loose(i) { k *= 0.93 + 0.1 * v; }
                    if wd.st.pending[i] > 0 && (hash3(x as u32, y as u32, 9) % 3 == 0) { k *= 0.45; }
                    let emit = p.emit; let em = (emit[0] as u16 + emit[1] as u16 + emit[2] as u16) > 0;
                    let lm = |c: f32, li: f32| if em { c * (0.8 + 0.4 * v) } else { c * (li.min(1.25) * 0.94 + 0.06) };
                    // paint: the pigment takes over most of the colour; the material's pattern and shading stay
                    let pn = gr.paint(i); let base = if pn > 0 && p.kind == Kind::Solid { let pc = craft::PIGMENTS[pn as usize]; [0usize, 1, 2].map(|j| p.color[j] as f32 * 0.25 + pc[j] as f32 * 0.75) } else {
                        // rails and ore pockets draw their own two-tone pixel art (`art::mat_color`)
                        let above = (1..=2).take_while(|&d| y >= d && gr.m(i - d * W) == m).count() as u32;
                        art::mat_color(m, x as u32, wy, above).unwrap_or(p.color.map(|v| v as f32)) };
                    let mut c = [lm(base[0] * k, l[0]), lm(base[1] * k, l[1]), lm(base[2] * k, l[2])];
                    let mut alpha = 255.;
                    if m == Mat::Water { c = [lm(40. + 30. * v2, l[0]), lm(90. + 20. * v2, l[1]), lm(170., l[2])]; alpha = 255.; }
                    if m == Mat::Lava { let f = 0.85 + 0.3 * ((x as f32 * 0.3 / SF + y as f32 * 0.2 / SF + wd.tick as f32 * 0.05).sin() * 0.5 + 0.5); c = [255. * f, 110. * f, 30. * f]; }
                    if m == Mat::Glass { c = [c[0] * 0.6 + 20., c[1] * 0.6 + 26., c[2] * 0.6 + 30.]; }
                    px = [c[0].min(255.) as u8, c[1].min(255.) as u8, c[2].min(255.) as u8, alpha as u8];
                }
            }
        }
        let mut px = px;
        // copperline and devices are sprites (wiring.rs), drawn over the world
        if buf[o..o + 4] != px { buf[o..o + 4].copy_from_slice(&px); changed[(y / CH) * (W / CH) + x / CH] = true; }
    } }
    // blasts: the cells a blast threw, in flight (the wave itself shows only through the dust it carries)
    let inview = |x: usize, y: usize| x >= x0 && x <= x1 && y >= y0 && y <= y1;
    let blend = |buf: &mut [u8], o: usize, c: [f32; 3], a: f32| { let a = a.clamp(0., 1.); for j in 0..3 { buf[o + j] = (buf[o + j] as f32 * (1. - a) + c[j] * a).min(255.) as u8; } buf[o + 3] = (buf[o + 3] as f32 + (255. - buf[o + 3] as f32) * a) as u8; };
    let acc = g.acc.clamp(0., 1.);
    for d in &wd.debris {
        let (fx, fy) = (d.ppx + (d.px - d.ppx) * acc, d.ppy + (d.py - d.ppy) * acc); if fx < 0. || fy < 0. { continue; }
        let (x, y) = (fx as usize, fy as usize); if x >= W || y >= H || !inview(x, y) { continue; }
        let i = y * W + x; let pc = PROPS[d.mat as usize].color; let l = wd.light.at(i); let lum = (l[0] * 0.3 + l[1] * 0.5 + l[2] * 0.2).clamp(0.35, 1.2);
        let c = if Mat::from_u8(d.mat) == Mat::Water { [70. * lum + 30., 120. * lum + 30., 200. * lum + 20.] } else { [pc[0] as f32 * lum, pc[1] as f32 * lum, pc[2] as f32 * lum] };
        blend(&mut buf, i * 4, c, 1.); changed[(y / CH) * (W / CH) + x / CH] = true;
    }
    let mut n = 0;
    for (k, &c) in changed.iter().enumerate() {
        if !c { continue; } n += 1;
        let (cx, cy) = (k % (W / CH), k / (W / CH));
        let Some(mut im) = images.get_mut(&gm.chunks[k]) else { continue };
        let Some(data) = im.data.as_mut() else { continue };
        for r in 0..CH { let src = ((cy * CH + r) * W + cx * CH) * 4; data[r * CH * 4..(r + 1) * CH * 4].copy_from_slice(&buf[src..src + CH * 4]); }
    }
    gm.uploads = n; gm.buf = buf;
    render_far(&mut gm, ct, win, &mut images);
}

/// Beyond the simulated window: rows the colony has seen are drawn from the row cache (dimmed, frozen);
/// the rest is the Deepholt's rock as a mole senses it through the stone (dim silhouettes of caves).
fn render_far(gm: &mut Game, ct: &Transform, win: &Window, images: &mut Assets<Image>) {
    let sc = ct.scale.x; let (hw, hh) = (win.width() / 2. * sc, win.height() / 2. * sc);
    let (vx0, vy0) = world_to_cell(Vec2::new(ct.translation.x - hw, ct.translation.y + hh));
    let (vx1, vy1) = world_to_cell(Vec2::new(ct.translation.x + hw, ct.translation.y - hh));
    let mut budget = 3;
    for k in 0..gm.far.len() {
        if gm.far_done[k] { continue; }
        let (b, cx) = (k / (W / CH), k % (W / CH));
        let y0 = if b < FAR { (b as i32 - FAR as i32) * CH as i32 } else { (H + (b - FAR) * CH) as i32 };
        let x0 = (cx * CH) as i32;
        let m8 = 8 * S as i32; if x0 + CH as i32 <= vx0 - m8 || x0 > vx1 + m8 || y0 + CH as i32 <= vy0 - m8 || y0 > vy1 + m8 { continue; }
        if budget == 0 { break; } budget -= 1;
        let Game { w, far_cells, .. } = &mut *gm; let mut data = vec![0u8; CH * CH * 4];
        for r in 0..CH { let wy = w.oy + (y0 + r as i32) as i64; let s0 = molehill_sim::band::band_start(w.oy, wy);
            let row = far_cells.entry(s0).or_insert_with(|| w.stored_cells(s0)).as_ref().map(|(m, f)| { let o = (wy - s0) as usize * W; (&m[o..o + W], &f[o..o + W]) });
            for c in 0..CH { let x = cx * CH + c;
                let (m, seen) = match row { Some((m, f)) => (m[x], f[x] & F_SEEN != 0), None => (w.gen.cell(x, wy, W).0, false) };
                let mm = Mat::from_u8(m); let p = &PROPS[m as usize]; let v = (hash3(x as u32, wy as u32, m as u32) & 255) as f32 / 255.;
                let px: [f32; 3] = if seen {
                    match p.kind { Kind::Empty | Kind::Gas => [30., 24., 22.], _ => { let k = 0.5 + 0.08 * v; [p.color[0] as f32 * k, p.color[1] as f32 * k, p.color[2] as f32 * k] } }
                } else {
                    let n = (hash3(x as u32 / (2 * S) as u32, wy as u32 / (2 * S) as u32, 5) & 7) as f32;
                    match p.kind { Kind::Empty | Kind::Gas => [8. + n, 7. + n, 11. + n], _ if mm == Mat::Lava => [70. + n, 30., 16.], _ if mm == Mat::Water => [16., 24. + n, 44.],
                        _ => { let k = 0.16 + 0.03 * v; [p.color[0] as f32 * k + 12. + n, p.color[1] as f32 * k + 10. + n, p.color[2] as f32 * k + 15. + n] } }
                };
                let o = (r * CH + c) * 4; data[o] = px[0] as u8; data[o + 1] = px[1] as u8; data[o + 2] = px[2] as u8; data[o + 3] = 255;
            } }
        if let Some(mut im) = images.get_mut(&gm.far[k]) { if let Some(d) = im.data.as_mut() { d.copy_from_slice(&data); } }
        gm.far_done[k] = true;
    }
}

/// Frame-to-frame feedback state for the mole sprite and the little particles (dig chips, landing dust).
#[derive(Default)] struct Fx { waves_seen: u64, grounded: bool, vy: f32, squash: f32, parts: Vec<Part>, last_mine: Option<[f32; 3]>, chip_t: f32, dust_t: f32, wet: bool, ember_t: f32, deny: bool, deny_t: f32, props: Vec<props::Placed>, props_key: (i64, Vec<u32>) }
/// A pixel particle; `g` is its gravity (negative drifts up, like embers).
struct Part { p: Vec2, v: Vec2, life: f32, max: f32, c: [f32; 3], size: f32, g: f32, drag: f32, al: f32 }
/// How far up (cells) a floor object's centre sits: half its height in world units.
fn sz_up(s: Vec2) -> f32 { s.y / 2. / CPX }
/// A held item takes the light where the mole stands, like the mole does.
fn c_hold(w: &World, m: &molehill_sim::world::Mole) -> Color { let l = w.light.at((m.y.saturating_sub(S)).min(H - 1) * W + m.x.min(W - 1)); let v = (l[0] * 0.3 + l[1] * 0.5 + l[2] * 0.2).clamp(0.5, 1.); Color::srgb(v, v, v) }
fn sync_sprites(mut commands: Commands, g: Res<Game>, sp: Res<Sprites>, time: Res<Time>, mut fx: Local<Fx>, mut q: Query<(Entity, &Pooled, &mut Transform, &mut Sprite, &mut Visibility)>) {
    // kind 0 moles, 22 found-place props, 12 throw arc, 16/17 furniture and its cloth, 18 furniture ghost, 19 Lid clues, 1 drops, 2 chests, 3 stations, 4 torches, 5 mushroom plots, 7 aim, 8 particles, 9 torch halos, 10 lamp halos, 11 placement pops, 14 held item, 15 headlamp lens halo
    let dt = if g.paused { 0. } else { time.delta_secs().min(0.05) }; let now = time.elapsed_secs();
    let w = &g.w;
    let mut want: Vec<(u8, usize, Vec2, f32, Handle<Image>, Vec2, bool, Color)> = vec![];
    let lightf = |x: usize, y: usize| { let l = w.light.at(y.min(H - 1) * W + x.min(W - 1)); let v = (l[0] * 0.3 + l[1] * 0.5 + l[2] * 0.2).clamp(0.12, 1.2); Color::srgb(v.min(1.) , v.min(1.), v.min(1.)) };
    let pl = &w.pl;
    // squash on landing (harder landings squash more), stretch on take-off; springs back in ~0.12 s
    if !g.paused {
        if pl.grounded && !fx.grounded && fx.vy > 12. * SF { fx.squash = -(fx.vy / (60. * SF)).clamp(0.12, 0.3); let at = cell_to_world(g.disp.x, g.disp.y + 2. * SF); for k in 0..6 { let s = if k % 2 == 0 { 1. } else { -1. }; fx.parts.push(Part { p: at + Vec2::new(s * 3., 0.), v: Vec2::new(s * (14. + k as f32 * 6.), 6. + k as f32 * 2.), life: 0., max: 0.3, c: [150., 130., 120.], size: 1.6, g: 260., drag: 0., al: 1. }); } }
        if !pl.grounded && pl.cling == 0 && pl.vy < -10. * SF && fx.vy > -5. * SF { fx.squash = 0.18; } // any sudden kick upward: a jump off the ground or a wall
        fx.grounded = pl.grounded; fx.vy = pl.vy; fx.squash *= (-dt * 18.).exp();
    }
    for (k, m) in w.moles.iter().enumerate() { if !m.alive || k == 0 && w.tr.in_tube.is_some() { continue; }
        let c = if k == 0 && w.pl.flash > 0. { if (w.pl.flash * 30.) as u32 % 2 == 0 { Color::srgb(6., 6., 6.) } else { Color::srgb(1.6, 0.35, 0.3) } } // hit flash: white / red strobe
            else if m.hp < 40. { Color::srgb(1., 0.6, 0.6) } else { let l = lightf(m.x, m.y - S).to_srgba(); let v = l.red.max(0.55); Color::srgb(v, v, v) };
        // pose: dig > climb > air > run > idle. Run and climb phases follow distance moved, so feet don't skate.
        let (pose, ph) = if pl.mine.is_some() { (art::Pose::Dig, (now * 14.) as u32) }
            else if w.tr.climbing || w.tr.on_rope.is_some() { (art::Pose::Climb, (pl.py / (1.2 * SF)).abs() as u32) }
            else if !pl.grounded { (if pl.vy < 2. * SF { art::Pose::Jump } else { art::Pose::Fall }, 0) }
            else if pl.vx.abs() > 0.5 * SF { (art::Pose::Run, (pl.px / (1.1 * SF)).abs() as u32) }
            else { (art::Pose::Idle, (now * 1.6) as u32) };
        let img = if w.gear.lamp { &sp.mole } else { &sp.mole_out }[art::mole_frame(pose, ph)].clone();
        let sq = fx.squash; let scale = Vec2::new(1. - sq * 0.8, 1. + sq);
        if pose == art::Pose::Climb {
            // nose up, belly to the wall: 20×26, hugging the wall face on the cling side
            let size = Vec2::new(20., 26.) * scale; let cx = g.disp.x - pl.cling as f32 * 1.4 * SF;
            want.push((0, k, cell_to_world(cx, g.disp.y + SF - 2.25 * SF), 5., img, size, pl.cling < 0, c));
        } else {
            // 26×20 with the feet on the floor line; squash keeps the feet planted
            let size = Vec2::new(26., 20.) * scale; let up = (size.y - 20.) / 2. / CPX;
            want.push((0, k, cell_to_world(g.disp.x, g.disp.y + SF - 1.5 * SF - up), 5., img, size, m.facing < 0, c));
        } }
    // the lit headlamp's lens gets a small warm halo (the beam itself is the sim's light)
    if let (Some(m), true) = (w.moles.first().filter(|m| m.alive), w.lamp_lit()) { // (a dry lamp shows no halo)
        let lens = if pl.cling != 0 { cell_to_world(g.disp.x - pl.cling as f32 * 1.4 * SF, g.disp.y + SF - 2.25 * SF) + Vec2::new(-pl.cling as f32 * 4., 12.) }
            else { cell_to_world(g.disp.x, g.disp.y + SF - 1.5 * SF) + Vec2::new(m.facing as f32 * 11., 4.) };
        want.push((15, 0, lens, 5.2, sp.glow.clone(), Vec2::splat(14.), false, Color::srgba(1., 0.93, 0.62, 0.45)));
    }
    // the item in the paws: turned toward the aim (snapped to 15° so the pixels stay clean), swung while
    // a pickaxe digs, shaken while a drill or bore runs, thrust out when a block or charge goes down.
    // A torch is held upright with its flame. Nothing is held while climbing (the paws are on the wall).
    let mut rot: std::collections::HashMap<(u8, usize), f32> = Default::default();
    if let (Some(m), true) = (w.moles.first().filter(|m| m.alive), pl.cling == 0) {
        use std::f32::consts::PI;
        let f = m.facing as f32; let paw = cell_to_world(g.disp.x, g.disp.y + SF - 1.5 * SF) + Vec2::new(f * 6., -4.);
        let mut ang = g.aim.map(|(x, y, _)| (cell_to_world(x as f32 + 0.5, y as f32 + 0.5) - paw).to_angle()).unwrap_or(if f > 0. { -0.3 } else { PI + 0.3 });
        if ang.cos() * f < -0.1 { ang = PI - ang; } // keep it on the side the mole faces
        let mining = pl.mine.is_some(); let jig = |k: u32| ((hash3(g.frame as u32, k, 21) & 255) as f32 / 255. - 0.5);
        let thrust = g.pops.iter().filter(|p| p.0 == 1).map(|p| 1. - ((now - p.4) / 0.15).clamp(0., 1.)).fold(0f32, f32::max);
        let item = |h: usize, wpx: f32, hpx: f32| (sp.held[h].clone(), Vec2::new(wpx, hpx));
        let held = match g.slots[g.hot] { Some(Thing::Pick(n)) => Some((item((n as usize).clamp(1, 3) - 1, [12., 14., 16.][(n as usize).clamp(1, 3) - 1], [11., 7., 9.][(n as usize).clamp(1, 3) - 1]), n == 1)),
            Some(Thing::Item(Item::Tnt)) => Some((item(3, 7., 8.), false)), Some(Thing::Item(Item::WoodPillar)) => Some((item(4, 6., 6.), false)),
            Some(Thing::Item(Item::Brick)) => Some((item(5, 6., 6.), false)), Some(Thing::Item(Item::IronBeam)) => Some((item(6, 6., 6.), false)),
            Some(Thing::Item(Item::RopeGun)) => Some((item(if w.tr.hook.is_none() && w.inv[Item::HookRope as usize] > 0 { 7 } else { 8 }, 16., 11.), false)), _ => None };
        if let Some(((img, size), pick)) = held {
            let swing = if pick && mining { let p = (now * 5.).fract(); if p < 0.55 { 1.2 * p / 0.55 } else { 1.2 - 1.7 * (p - 0.55) / 0.45 } } else { 0. };
            let mut a = ang + f * swing; let shake = if mining && !pick { 1. } else { 0. }; a += shake * jig(1) * 0.12;
            let a = (a / (PI / 12.)).round() * (PI / 12.); let dir = Vec2::from_angle(a);
            let pos = paw + dir * (size.x / 2. - 1. + 3. * thrust) + Vec2::new(jig(2), jig(3)) * 1.4 * shake;
            rot.insert((14, 0), if f < 0. { a - PI } else { a }); want.push((14, 0, pos, 5.1, img, size, f < 0., c_hold(&w, m)));
        } else if g.slots[g.hot] == Some(Thing::Item(Item::Torch)) {
            let tilt = ((ang - PI / 2.).sin() * -0.35).clamp(-0.35, 0.35); let fr = ((now * 10.) as usize) % 3;
            rot.insert((14, 0), tilt); want.push((14, 0, paw + Vec2::new(f * 1., 5.), 5.1, sp.torch_f[fr].clone(), Vec2::new(8., 12.), false, Color::WHITE));
        } else if let Some(Thing::Item(it)) = g.slots[g.hot] { want.push((14, 0, paw + Vec2::new(f * 2., 1.), 5.1, sp.icons[it as usize].clone(), Vec2::splat(8.), false, c_hold(&w, m))); }
    }
    // dig chips: while a bite is in progress, flecks of the target's material spray toward the mole
    if let (Some((x, y)), false) = (g.aim.filter(|_| pl.mine.is_some()).map(|(x, y, _)| (x, y)), g.paused) {
        let m = w.g.m(y * W + x); let col = if m == Mat::Empty { [120., 100., 80.] } else { let c = PROPS[m as usize].color; [c[0] as f32, c[1] as f32, c[2] as f32] };
        if m != Mat::Empty { fx.last_mine = Some(col); } fx.chip_t += dt;
        while fx.chip_t > 0.03 { fx.chip_t -= 0.03;
            let h = hash3(g.frame as u32, fx.parts.len() as u32, 3); let r = |k: u32| ((h >> k) & 255) as f32 / 255. - 0.5;
            let dir = (cell_to_world(g.disp.x, g.disp.y) - cell_to_world(x as f32 + 0.5, y as f32 + 0.5)).normalize_or_zero();
            fx.parts.push(Part { p: cell_to_world(x as f32 + 0.5, y as f32 + 0.5) + Vec2::new(r(0), r(8)) * 6., v: dir * 40. + Vec2::new(r(16) * 60., 30. + r(24) * 40.), life: 0., max: 0.35 + r(4) * 0.2, c: col, size: 1.2 + (h & 1) as f32 * 0.8, g: 260., drag: 0., al: 1. }); }
    } else { fx.chip_t = 0.; }
    // a refused dig (`World::dig_refusal`; the sim clinks and shakes): pale sparks glance off the face and the
    // cold ring jolts, so the no is seen and felt at the cursor, not only read in the log
    let deny = pl.deny.is_some();
    if deny && !fx.deny { fx.deny_t = now; if let Some((x, y, _)) = g.aim {
        let at = cell_to_world(x as f32 + 0.5, y as f32 + 0.5); let away = (cell_to_world(g.disp.x, g.disp.y) - at).normalize_or_zero();
        for k in 0..7u32 { let h = hash3(g.frame as u32, k, 5); let r = |b: u32| ((h >> b) & 255) as f32 / 255. - 0.5;
            fx.parts.push(Part { p: at + Vec2::new(r(0), r(8)) * 4., v: away * 70. + Vec2::new(r(16) * 90., 40. + r(24) * 50.), life: 0., max: 0.22 + r(4).abs() * 0.15, c: [205., 212., 228.], size: 1.1, g: 300., drag: 0., al: 1. }); } } }
    fx.deny = deny;
    // a burst when a bite breaks through
    // (the sim marks the bite the tick it clears; `Event::Dig` also sounds the strike on press, so it can't be the trigger)
    if let Some(&(_, bx, by, _, _)) = g.pops.iter().find(|p| p.0 == 0 && p.4 == now) { if let Some(col) = fx.last_mine.take() {
        for k in 0..10u32 { let a = k as f32 / 10. * std::f32::consts::TAU; fx.parts.push(Part { p: cell_to_world(bx, by), v: Vec2::new(a.cos(), a.sin().abs() + 0.3) * 55., life: 0., max: 0.4, c: col, size: 1.8, g: 260., drag: 0., al: 1. }); } } }
    if !g.paused {
        // run dust: little puffs of the floor's colour kicked back from the feet while running
        let floor = w.g.m((pl.py as usize + 1).min(H - 1) * W + (pl.px as usize).min(W - 1));
        if pl.grounded && pl.vx.abs() > 6. * SF && floor != Mat::Empty { fx.dust_t += dt; } else { fx.dust_t = 0.; }
        while fx.dust_t > 0.11 { fx.dust_t -= 0.11; let c = PROPS[floor as usize].color; let h = hash3(g.frame as u32, 7, 11); let r = ((h & 255) as f32 / 255.) - 0.5;
            fx.parts.push(Part { p: cell_to_world(g.disp.x, g.disp.y + SF) + Vec2::new(-pl.vx.signum() * 4., 1.), v: Vec2::new(-pl.vx.signum() * (10. + r * 8.), 12. + r * 6.), life: 0., max: 0.28, c: [c[0] as f32 * 0.8 + 30., c[1] as f32 * 0.8 + 30., c[2] as f32 * 0.8 + 30.], size: 1.4, g: 90., drag: 0., al: 1. }); }
        // a roof giving warning: dust sifts from strained rock (loaded past capacity, still holding) now and
        // then, and faster from rock that has started to crack (it lets go in 1.5 s)
        let cracking = w.st.cracking.iter().filter(|&&i| (i as usize % W).abs_diff(pl.px as usize) < 20 * molehill_sim::mat::CPM).take(64).copied();
        for (k, i) in w.strain.iter().copied().map(|i| (0u32, i)).chain(cracking.map(|i| (1u32, i))) {
            let h = hash3(i, g.frame as u32, 23); if h % 1000 >= if k == 0 { 6 } else { 40 } { continue; }
            let (x, y) = ((i as usize % W) as f32, (i as usize / W) as f32 + 1.); let c = PROPS[w.g.mat[i as usize] as usize].color; let r = ((h >> 10) & 255) as f32 / 255. - 0.5;
            fx.parts.push(Part { p: cell_to_world(x + 0.5 + r, y), v: Vec2::new(r * 4., -2.), life: 0., max: 1.1, c: [c[0] as f32 * 0.7 + 40., c[1] as f32 * 0.7 + 38., c[2] as f32 * 0.7 + 34.], size: 1.1, g: 45., drag: 0.6, al: 0.8 }); }
        // splash: entering water (or lava) throws a crown of droplets
        let body = w.g.m(((pl.py - SF) as usize).min(H - 1) * W + (pl.px as usize).min(W - 1)); let wet = matches!(body, Mat::Water | Mat::Lava);
        if wet && !fx.wet && pl.vy.abs() > 4. * SF { let c = if body == Mat::Lava { [255., 140., 40.] } else { [120., 170., 230.] };
            for k in 0..12u32 { let s = if k % 2 == 0 { 1. } else { -1. }; let h = hash3(g.frame as u32, k, 13); let r = ((h & 255) as f32 / 255.) - 0.5;
                fx.parts.push(Part { p: cell_to_world(g.disp.x, g.disp.y - 0.5 * SF) + Vec2::new(s * (2. + k as f32 * 0.6), 0.), v: Vec2::new(s * (12. + k as f32 * 4. + r * 10.), 50. + r * 30. + (pl.vy.abs() / SF * 1.5).min(40.)), life: 0., max: 0.5, c, size: 1.5, g: 300., drag: 0., al: 1. }); } }
        fx.wet = wet;
        // embers: torches shed the odd spark that drifts up and winks out
        fx.ember_t += dt * w.torches.len().min(12) as f32;
        while fx.ember_t > 0.5 { fx.ember_t -= 0.5; let h = hash3(g.frame as u32, fx.parts.len() as u32, 17); if w.torches.is_empty() { break; }
            let (x, y) = w.torches[h as usize % w.torches.len()]; let r = (((h >> 8) & 255) as f32 / 255.) - 0.5;
            fx.parts.push(Part { p: cell_to_world(x as f32 + 0.5, y as f32 - 0.3 * SF), v: Vec2::new(r * 14., 10. + r.abs() * 10.), life: 0., max: 0.9, c: [255., 180., 90.], size: 0.9, g: -14., drag: 0., al: 1. }); }
    }
    // blasts: a puff of smoke at the charge, and dust raised by the pressure front that rides it out along
    // the tunnels, slows and settles (render only; the front itself is drawn in render_world)
    if !g.paused { for wv in &w.waves {
        let age = wv.age(w.tick);
        if wv.tick > fx.waves_seen { fx.waves_seen = wv.tick; let at = cell_to_world(wv.cx as f32 + 0.5, wv.cy as f32 + 0.5);
            for k in 0..10u32 { let h = hash3(wv.tick as u32, k, 41); let a = k as f32 / 10. * std::f32::consts::TAU; let r = ((h & 255) as f32 / 255.) * 0.8 + 0.4;
                fx.parts.push(Part { p: at, v: Vec2::new(a.cos(), a.sin()) * 90. * r, life: 0., max: 1.6 + r, c: [90. + 30. * r, 86. + 30. * r, 80. + 28. * r], size: 2. + 2. * r, g: -10., drag: 3.5, al: 0.3 }); } }
        if age > 1.2 { continue; }
        let spd = molehill_sim::debris::WAVE_SHOW_MPS * molehill_sim::mat::CPM as f32 * CPX; let mut n = 0;
        for c in wv.front(w.tick) { let i = c.i as usize; let h = hash3(c.i, g.frame as u32, 43); if h % 29 != 0 || c.p < 25. { continue; }
            let (x, y) = (i % W, i / W); if w.g.flags[i] & F_SEEN == 0 { continue; }
            // the dust takes the colour of the rock it was knocked off (or a grey haze in the open)
            let near = [1i64, -1, W as i64, -(W as i64)].iter().map(|o| (i as i64 + o).clamp(0, (W * H - 1) as i64) as usize).map(|j| w.g.m(j)).find(|m| m.kind() != Kind::Empty && m.kind() != Kind::Gas && m.kind() != Kind::Liquid);
            let col = near.map_or([150., 145., 135.], |m| { let k = PROPS[m as usize].color; [k[0] as f32 * 0.6 + 60., k[1] as f32 * 0.6 + 58., k[2] as f32 * 0.6 + 55.] });
            let l = ((c.dx as f32).powi(2) + (c.dy as f32).powi(2)).sqrt().max(1.); let dir = Vec2::new(c.dx as f32 / l, -(c.dy as f32) / l);
            let r = ((h >> 8) & 255) as f32 / 255.;
            fx.parts.push(Part { p: cell_to_world(x as f32 + 0.5, y as f32 + 0.5), v: dir * spd * (0.35 + 0.3 * r) + Vec2::new(0., 10. * r), life: 0., max: 0.8 + 0.6 * r, c: col, size: 1. + 1. * r, g: 22., drag: 2.6, al: ((c.p / 300.).sqrt()).clamp(0.15, 0.45) });
            n += 1; if n >= 14 { break; } }
    } }
    for p in fx.parts.iter_mut() { p.life += dt; p.v.y -= p.g * dt; p.v *= (-p.drag * dt).exp(); p.p += p.v * dt; }
    fx.parts.retain(|p| p.life < p.max); while fx.parts.len() > 300 { fx.parts.remove(0); }
    for (k, p) in fx.parts.iter().enumerate() { let a = (1. - p.life / p.max) * p.al; want.push((8, k, p.p, 5.5, sp.px.clone(), Vec2::splat(p.size), false, Color::srgba(p.c[0] / 255., p.c[1] / 255., p.c[2] / 255., a))); }
    // loose items: drawn between their last two sim positions (they fly now); a lit charge blinks
    let a = g.acc.clamp(0., 1.);
    for (k, d) in w.drops.iter().enumerate() { if w.g.flags[d.y * W + d.x] & F_SEEN == 0 { continue; }
        let c = if d.fuse > 0 && (d.fuse / 6) % 2 == 0 { Color::srgb(2.5, 0.8, 0.5) } else { lightf(d.x, d.y) };
        want.push((1, k, cell_to_world(d.ppx + (d.px - d.ppx) * a, d.ppy + (d.py - d.ppy) * a), 4., sp.icons[d.item as usize].clone(), Vec2::splat(8.), false, c)); }
    // charging a throw: dots along the arc it would fly (until it meets rock), brighter with more charge
    if let (Some(ch), false) = (g.charge, g.paused) { let t = sw::TUNE; let f = ch / THROW_CHARGE;
        let (ax, ay) = g.throw_aim; let l = (ax * ax + ay * ay).sqrt().max(1e-3); let sp0 = molehill_sim::items::THROW_MIN + (molehill_sim::items::THROW_MAX - molehill_sim::items::THROW_MIN) * f;
        let (mut x, mut y) = (pl.px, pl.py - sw::BODY_H * 0.7); let (mut vx, mut vy) = (ax / l * sp0 + pl.vx * 0.5, ay / l * sp0 + pl.vy.min(0.) * 0.5);
        for k in 0..40 { let dt = 1. / 60.; for _ in 0..3 { vy = (vy + t.gravity * dt).min(t.terminal); x += vx * dt; y += vy * dt; }
            if x < 0. || y < 0. || x as usize >= W || y as usize >= H || w.g.is_solidish(y as usize * W + x as usize) { break; }
            want.push((12, k, cell_to_world(x, y), 6., sp.px.clone(), Vec2::splat(CPX * SF * 0.6), false, Color::srgba(1., 0.95, 0.8, 0.25 + 0.5 * f))); } }
    // mine carts: a plank tub with a bronze rim on two iron wheels (`art::cart`), drawn between their last two sim positions;
    // its cargo heaped over the rim; the mole riding sits in it (the box is drawn in front of it)
    for (k, c) in w.tr.carts.iter().enumerate() {
        let (x, y) = (c.px + (c.x - c.px) * a, c.py + (c.y - c.py) * a); let (cx, cy) = (x as usize, (y as usize).saturating_sub(1));
        if cx >= W || cy >= H || w.g.flags[cy * W + cx] & F_SEEN == 0 { continue; }
        let l = lightf(cx, cy).to_srgba(); let lit = |r: f32, gg: f32, b: f32| Color::srgb(r * l.red, gg * l.green, b * l.blue);
        let (cw, ch) = (molehill_sim::transport::CART_W * CPX + 4., molehill_sim::transport::CART_H * CPX);
        let z = if w.tr.riding == Some(k) { 5.2 } else { 4.2 };
        let mid = cell_to_world(x, y) + Vec2::new(0., ch / 2. + 2.);
        want.push((30, k, cell_to_world(x, y) + Vec2::new(0., 5.), z, sp.cart.clone(), Vec2::new(20., 10.), false, lit(1., 1., 1.)));
        if let Some(&(it, _)) = c.cargo.first() { let col = it.color(); let n = c.load().min(40) as f32 / 40.;
            want.push((32, k, mid + Vec2::new(0., ch / 2. + 1. + 2. * n), z - 0.02, sp.px.clone(), Vec2::new(cw - 4., 2. + 4. * n), false, lit(col[0] as f32 / 255., col[1] as f32 / 255., col[2] as f32 / 255.))); }
    }
    // rope ladders: two fibre ropes with wooden rungs every 0.25 m, lit where they hang
    for (k, l) in w.tr.ladders.iter().enumerate() {
        if w.g.flags[l.top.min(H - 1) * W + l.x.min(W - 1)] & F_SEEN == 0 { continue; }
        let lt = lightf(l.x, l.top + l.len / 2).to_srgba(); let rope = Color::srgb(0.62 * lt.red, 0.5 * lt.green, 0.32 * lt.blue); let rung = Color::srgb(0.55 * lt.red, 0.38 * lt.green, 0.22 * lt.blue);
        let (xc, y0, len) = (l.x as f32 + 0.5, l.top as f32, l.len as f32);
        let mid = cell_to_world(xc, y0 + len / 2.);
        for (j, s) in [-1f32, 1.].into_iter().enumerate() { want.push((34, k * 2 + j, mid + Vec2::new(s * 0.9 * SF * CPX, 0.), 3.9, sp.px.clone(), Vec2::new(1., len * CPX), false, rope)); }
        for j in 0..(l.len / S) { want.push((35, k * 64 + j, cell_to_world(xc, y0 + (j * S) as f32 + 0.5 * SF), 3.95, sp.px.clone(), Vec2::new(1.8 * SF * CPX, 1.), false, rung)); }
    }
    // rope gun art: each rope a chain of twisted-fibre lengths, one per verlet segment, turned along it; the stone
    // hook at its head points the way the rope runs into the rock it bit (or, in flight, the way it flies)
    let rope_col = |x: f32, y: f32| { let lt = lightf((x.max(0.) as usize).min(W - 1), (y.max(0.) as usize).min(H - 1)).to_srgba(); Color::srgb(lt.red, lt.green, lt.blue) };
    let chains = w.tr.ropes.iter().map(|r| (&r.pts, None)).chain(w.tr.hook.iter().map(|h| (&h.pts, Some((h.vx, h.vy)))));
    for (k, (pts, flying)) in chains.enumerate() {
        let (hx, hy) = pts[0]; if w.g.flags[(hy.max(0.) as usize).min(H - 1) * W + (hx.max(0.) as usize).min(W - 1)] & F_SEEN == 0 && flying.is_none() { continue; }
        for (n, s) in pts.windows(2).enumerate() { let (a0, a1) = (cell_to_world(s[0].0, s[0].1), cell_to_world(s[1].0, s[1].1)); let d = a1 - a0; let len = d.length(); if len < 0.1 { continue; }
            let id = k * 64 + n; rot.insert((51, id), d.to_angle());
            want.push((51, id, (a0 + a1) / 2., 3.92, sp.rope[0].clone(), Vec2::new(len + 0.6, 1.6), false, rope_col((s[0].0 + s[1].0) / 2., (s[0].1 + s[1].1) / 2.))); }
        let dir = match flying { Some((vx, vy)) if vx * vx + vy * vy > 1. => Vec2::new(vx, -vy), _ => pts.get(1).map(|&(x, y)| cell_to_world(hx, hy) - cell_to_world(x, y)).unwrap_or(Vec2::X) };
        rot.insert((52, k), dir.to_angle());
        want.push((52, k, cell_to_world(hx, hy), 3.96, sp.rope[1].clone(), Vec2::new(8., 7.), false, rope_col(hx, hy)));
    }
    // lifts: the head frame under the ceiling, the rope down to the cage floor, the cage's rails, and the
    // counterweight on its own rope beside the shaft wall (it rises as the cage sinks)
    for (k, l) in w.tr.lifts.iter().enumerate() {
        if w.g.flags[l.head.min(H - 1) * W + l.x.min(W - 1)] & F_SEEN == 0 { continue; }
        let y = l.py + (l.y - l.py) * a; let (x0, _, x1, _) = l.cage(); let xc = (x0 + x1) / 2.; let cw = x1 - x0;
        let lt = lightf(l.x, l.head + 2).to_srgba(); let lit = |r: f32, gg: f32, b: f32| Color::srgb(r * lt.red, gg * lt.green, b * lt.blue);
        let head = l.head as f32;
        want.push((36, k, cell_to_world(xc, head + 0.5 * SF), 3.9, sp.px.clone(), Vec2::new(cw * CPX + 4., SF * CPX), false, lit(0.45, 0.33, 0.2)));
        want.push((37, k, cell_to_world(xc, head + SF), 3.95, sp.px.clone(), Vec2::splat(SF * CPX * 1.2), false, lit(0.6, 0.6, 0.66)));
        want.push((38, k, cell_to_world(xc, (head + SF + y) / 2.), 3.8, sp.px.clone(), Vec2::new(1., (y - head - SF).max(0.) * CPX), false, lit(0.55, 0.48, 0.36)));
        let cl = lightf(xc as usize, (y as usize).saturating_sub(1)).to_srgba(); let cage = |r: f32, gg: f32, b: f32| Color::srgb(r * cl.red, gg * cl.green, b * cl.blue);
        want.push((39, k, cell_to_world(xc, y + 0.5), 4.3, sp.px.clone(), Vec2::new(cw * CPX, CPX), false, cage(0.62, 0.6, 0.62)));
        for (j, s) in [-1f32, 1.].into_iter().enumerate() { want.push((40, k * 2 + j, cell_to_world(xc + s * (cw / 2. - 0.5), y - 1.5 * SF), 4.3, sp.px.clone(), Vec2::new(1.5, 3. * SF * CPX), false, cage(0.5, 0.5, 0.55))); }
        want.push((41, k, cell_to_world(xc, y - 3. * SF), 4.3, sp.px.clone(), Vec2::new(cw * CPX, 1.5), false, cage(0.5, 0.5, 0.55)));
        let cwy = head + SF + (l.bottom as f32 - 1. - y).max(0.); let cwx = x1 - 1.;
        want.push((42, k, cell_to_world(cwx, cwy + SF), 3.7, sp.px.clone(), Vec2::new(1.5 * SF * CPX, 2. * SF * CPX), false, lit(0.4, 0.38, 0.36)));
    }
    // pressure tubes: each hopper a brass mouth on its pipe (brighter while its blower runs), and every
    // capsule a brass pill sliding along the pipe between its last two positions
    for (k, h) in w.tr.hoppers.iter().enumerate() {
        if w.g.flags[h.y.min(H - 1) * W + h.x.min(W - 1)] & F_SEEN == 0 { continue; }
        let (cx, cy) = h.centre(); let lt = lightf(cx as usize, cy as usize).to_srgba(); let hot = if w.logic.net_lit(w.logic.block_of(cx as usize, cy as usize)) { 1.25 } else { 1. };
        let lit = |r: f32, gg: f32, b: f32| Color::srgb(r * lt.red * hot, gg * lt.green * hot, b * lt.blue * hot);
        let n = molehill_sim::tubes::HOPPER as f32;
        want.push((43, k, cell_to_world(cx, cy + n * 0.25), 3.9, sp.px.clone(), Vec2::new(n * CPX, n * 0.5 * CPX), false, lit(0.62, 0.45, 0.22)));
        want.push((44, k, cell_to_world(cx, cy - n * 0.15), 3.9, sp.px.clone(), Vec2::new(n * 0.8 * CPX, n * 0.3 * CPX), false, lit(0.7, 0.52, 0.26)));
        want.push((45, k, cell_to_world(cx, cy - n * 0.32), 3.95, sp.px.clone(), Vec2::new(n * 0.6 * CPX, 1.5), false, lit(0.15, 0.13, 0.12)));
    }
    // ore hoppers: the bin's back boards, the heap of what it holds tinted with that item's colour, the front
    // frame (its gate shut or swung open under the spout) and two legs stretched down to the floor
    for (k, b) in w.tr.bins.iter().enumerate() {
        if w.g.flags[b.y.min(H - 1) * W + b.x.min(W - 1)] & F_SEEN == 0 { continue; }
        let (x0, y0, x1, y1) = b.rect(); let xc = (x0 + x1) / 2.;
        let lt = lightf(b.x, b.y.saturating_sub(2)).to_srgba(); let lit = |r: f32, gg: f32, bb: f32| Color::srgb(r * lt.red, gg * lt.green, bb * lt.blue);
        let (bw, bh) = (x1 - x0, y1 - y0 + 1.);
        let legs = (b.floor as f32 - y1).max(0.);
        for (j, xl) in [x0 + 1.5, x1 - 1.5].into_iter().enumerate() { want.push((48, k * 2 + j, cell_to_world(xl, y1 + legs / 2.), 3.85, sp.bin[3].clone(), Vec2::new(2., legs * CPX), false, lit(1., 1., 1.))); }
        want.push((47, k, cell_to_world(xc, y0 + bh / 2.), 3.9, sp.bin[0].clone(), Vec2::new(bw * CPX, bh * CPX), false, lit(1., 1., 1.)));
        let f = b.load() as f32 / molehill_sim::production::BIN_LOAD as f32;
        if let Some(&(it, _)) = b.contents.first() { let col = it.color(); let straight = (y1 - y0) * 5. / 8.; let hh = ((straight - 1.) * f).max(1.); // the heap fills the straight-sided part above the taper
            want.push((49, k, cell_to_world(xc, y0 + straight - hh / 2.), 3.95, sp.bin[4].clone(), Vec2::new((bw - 2.) * CPX, hh * CPX), false, lit(col[0] as f32 / 255., col[1] as f32 / 255., col[2] as f32 / 255.))); }
        want.push((50, k, cell_to_world(xc, y0 + bh / 2.), 3.96, sp.bin[if b.open { 2 } else { 1 }].clone(), Vec2::new(bw * CPX, bh * CPX), false, lit(1., 1., 1.)));
    }
    for (k, c) in w.tr.capsules.iter().enumerate() {
        let at = c.pat + (c.at - c.pat) * a; let i0 = c.path[(at as usize).min(c.path.len() - 1)] as usize; let i1 = c.path[(at as usize + 1).min(c.path.len() - 1)] as usize; let f = at.fract();
        let (x, y) = ((i0 % W) as f32 * (1. - f) + (i1 % W) as f32 * f + 0.5, (i0 / W) as f32 * (1. - f) + (i1 / W) as f32 * f + 0.5);
        want.push((46, k, cell_to_world(x, y), 4.6, sp.px.clone(), Vec2::splat(1.6 * SF * CPX), false, if c.rider { Color::srgb(1.0, 0.8, 0.45) } else { Color::srgb(0.85, 0.62, 0.3) }));
    }
    for (k, c) in w.chests.iter().enumerate() { if w.g.flags[c.y * W + c.x] & F_SEEN == 0 || w.g.is_solidish(c.y * W + c.x) && !c.opened { continue; } want.push((2, k, cell_to_world(c.x as f32 + 0.5 * SF, c.y as f32 + 1. - 1.1 * SF), 3., sp.chest[c.opened as usize].clone(), Vec2::new(16., 13.), false, lightf(c.x, c.y))); }
    for (k, b) in w.buildings.iter().enumerate() { want.push((3, k, cell_to_world(b.x as f32 + Station::W as f32 / 2., b.y as f32 + 1. - Station::H as f32 / 2.), 2., sp.station[b.kind.art() as usize][b.built as usize].clone(), Vec2::new(Station::W as f32 * CPX, Station::H as f32 * CPX), false, lightf(b.x + 5 * S, b.y - 2 * S))); }
    // torches: flickering flame frames and a small bloom round the flame (the light itself, with its shadows,
    // is the sim's ray-cast light); lit lamps get a small steady bloom
    for (k, &(x, y)) in w.torches.iter().enumerate() {
        let h = hash3(x as u32, y as u32, 77) as f32 / u32::MAX as f32; let t = now * 9. + h * 40.;
        let fl = 0.85 + 0.1 * (t * 1.3).sin() + 0.05 * (t * 3.7).sin();
        let f = ((now * 10. + h * 7.) as usize + k) % 3;
        want.push((4, k, cell_to_world(x as f32 + 0.5, y as f32 + 1. - 0.75 * SF), 4.5, sp.torch_f[f].clone(), Vec2::new(8., 12.), false, Color::WHITE));
        want.push((9, k, cell_to_world(x as f32 + 0.5, y as f32 + 1. - 1.3 * SF), 4.4, sp.glow.clone(), Vec2::splat(18. * fl), false, Color::srgba(1., 0.66, 0.3, 0.35 * fl)));
    }
    // glowmites: a bright speck with a soft cold halo that pulses a little
    for (k, (x, y, _, n)) in w.glowmites().into_iter().enumerate() {
        let p = 0.75 + 0.25 * (now * 3.1 + n as f32 * 1.7).sin();
        want.push((20, k, cell_to_world(x, y), 4.6, sp.px.clone(), Vec2::splat(1.6 * p.max(0.6)), false, Color::srgba(0.85, 1., 0.6, 0.95)));
        want.push((21, k, cell_to_world(x, y), 4.4, sp.glow.clone(), Vec2::splat(20. * p), false, Color::srgba(0.6, 1., 0.45, 0.18 * p)));
    }
    for (k, &b) in w.logic.dev_list.iter().enumerate() {
        if w.logic.dev[b] != Dev::Lamp as u8 || w.logic.dev_s[b] == 0 { continue; } let (x, y) = w.logic.centre(b);
        want.push((10, k, cell_to_world(x as f32 + 0.5, y as f32 + 0.5), 4.4, sp.glow.clone(), Vec2::splat(20.), false, Color::srgba(1., 0.95, 0.75, 0.3)));
    }
    // furniture: a base sprite and its cloth (dyed, or the undyed colour), lit like everything else; rugs lie behind
    let furn_px = |fu: &sw::Furniture| { let (x0, y0, x1, y1) = fu.rect(); (cell_to_world((x0 + x1 + 1) as f32 / 2., (y0 + y1 + 1) as f32 / 2.), Vec2::new((x1 + 1 - x0) as f32 * CPX, (y1 + 1 - y0) as f32 * CPX)) };
    for (k, fu) in w.furniture.iter().enumerate() {
        let Some((base, cloth)) = sp.furn[fu.item as usize].clone() else { continue }; let (x0, y0, x1, y1) = fu.rect();
        if w.g.flags[y1.min(H - 1) * W + x0.min(W - 1)] & F_SEEN == 0 { continue; }
        let (pos, size) = furn_px(fu); let l = lightf((x0 + x1) / 2, (y0 + y1) / 2).to_srgba(); let z = if fu.item == Item::Rug { 2.4 } else { 2.5 };
        want.push((16, k, pos, z, base, size, false, Color::srgb(l.red, l.green, l.blue)));
        if let Some(c) = cloth { let t = if fu.tint > 0 { craft::PIGMENTS[fu.tint as usize] } else { decorart::undyed(fu.item) };
            want.push((17, k, pos, z + 0.01, c, size, false, Color::srgb(t[0] as f32 / 255. * l.red, t[1] as f32 / 255. * l.green, t[2] as f32 / 255. * l.blue))); }
    }
    // found-place furnishings (props.rs): placed once per window against the cells there, drawn behind the
    // furniture and lit like it; they show once you've seen where they stand
    let key = (w.oy, w.sites.iter().map(|s| s.id).collect::<Vec<_>>()); if fx.props_key != key { fx.props = props::dress(w); fx.props_key = key; }
    for (k, p) in fx.props.iter().enumerate() {
        let (cx, by) = ((p.x + p.w / 2).min(W - 1), (p.y + p.h - 1).min(H - 1));
        if w.g.flags[by * W + cx] & F_SEEN == 0 && w.g.flags[p.y.min(H - 1) * W + cx] & F_SEEN == 0 { continue; }
        let pos = cell_to_world(p.x as f32 + p.w as f32 / 2., p.y as f32 + p.h as f32 / 2.);
        want.push((22, k, pos, 2.3, sp.props[p.prop as usize].clone(), Vec2::new(p.w as f32 * CPX, p.h as f32 * CPX), false, lightf(cx, (p.y + p.h / 2).min(H - 1))));
    }
    // Lid clues: carvings set in the wall, notes and odd finds on the floor (dimmer once read); a draft is only
    // felt: dust motes drifting down through it
    for (k, c) in w.clues.iter().enumerate() {
        if w.g.flags[c.y.min(H - 1) * W + c.x.min(W - 1)] & F_SEEN == 0 || w.g.is_solidish(c.y * W + c.x) { continue; }
        let l = lightf(c.x, c.y).to_srgba(); let k2 = if c.found { 0.75 } else { 1. };
        if let Some(i) = decorart::clue_index(c.kind, c.look) {
            let (sz, up) = if i < 2 { (Vec2::splat(12.), 0.5) } else { let s2 = [Vec2::splat(12.), Vec2::splat(12.), Vec2::new(9., 6.), Vec2::new(9., 6.), Vec2::new(11., 5.), Vec2::splat(6.), Vec2::splat(5.), Vec2::new(7., 5.), Vec2::new(9., 4.)][i]; (s2, sz_up(s2)) };
            want.push((19, k, cell_to_world(c.x as f32 + 0.5, c.y as f32 + 1. - up), 2.6, sp.clue[i].clone(), sz, false, Color::srgb(l.red * k2, l.green * k2, l.blue * k2)));
        } else if !g.paused && (g.frame as usize + k * 7) % 9 == 0 {
            let h = hash3(g.frame as u32, k as u32, 23); let r = ((h & 255) as f32 / 255.) - 0.5;
            fx.parts.push(Part { p: cell_to_world(c.x as f32 + 0.5 + r * 3. * SF, c.y as f32 - 5. * SF), v: Vec2::new(r * 6., -10. - (h >> 8 & 15) as f32), life: 0., max: 1.6, c: [200., 196., 186.], size: 0.9, g: 0., drag: 0., al: 1. });
        }
    }
    // placing furniture: a pale ghost where it would go
    if let (Some(Thing::Item(it)), Some((x, y, true))) = (g.slots[g.hot], g.aim) { if it.furn().is_some() { if let Some(fu) = w.furniture_spot(it, x, y) {
        if let Some((base, _)) = sp.furn[it as usize].clone() { let (pos, size) = furn_px(&fu); want.push((18, 0, pos, 6., base, size, false, Color::srgba(1., 1., 1., 0.45))); } } } }
    for (k, f) in w.farms.iter().enumerate() { let st = if f.growth < 0.3 { 0 } else if f.growth < 0.85 { 1 } else { 2 }; want.push((5, k, cell_to_world(f.x as f32 + 0.5 * SF, f.y as f32 + 1. - 1.9 * SF), 3.5, sp.shroom[st].clone(), Vec2::splat(10.), false, lightf(f.x, f.y))); }
    // aim: a faint outline on the cell under the cursor; mining: the targeted cell brightens and cracks as it gives way
    // aim: with a dig tool, a soft ring shows the brush; otherwise a faint cell outline. Mining: the ring tightens as the bite gives way.
    let brush = brush_of(g.slots[g.hot]);
    if let Some((x, y, reach)) = g.aim {
        if let Some(b) = brush.filter(|&b| g.sizes.snapping(b)) {
            // grid snap: a crisp square outline on the grid (the whole line while dragging), with a faint fill
            let (x0, y0, x1, y1) = World::line_extent(g.drag.unwrap_or((x, y)), (x, y), g.sizes.snap_cells());
            let k = if let sw::Brush::Place(it) = b { it.color() } else { [255, 255, 255] };
            let col = |a: f32| if reach { Color::srgba(k[0] as f32 / 255. * 1.2, k[1] as f32 / 255. * 1.2, k[2] as f32 / 255. * 1.2, a) } else { Color::srgba(1., 0.45, 0.35, a * 0.6) };
            let (wa, wb) = (cell_to_world(x0 as f32, y0 as f32), cell_to_world(x1 as f32 + 1., y1 as f32 + 1.));
            let (c, sz) = ((wa + wb) / 2., (wb - wa).abs()); let t = (CPX * 0.35).max(0.6);
            want.push((7, 0, c, 6., sp.px.clone(), sz, false, col(0.12)));
            for (j, (p, s2)) in [(Vec2::new(c.x, wa.y), Vec2::new(sz.x, t)), (Vec2::new(c.x, wb.y), Vec2::new(sz.x, t)), (Vec2::new(wa.x, c.y), Vec2::new(t, sz.y)), (Vec2::new(wb.x, c.y), Vec2::new(t, sz.y))].into_iter().enumerate() {
                want.push((7, 1 + j, p, 6.1, sp.px.clone(), s2, false, col(0.9))); }
        }
        else if let Some(b) = brush { let r = brush_r(&g, b); let d = 2. * r.max(0.5 * SF) * CPX + CPX * 0.5 * SF;
            let f = if matches!(b, sw::Brush::Dig(_)) { w.pl.mine.map_or(0., |(_, a, b)| a as f32 / b.max(1) as f32) } else { 0. };
            // dig: warm white ring that tightens as the bite gives way (a dim cold ring over rock your tools can't
            // dig, so it shows before the click); place: ring in the block's colour
            let refused = reach && matches!(b, sw::Brush::Dig(_)) && x < w.g.w && y < w.g.h && w.dig_refusal(w.g.get(x, y)).is_some();
            let c = if !reach { Color::srgba(1., 0.45, 0.35, 0.35) } else if refused { let [r, gg, bb, a] = hotbar::REFUSE_RING; let j = hotbar::refuse_jolt(now - fx.deny_t); Color::srgba(r + 0.3 * j, gg + 0.3 * j, bb + 0.25 * j, (a + 0.35 * j).min(1.)) } else if let sw::Brush::Place(it) = b { let k = it.color(); Color::srgba(k[0] as f32 / 255. * 1.2, k[1] as f32 / 255. * 1.2, k[2] as f32 / 255. * 1.2, 0.8) } else { Color::srgba(1., 0.97, 0.9, 0.6 + 0.4 * f) };
            let jolt = if refused { Vec2::new(hotbar::refuse_shake(now - fx.deny_t) * CPX * SF, 0.) } else { Vec2::ZERO };
            want.push((7, 0, cell_to_world(x as f32 + 0.5, y as f32 + 0.5) + jolt, 6., sp.ring.clone(), Vec2::splat(d * (1. - 0.15 * f)), false, c)); }
        else { let c = if reach { Color::srgba(1., 1., 0.85, 0.22) } else { Color::srgba(1., 0.4, 0.3, 0.10) }; want.push((7, 0, cell_to_world(x as f32 + 0.5, y as f32 + 0.5), 6., sp.px.clone(), Vec2::splat(CPX), false, c)); }
    }
    // placement pop: the placed block (or brush) flashes white and settles (bites burst into chips, above)
    for (k, &(kind, x, y, r, t0)) in g.pops.iter().enumerate() { if kind != 1 { continue; } let a = ((now - t0) / 0.2).clamp(0., 1.);
        want.push((11, k, cell_to_world(x, y), 6.5, sp.px.clone(), Vec2::splat(CPX * (2. * r + (0.7 - 0.7 * a) * SF)), false, Color::srgba(1., 1., 0.95, 0.8 * (1. - a)))); }
    let mut used = std::collections::HashSet::new();
    for (e, p, mut t, mut s, mut vis) in &mut q {
        if let Some(wn) = want.iter().find(|wn| wn.0 == p.0 && wn.1 == p.1) {
            t.translation = wn.2.extend(wn.3); t.rotation = Quat::from_rotation_z(rot.get(&(p.0, p.1)).copied().unwrap_or(0.)); s.image = wn.4.clone(); s.custom_size = Some(wn.5); s.flip_x = wn.6; s.color = wn.7; *vis = Visibility::Visible; used.insert((p.0, p.1));
        } else { commands.entity(e).despawn(); }
    }
    for wn in want { if !used.contains(&(wn.0, wn.1)) { let r = rot.get(&(wn.0, wn.1)).copied().unwrap_or(0.); commands.spawn((Sprite { image: wn.4, custom_size: Some(wn.5), flip_x: wn.6, color: wn.7, ..default() }, Transform::from_translation(wn.2.extend(wn.3)).with_rotation(Quat::from_rotation_z(r)), Pooled(wn.0, wn.1))); } }
}

/// What the mole has on: the headlamp (G or Settings), the item in its paws, and where it looks.
/// Kept through pauses (the aim only updates while playing), so a menu doesn't flick the light.
fn gear(mut g: ResMut<Game>, mut menu: ResMut<menu::Menu>, keys: Res<ButtonInput<KeyCode>>) {
    if !menu.open() && keys.just_pressed(KeyCode::KeyG) { menu.headlamp = !menu.headlamp; menu.persist(); g.msg = if menu.headlamp { "Headlamp lit".into() } else { "Headlamp out".into() }; }
    let held = match g.slots[g.hot] { Some(Thing::Item(it)) => Some(it), _ => None };
    let lamp = menu.headlamp; let aim = g.aim.map(|(x, y, _)| (x as f32 + 0.5, y as f32 + 0.5));
    let gr = &mut g.w.gear; gr.lamp = lamp; gr.held = held; if !menu.open() { gr.aim = aim; }
}
fn sound(mut g: ResMut<Game>, mut au: NonSendMut<audio::Audio>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, menu: Res<menu::Menu>, mut place: Local<(f32, f32)>) {
    if keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some() { au.unlock(); }
    au.set_volume(menu.volume); au.set_music(menu.music);
    let ev = std::mem::take(&mut g.ev);
    for e in ev { au.play(e, if e == sw::Event::Dig { 0.6 } else { 1. }); }
    if g.frame % 30 == 0 { let d = g.w.max_depth / 60.; au.ambient(d); }
    // where the music thinks we are: the mole's depth, and how much lava is around it (sampled every second)
    if g.frame % 60 == 0 { if let Some(m) = g.w.moles.first() {
        let (mx, my) = (m.x as i32, m.y as i32); let (mut lava, mut n) = (0, 0);
        let k = S as i32; for y in (my - 40 * k..=my + 40 * k).step_by(4 * S) { for x in (mx - 60 * k..=mx + 60 * k).step_by(4 * S) { if x < 0 || y < 0 || x >= W as i32 || y >= H as i32 { continue; } n += 1; if g.w.g.m(y as usize * W + x as usize) == Mat::Lava { lava += 1; } } }
        *place = (g.w.depth_m(m.x, m.y), lava as f32 / n.max(1) as f32); } }
    au.music(place.0, place.1);
}
