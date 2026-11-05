//! Molehill - Bevy front-end. The simulation lives in `molehill-sim`; this crate composites the
//! world into a GPU texture (albedo x per-cell light, fog of war, designations, cracks), draws
//! parallax background layers behind open cells, and syncs sprites for moles, items, chests,
//! stations and torches.
mod art;
mod audio;
mod ui;
mod menu;
use molehill_sim::logic::{Dev, DEVS, B as LB};
use molehill_sim::disasters::{Phase, Kind2};
use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::window::PrimaryWindow;
use molehill_sim::craft::{self, Item, Station, ITEMS, NITEM, QUESTS, RECIPES, STATIONS};
use molehill_sim::grid::{hash3, F_SEEN};
use molehill_sim::mat::{Kind, Mat, PROPS};
use molehill_sim::world::{self as sw, World};

const W: usize = 1024; const H: usize = 512; const CPX: f32 = 4.0; // world units per cell
const PAD: usize = 512;
const CH: usize = 64; // texture chunk size in cells
const FAR: usize = 4; // chunk rows drawn beyond each end of the simulated window (the camera roams there; nothing simulates)

#[derive(Resource)] pub struct Game { pub w: World, chunks: Vec<Handle<Image>>, buf: Vec<u8>, pub uploads: usize, acc: f32, pub paused: bool, speed: u32, pub frame: u64, last_light: u64, msg: String, pub help: bool, ev: Vec<sw::Event>, shake_off: Vec2, autosave: f32, last_oy: i64, far: Vec<Handle<Image>>, far_done: Vec<bool>, pub inv_open: bool, pub aim: Option<(usize, usize, bool)>,
    pub slots: Vec<Option<Thing>>, pub held: Option<Thing>, pub hot: usize, pub sel_t: f32, pub toasts: Vec<(String, f32)>, log_seen: u64, pub show_dev: bool, disp: Vec2, pub seed: u32,
    /// chosen brush radius per brush item (`brush_slot`), 0 = the item's default
    pub brush: [f32; 7],
    /// dev only (`--shot-walk`): hold right as if `D` were down
    pub demo_right: bool }
/// What an inventory slot holds. Counts come from the shared pack (`World::inv`).
#[derive(Clone, Copy, PartialEq, Debug)] pub enum Thing { Pick(u8), Item(Item), Dev(usize), Station(usize) }
pub const NSLOT: usize = 70;
/// The round brush an inventory thing uses, if any (dig tools and placeable blocks).
pub fn brush_of(t: Option<Thing>) -> Option<sw::Brush> { match t { Some(Thing::Pick(n)) => Some(sw::Brush::Dig(n)), Some(Thing::Item(it @ (Item::WoodPillar | Item::Brick | Item::IronBeam))) => Some(sw::Brush::Place(it)), _ => None } }
fn brush_slot(b: sw::Brush) -> usize { match b { sw::Brush::Dig(n) => (n as usize).min(3), sw::Brush::Place(Item::WoodPillar) => 4, sw::Brush::Place(Item::Brick) => 5, _ => 6 } }
/// The radius the brush of this thing will actually use (the player's choice, clamped to the item's range).
pub fn brush_r(g: &Game, b: sw::Brush) -> f32 { sw::brush_radius(b, g.brush[brush_slot(b)]) }
pub fn thing_name(_g: &Game, t: Thing) -> String { match t { Thing::Pick(n) => ["Pickaxe", "Drill", "Bore"][(n as usize - 1).min(2)].into(), Thing::Item(it) => it.name().into(), Thing::Dev(d) => DEVS[d].name().into(), Thing::Station(s) => STATIONS[s].name().into() } }
/// Tooltip line for an inventory thing.
pub fn thing_desc(t: Thing) -> &'static str { match t { Thing::Pick(1) => "Breaks soil and soft stone in a mole-sized bite.", Thing::Pick(2) => "Copperline-driven auger. Cuts granite, basalt and ore.", Thing::Pick(_) => "Heavy heat-bore. The widest bite; cuts the hardest stone.", Thing::Item(it) => it.desc(), Thing::Dev(d) => DEVS[d].desc(), Thing::Station(s) => STATIONS[s].desc() } }
pub fn thing_count(g: &Game, t: Thing) -> Option<u32> { match t { Thing::Item(it) => Some(g.w.inv[it as usize]), Thing::Dev(d) => dev_cost(DEVS[d]).map(|it| g.w.inv[it as usize]), _ => None } }
fn default_slots() -> Vec<Option<Thing>> {
    let mut v: Vec<Thing> = vec![Thing::Pick(1), Thing::Pick(2), Thing::Pick(3), Thing::Item(Item::WoodPillar), Thing::Item(Item::Brick), Thing::Item(Item::IronBeam), Thing::Item(Item::Torch), Thing::Item(Item::Tnt), Thing::Item(Item::CopperWire), Thing::Dev(DEVS.iter().position(|&d| d == Dev::Lever).unwrap_or(0))];
    v.push(Thing::Item(Item::Spore));
    for d in 0..DEVS.len() { if !v.contains(&Thing::Dev(d)) { v.push(Thing::Dev(d)); } }
    for k in 0..4 { v.push(Thing::Station(k)); }
    for &it in ITEMS.iter() { if !v.contains(&Thing::Item(it)) && !matches!(it, Item::StoneTools | Item::BronzeTools | Item::IronTools) { v.push(Thing::Item(it)); } }
    let mut out: Vec<Option<Thing>> = v.into_iter().map(Some).collect(); out.resize(NSLOT, None); out.truncate(NSLOT); out
}
#[derive(Resource)] struct Bg { layers: Vec<(Vec<u8>, f32)> }
#[derive(Component)] struct WorldSprite;
#[derive(Component)] struct Pooled(u8, usize);
#[derive(Resource)] pub struct Sprites { pub tools: [Handle<Image>; 3], pub dev: Handle<Image>, pub ring: Handle<Image>, pub mole: [Handle<Image>; 2], pub icons: Vec<Handle<Image>>, pub chest: [Handle<Image>; 2], pub station: Vec<[Handle<Image>; 2]>, pub torch: Handle<Image>, pub shroom: [Handle<Image>; 3], pub px: Handle<Image> }

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
    #[cfg(not(target_arch = "wasm32"))] { std::fs::write(format!("{slot}.save"), b).map_err(|e| e.to_string()) }
}
fn cell_to_world(x: f32, y: f32) -> Vec2 { Vec2::new(x * CPX - W as f32 * CPX / 2., H as f32 * CPX / 2. - y * CPX) }
fn world_to_cell(p: Vec2) -> (i32, i32) { (((p.x + W as f32 * CPX / 2.) / CPX).floor() as i32, ((H as f32 * CPX / 2. - p.y) / CPX).floor() as i32) }

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(Window { title: "Molehill".into(), canvas: Some("#bevy".into()), fit_canvas_to_parent: true, prevent_default_event_handling: true, ..default() }), ..default() }))
        .insert_resource(ClearColor(Color::srgb(0.04, 0.035, 0.05)))
        .add_systems(Startup, setup)
        .insert_non_send(audio::Audio::new())
        .add_systems(Update, (dev_shot, menu::update, input, simulate, render_world, sync_sprites, ui::update, sound).chain())
        .run();
}

/// Dev visual check (native): `--shot=FILE` starts a holt straight away, saves one frame after
/// `--shot-at=N` frames (default 150) and quits. `--shot-ui=pause|inv|help|dev` opens that panel first,
/// `--shot-walk` holds right. One still frame only: no videos.
fn dev_shot(mut n: Local<u32>, mut g: ResMut<Game>, mut m: ResMut<menu::Menu>, mut commands: Commands, mut exit: MessageWriter<AppExit>) {
    #[cfg(not(target_arch = "wasm32"))] {
        let arg = |k: &str| std::env::args().find_map(|a| a.strip_prefix(k).map(|v| v.to_string()));
        let Some(path) = arg("--shot=") else { return };
        let at: u32 = arg("--shot-at=").and_then(|v| v.parse().ok()).unwrap_or(150); let ui = arg("--shot-ui=").unwrap_or_default();
        *n += 1;
        if *n == 2 { m.go(menu::Screen::Off); g.paused = false; g.demo_right = std::env::args().any(|a| a == "--shot-walk"); }
        if *n == at.saturating_sub(5) { match ui.as_str() { "pause" => { m.go(menu::Screen::Pause); g.paused = true; } "inv" => g.inv_open = true, "help" => g.help = true, "dev" => g.show_dev = true, _ => {} } }
        if *n == at { commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(move |c: On<bevy::render::view::screenshot::ScreenshotCaptured>| write_bmp(&path, &c.image)); }
        if *n == at + 40 { exit.write(AppExit::Success); }
    }
    #[cfg(target_arch = "wasm32")] { let _ = (&mut n, &mut g, &mut m, &mut commands, &mut exit); }
}
/// 24-bit BMP writer for `--shot` (no image codecs are compiled in). Handles Rgba8/Bgra8 targets.
#[cfg(not(target_arch = "wasm32"))]
fn write_bmp(path: &str, im: &Image) {
    let (w, h) = (im.width() as usize, im.height() as usize); let Some(d) = im.data.as_ref() else { return };
    info!("shot {}x{} {:?}, {} non-zero bytes", w, h, im.texture_descriptor.format, d.iter().filter(|&&v| v != 0).count());
    let bgra = matches!(im.texture_descriptor.format, bevy::render::render_resource::TextureFormat::Bgra8UnormSrgb | bevy::render::render_resource::TextureFormat::Bgra8Unorm);
    let row = (w * 3 + 3) & !3; let size = 54 + row * h; let mut b = Vec::with_capacity(size);
    let le32 = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
    b.extend_from_slice(b"BM"); le32(&mut b, size as u32); le32(&mut b, 0); le32(&mut b, 54); le32(&mut b, 40); le32(&mut b, w as u32); le32(&mut b, h as u32);
    b.extend_from_slice(&1u16.to_le_bytes()); b.extend_from_slice(&24u16.to_le_bytes()); for _ in 0..6 { le32(&mut b, 0); }
    for y in (0..h).rev() { for x in 0..w { let o = (y * w + x) * 4; let (r, g, bl) = if bgra { (d[o + 2], d[o + 1], d[o]) } else { (d[o], d[o + 1], d[o + 2]) }; b.extend_from_slice(&[bl, g, r]); } for _ in 0..row - w * 3 { b.push(0); } }
    let _ = std::fs::write(path, b);
}
fn make_world(seed: u32) -> World { let mut w = World::new(W, H, seed); w.disasters.enabled = false; for v in w.inv.iter_mut() { *v = (*v).max(999); } stage_scene(&mut w); w }
/// --scene=water|tnt (native, for recording): stage a set piece next to the burrow. Play is unchanged.
fn stage_scene(w: &mut World) {
    #[cfg(not(target_arch = "wasm32"))] {
        let Some(sc) = std::env::args().find_map(|a| a.strip_prefix("--scene=").map(|s| s.to_string())) else { return };
        let (mx, my) = (w.moles[0].x, w.moles[0].y); let set = |w: &mut World, x: usize, y: usize, m: Mat| { let i = w.g.idx(x, y); w.g.set(i, m); };
        w.disasters.enabled = false;
        match sc.as_str() {
            "water" => { // an open tunnel east out of the burrow ending in clay, with a water pocket behind it
                for x in mx + 4..mx + 30 { for y in my - 2..=my { set(w, x, y, Mat::Empty); } set(w, x, my + 1, Mat::Clay); set(w, x, my - 3, Mat::Limestone); }
                for x in mx + 30..mx + 34 { for y in my - 5..=my + 1 { set(w, x, y, Mat::Clay); } }
                for x in mx + 34..mx + 50 { for y in my - 12..=my + 1 { set(w, x, y, Mat::Water); } set(w, x, my + 2, Mat::Clay); }
                molehill_sim::light::reveal_box(&mut w.g, mx, my - 20, mx + 60, my + 10);
            }
            "tnt" => { // a long tunnel under a sand bed
                for x in mx + 4..mx + 60 { for y in my - 2..=my { set(w, x, y, Mat::Empty); } set(w, x, my + 1, Mat::Limestone); for y in my - 6..my - 2 { set(w, x, y, Mat::Dirt); } for y in my - 16..my - 6 { set(w, x, y, Mat::Sand); } }
                w.inv[Item::Tnt as usize] = 3; w.inv[Item::Torch as usize] = 12;
                molehill_sim::light::reveal_box(&mut w.g, mx, my - 24, mx + 70, my + 10);
                for k in 0..4 { w.torches.push((mx + 22 + k * 10, my - 2)); }
            }
            _ => {}
        }
        let f = w.st.solve(&w.g); w.st.crack(&f);
    }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut fonts: ResMut<Assets<Font>>) {
    let seed = seed_now() % 100000;
    let w = make_world(seed);
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
    let (sx, sy) = (w.moles[0].x as f32, w.moles[0].y as f32 - 1.5);
    let p = cell_to_world(sx, sy);
    commands.spawn((Camera2d, Transform::from_xyz(p.x, p.y, 100.).with_scale(Vec3::splat(crate::menu::Menu::load().zoom))));
    let font: Handle<Font> = fonts.add(Font::from_bytes(include_bytes!("../assets/PixelifySans.ttf").to_vec()));
    menu::setup(&mut commands, &font); let mset = menu::Menu::load(); commands.insert_resource(mset);
    ui::setup(&mut commands, &font); commands.insert_resource(ui::UiFont(font));
    let sp = Sprites { tools: [images.add(art::tool_icon(1)), images.add(art::tool_icon(2)), images.add(art::tool_icon(3))], dev: images.add(art::icon([200, 150, 90], 1)), ring: images.add(art::ring()),
        mole: [images.add(art::mole(0)), images.add(art::mole(1))],
        icons: ITEMS.iter().map(|it| { let k = match it { Item::Crystal | Item::Quartz => 2, Item::Brick | Item::Glass | Item::CopperIngot | Item::TinIngot | Item::BronzeIngot | Item::IronIngot | Item::WoodPillar | Item::IronBeam => 1, Item::Torch | Item::Lamp => 3, _ => 0 }; images.add(art::icon(it.color(), k)) }).collect(),
        chest: [images.add(art::chest(false)), images.add(art::chest(true))],
        station: (0..4u8).map(|k| [images.add(art::station(k, false)), images.add(art::station(k, true))]).collect(),
        torch: images.add(art::icon([255, 190, 90], 3)),
        shroom: [images.add(art::mushroom(0)), images.add(art::mushroom(1)), images.add(art::mushroom(2))],
        px: images.add(art::image(1, 1, vec![255; 4], true)),
    };
    commands.insert_resource(sp);
    let disp = Vec2::new(sx, sy + 1.5); commands.insert_resource(Game { w, chunks, buf: vec![1; W * H * 4], uploads: 0, acc: 0., paused: true, speed: 1, frame: 0, last_light: 0, msg: String::new(), help: false, ev: vec![], shake_off: Vec2::ZERO, autosave: 0., last_oy: oy0, far, far_done, inv_open: false, aim: None, slots: default_slots(), held: None, hot: 0, sel_t: 0., toasts: vec![], log_seen: 0, show_dev: false, disp, seed, brush: [0.; 7], demo_right: false });
}

pub fn save_now(g: &mut Game) { let b = molehill_sim::save::save(&g.w); g.msg = match storage_put("molehill-manual", &b) { Ok(()) => "Game saved".into(), Err(e) => format!("Save failed: {e}") }; }
pub fn save_auto(g: &Game) { let b = molehill_sim::save::save(&g.w); let _ = storage_put("molehill-auto", &b); }
fn reset_view(g: &mut Game, commands: &mut Commands) {
    g.last_oy = g.w.oy; g.far_done.iter_mut().for_each(|v| *v = false); commands.insert_resource(make_bg(&g.w)); g.buf.iter_mut().for_each(|v| *v = 1);
    g.w.pl.sync = None; g.inv_open = false; g.help = false; g.acc = 0.;
}
/// Start a fresh holt (world) from a seed.
pub fn begin(g: &mut Game, commands: &mut Commands, seed: u32) { g.w = make_world(seed); g.seed = seed; reset_view(g, commands); g.msg = format!("Holt {seed}"); }
/// Load the manual save (or the autosave). Returns false if there is none or it failed.
pub fn load_saved(g: &mut Game, commands: &mut Commands) -> bool {
    match storage_get("molehill-manual").or_else(|| storage_get("molehill-auto")).map(|b| molehill_sim::save::load(&b)) {
        Some(Ok(w)) => { g.w = w; for v in g.w.inv.iter_mut() { *v = (*v).max(999); } reset_view(g, commands); g.msg = "Game loaded".into(); true }
        Some(Err(e)) => { g.msg = format!("Load failed: {e}"); false } None => { g.msg = "No save yet".into(); false } }
}
fn make_bg(w: &World) -> Bg {
    let mk = |k, f| (art::layer(k, w.oy, &w.info.biome, W, H, PAD, 1, w.info.seed).0, f);
    Bg { layers: vec![mk(art::Layer::Strata, 0.8), mk(art::Layer::Caves, 0.6), mk(art::Layer::Veins, 0.35)] }
}
fn dev_cost(d: Dev) -> Option<Item> { d.cost() }
fn input(mut g: ResMut<Game>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, scroll: Res<AccumulatedMouseScroll>,
         win: Query<&Window, With<PrimaryWindow>>, mut cam: Query<&mut Transform, With<Camera2d>>, time: Res<Time>,
         mut commands: Commands, mut menu: ResMut<menu::Menu>) {
    let Ok(win) = win.single() else { return }; let Ok(mut ct) = cam.single_mut() else { return };
    if g.paused { g.w.input = Default::default(); g.aim = None; return; } // a menu is open: no input, no aim ring
    let dt = time.delta_secs(); let sc = ct.scale.x; let now = time.elapsed_secs();
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
    // Space jumps (and wall-jumps); W also jumps when not on a wall, and climbs when clinging
    g.w.input.jump = keys.pressed(KeyCode::Space) || upk && g.w.pl.cling == 0;
    g.w.input.mine = None;
    let sel0 = g.hot;
    // wheel: hotbar; Ctrl+wheel: brush size with a brush item in hand, else zoom; Shift+wheel: zoom.
    // + / - zoom too; [ / ] resize the brush. Zoom is shared with Settings (clamped, saved).
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let held_brush = brush_of(g.slots[g.hot]);
    let notch = scroll.delta.y.signum();
    let mut resize = if keys.just_pressed(KeyCode::BracketRight) { 1. } else if keys.just_pressed(KeyCode::BracketLeft) { -1. } else { 0. };
    if notch != 0. {
        if ctrl && held_brush.is_some() { resize += notch; }
        else if ctrl || shift { menu.zoom_by(notch, &mut ct); }
        else { g.hot = (g.hot as i32 - notch as i32).rem_euclid(10) as usize; }
    }
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) { menu.zoom_by(1., &mut ct); }
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) { menu.zoom_by(-1., &mut ct); }
    let _ = sc;
    if let (Some(b), true) = (held_brush, resize != 0.) {
        let k = brush_slot(b); let r = brush_r(&g, b); let (lo, hi, _) = sw::brush_range(b).unwrap_or((0.5, 0.5, 0.5));
        g.brush[k] = (r + resize * 0.5).clamp(lo, hi); g.sel_t = now;
    }
    // render interpolation between the last two fixed sim steps; tight camera with a little lookahead
    { let a = g.acc.clamp(0., 1.); let pl = &g.w.pl; g.disp = Vec2::new(pl.ppx + (pl.px - pl.ppx) * a, pl.ppy + (pl.py - pl.ppy) * a - 1.0);
      let look = Vec2::new(g.w.pl.vx * 0.12, g.w.pl.vy.max(0.) * 0.05);
      let p = cell_to_world(g.disp.x + look.x, g.disp.y - 1.5 + look.y); let k = 1. - (-30. * dt).exp();
      let d = Vec2::new(p.x - ct.translation.x, p.y - ct.translation.y);
      if d.length() > 200. { ct.translation.x = p.x; ct.translation.y = p.y; } else { ct.translation.x += d.x * k; ct.translation.y += d.y * k; } }
    let half = Vec2::new(W as f32 * CPX / 2., H as f32 * CPX / 2.);
    ct.translation.x = ct.translation.x.clamp(-half.x, half.x); { let fy = half.y + (FAR * CH) as f32 * CPX; ct.translation.y = ct.translation.y.clamp(-fy, fy); }
    let digits = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9, KeyCode::Digit0];
    for (k, d) in digits.iter().enumerate() { if keys.just_pressed(*d) { g.hot = k; } }
    if keys.just_pressed(KeyCode::Escape) { g.inv_open = false; g.help = false; }
    if keys.just_pressed(KeyCode::KeyF) { g.speed = if g.speed >= 4 { 1 } else { g.speed * 2 }; }
    if keys.just_pressed(KeyCode::KeyI) || keys.just_pressed(KeyCode::KeyE) && g.inv_open { g.inv_open = !g.inv_open; }
    if !g.inv_open { if let Some(t) = g.held.take() { if let Some(k) = g.slots.iter().position(|s| s.is_none()) { g.slots[k] = Some(t); } } }
    if keys.just_pressed(KeyCode::KeyH) { g.help = !g.help; }
    if keys.just_pressed(KeyCode::F3) { g.show_dev = !g.show_dev; }
    if keys.just_pressed(KeyCode::KeyK) { save_now(&mut g); }
    if keys.just_pressed(KeyCode::KeyL) { load_saved(&mut g, &mut commands); }
    if keys.just_pressed(KeyCode::KeyB) && g.show_dev { let kind = [Kind2::Flood, Kind2::Lava, Kind2::Quake][(g.frame % 3) as usize]; let mut d = std::mem::replace(&mut g.w.disasters, molehill_sim::disasters::Disasters::new(0)); let _ = d.schedule(&mut g.w, kind); g.w.disasters = d; }
    // UI clicks: slots swallow the mouse. Hotbar click selects; in the pack, click picks up / drops / swaps.
    let cur = win.cursor_position();
    let over = cur.and_then(|c| ui::slot_at(c, win.width(), win.height(), g.inv_open));
    if let Some(k) = over {
    if mouse.just_pressed(MouseButton::Left) {
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
    if (keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)) && g.w.pl.grounded { g.w.input.mine = Some((mx, my + 2)); }
    let on_panel = over.is_some() || g.inv_open && cur.map_or(false, |c| { let a = ui::slot_pos(10, win.width(), win.height()); c.y > a.y - 10. && (c.x - win.width() / 2.).abs() < 5. * (ui::S + ui::GAP) + 10. });
    let Some(cur) = cur else { g.aim = None; return };
    if on_panel { g.aim = None; return; }
    let wp = Vec2::new(ct.translation.x + (cur.x - win.width() / 2.) * sc, ct.translation.y - (cur.y - win.height() / 2.) * sc);
    let (cx, cy) = world_to_cell(wp); if cx < 0 || cy < 0 || cx as usize >= W || cy as usize >= H { g.aim = None; return; }
    let (cx, cy) = (cx as usize, cy as usize);
    let reach = g.w.in_reach(cx, cy); g.aim = Some((cx, cy, reach));
    let blk = g.w.logic.block_of(cx, cy);
    if !reach { return; }
    // right mouse: interact (lever, thrum channel, ticker, light a fuse or TNT); Ctrl+right picks up copperline or a device
    if mouse.pressed(MouseButton::Right) && ctrl { g.w.pick_up(cx, cy); }
    else if mouse.just_pressed(MouseButton::Right) { g.w.use_device(cx, cy); }
    // left mouse: use the item in hand
    let (lp, lj) = (mouse.pressed(MouseButton::Left), mouse.just_pressed(MouseButton::Left));
    match sel {
        Some(Thing::Pick(_)) if lp => g.w.input.mine = Some((cx, cy)),
        Some(Thing::Item(it @ (Item::WoodPillar | Item::Brick | Item::IronBeam))) if lp => { let r = brush_r(&g, sw::Brush::Place(it)); if let Err(e) = g.w.place_brush(cx, cy, it, r) { if lj { g.msg = format!("Can't place: {e}"); } } }
        Some(Thing::Item(Item::CopperWire)) if lp => { g.w.lay_wire(cx, cy); }
        Some(Thing::Item(Item::Torch)) if lj => { if !g.w.place_torch(cx, cy) { g.msg = "Needs open space".into(); } }
        // TNT goes down inert; right mouse lights it, or wire it to a lever / knot / listener
        Some(Thing::Item(Item::Tnt)) if lj => { if g.w.logic.dev[blk] != 0 { g.w.use_device(cx, cy); } else if let Err(e) = g.w.place_device(Dev::Tnt, cx, cy) { g.msg = format!("Can't place: {e}"); } }
        // pack items that are exactly one device place it (knots and thrummers come in several kinds: use their device slots)
        Some(Thing::Item(it @ (Item::Lamp | Item::Vent))) if lj => { let d = if it == Item::Lamp { Dev::Lamp } else { Dev::Vent };
            if g.w.logic.dev[blk] != 0 { g.w.use_device(cx, cy); } else if let Err(e) = g.w.place_device(d, cx, cy) { g.msg = format!("Can't place: {e}"); } }
        Some(Thing::Item(Item::Spore)) if lj => { if let Err(e) = g.w.plant(cx, cy) { g.msg = format!("Can't plant: {e}"); } }
        Some(Thing::Dev(d)) if lj => {
            if g.w.logic.dev[blk] != 0 { g.w.use_device(cx, cy); }
            else if let Err(e) = g.w.place_device(DEVS[d], cx, cy) { g.msg = format!("Can't place: {e}"); }
        }
        Some(Thing::Station(k)) if lj => { let s = STATIONS[k]; if let Err(e) = g.w.place_building(s, cx.saturating_sub(Station::W / 2), cy) { g.msg = format!("Can't build: {e}"); } }
        _ => {}
    }
}

fn simulate(mut commands: Commands, mut g: ResMut<Game>, time: Res<Time>, mut cam: Query<(&mut Transform, &Projection), With<Camera2d>>, win: Query<&Window, With<PrimaryWindow>>) {
    g.frame += 1;
    if !g.paused {
        g.acc += time.delta_secs().min(0.1) * 60. * g.speed as f32;
        let mut n = 0; while g.acc >= 1. && n < 3 * g.speed { let pre = std::mem::take(&mut g.w.events); g.w.step(); let mut e = std::mem::take(&mut g.w.events); let gg = &mut *g; gg.ev.extend(pre); gg.ev.append(&mut e); g.acc -= 1.; n += 1; }
        g.autosave += time.delta_secs();
        if g.autosave > 30. { g.autosave = 0.; let b = molehill_sim::save::save(&g.w); let _ = storage_put("molehill-auto", &b); }
        if g.acc > 4. { g.acc = 0.; }
    }
    let (Ok((mut ct, _)), Ok(win)) = (cam.single_mut(), win.single()) else { return };
    if g.w.oy != g.last_oy { // window slid: keep the camera on the same rock, redraw everything
        let d = (g.w.oy - g.last_oy) as f32; ct.translation.y += d * CPX; g.last_oy = g.w.oy;
        commands.insert_resource(make_bg(&g.w)); g.buf.iter_mut().for_each(|v| *v = 1); g.far_done.iter_mut().for_each(|v| *v = false);
    }
    // camera shake (quakes, explosions): remove last frame's offset, add a new one
    let off = g.shake_off; ct.translation.x -= off.x; ct.translation.y -= off.y;
    let sh = g.w.shake; let f = g.frame as f32;
    g.shake_off = if sh > 0.05 { Vec2::new((f * 1.7).sin(), (f * 2.3).cos()) * sh * 3. * ct.scale.x.max(0.3) } else { Vec2::ZERO };
    ct.translation.x += g.shake_off.x; ct.translation.y += g.shake_off.y;
    let ct = &*ct;
    if g.frame - g.last_light >= 2 {
        g.last_light = g.frame;
        let r = view_rect(ct, win, 24);
        g.w.compute_light(r);
    }
}
fn view_rect(ct: &Transform, win: &Window, margin: i32) -> (usize, usize, usize, usize) {
    let sc = ct.scale.x; let hw = win.width() / 2. * sc; let hh = win.height() / 2. * sc;
    let (x0, y0) = world_to_cell(Vec2::new(ct.translation.x - hw, ct.translation.y + hh));
    let (x1, y1) = world_to_cell(Vec2::new(ct.translation.x + hw, ct.translation.y - hh));
    ((x0 - margin).clamp(0, W as i32 - 1) as usize, (y0 - margin).clamp(0, H as i32 - 1) as usize, (x1 + margin).clamp(0, W as i32 - 1) as usize, (y1 + margin).clamp(0, H as i32 - 1) as usize)
}

fn render_world(mut gm: ResMut<Game>, bg: Res<Bg>, mut images: ResMut<Assets<Image>>, cam: Query<&Transform, With<Camera2d>>, win: Query<&Window, With<PrimaryWindow>>) {
    let (Ok(ct), Ok(win)) = (cam.single(), win.single()) else { return };
    let (x0, y0, x1, y1) = view_rect(ct, win, 2);
    // snap the composited rect to whole chunks
    let (x0, y0) = (x0 / CH * CH, y0 / CH * CH); let (x1, y1) = (((x1 / CH) + 1) * CH - 1, ((y1 / CH) + 1) * CH - 1);
    let (x1, y1) = (x1.min(W - 1), y1.min(H - 1));
    let mut buf = std::mem::take(&mut gm.buf);
    let mut changed = vec![false; (W / CH) * (H / CH)];
    let g = &*gm;
    let lr = g.w.light_rect;
    let wd = &g.w; let gr = &wd.g;
    let camx = (ct.translation.x / CPX) as f32; // cells from world centre
    let bw = W + 2 * PAD;
    let (pmx, pmy) = wd.moles.first().filter(|m| m.alive).map(|m| (m.x as f32, m.y as f32 - 1.5)).unwrap_or((-999., -999.));
    let sample = |layer: &Vec<u8>, f: f32, x: usize, y: usize| -> [f32; 4] {
        let u = (x as f32 - camx * f) as i32 + PAD as i32; let u = u.clamp(0, bw as i32 - 1) as usize; let o = (y * bw + u) * 4;
        [layer[o] as f32, layer[o + 1] as f32, layer[o + 2] as f32, layer[o + 3] as f32 / 255.]
    };
    let over = |c: [f32; 3], s: [f32; 4]| [c[0] + (s[0] - c[0]) * s[3], c[1] + (s[1] - c[1]) * s[3], c[2] + (s[2] - c[2]) * s[3]];
    for y in y0..=y1 { for x in x0..=x1 {
        let i = y * W + x; let o = i * 4;
        let m = gr.m(i); let p = &PROPS[m as usize];
        let seen = gr.flags[i] & F_SEEN != 0;
        let px: [u8; 4];
        if !seen {
            let n = (hash3(x as u32 / 2, y as u32 / 2, 5) & 7) as u8;
            px = [14 + n, 12 + n, 17 + n, 255];
        } else {
            let lit = x >= lr.0 && x <= lr.2 && y >= lr.1 && y <= lr.3;
            let l = if lit { wd.light.at(i) } else { [0.05; 3] };
            let l = [l[0].max(0.22), l[1].max(0.21), l[2].max(0.24)]; // remembered rock stays faintly visible
            // the mole's own warm glow (its whiskers' sense of the rock nearby)
            let dd = ((x as f32 - pmx).powi(2) + (y as f32 - pmy).powi(2)).sqrt(); let gl = (1. - dd / 22.).max(0.) * 0.75;
            let l = [l[0].max(gl), l[1].max(gl * 0.92), l[2].max(gl * 0.8)];
            match p.kind {
                Kind::Empty | Kind::Gas => {
                    let lum = (l[0] * 0.3 + l[1] * 0.5 + l[2] * 0.2).min(1.3);
                    let mut c: [f32; 3] = [44., 34., 30.];
                    for (ly, f) in &bg.layers { c = over(c, sample(ly, *f, x, y)); }
                    // darkened by light, never pure black; coloured light tints the rock behind
                    let k = 0.16 + 0.84 * (lum * 0.8).min(1.);
                    let tint = [l[0] / lum.max(0.05), l[1] / lum.max(0.05), l[2] / lum.max(0.05)];
                    let tw = (lum * 0.6).min(0.5);
                    for j in 0..3 { c[j] = c[j] * k * (1. - tw + tw * tint[j].min(1.6)) * 0.72; }
                    // stale air: a sickly olive haze where CO2 has built up
                    let co2 = wd.air.at(x, y); if co2 > 0.2 { c = over(c, [96., 104., 52., ((co2 - 0.2) * 0.9).min(0.55)]); }
                    if m == Mat::Steam { c = over(c, [200., 200., 210., 0.4]); }
                    if m == Mat::Fire { c = [255., 150., 50.]; }
                    px = [c[0].min(255.) as u8, c[1].min(255.) as u8, c[2].min(255.) as u8, 255];
                }
                _ => {
                    let v = (hash3(x as u32, y as u32, m as u32) & 255) as f32 / 255.;
                    let v2 = (hash3(x as u32 / 3, y as u32 / 2, m as u32 + 77) & 255) as f32 / 255.;
                    let mut k = 0.86 + v * 0.14 + (v2 - 0.5) * 0.12;
                    if m == Mat::Grass { k *= 0.9 + 0.2 * v; }
                    // soft shading: lit top faces, shadowed undersides
                    if y > 0 && !gr.is_solidish(i - W) { k *= 1.18; } else if y + 1 < H && !gr.is_solidish(i + W) { k *= 0.78; }
                    if p.kind == Kind::Powder || gr.loose(i) { k *= 0.93 + 0.1 * v; }
                    if wd.st.pending[i] > 0 && (hash3(x as u32, y as u32, 9) % 3 == 0) { k *= 0.45; }
                    let emit = p.emit; let em = (emit[0] as u16 + emit[1] as u16 + emit[2] as u16) > 0;
                    let lm = |c: f32, li: f32| if em { c * (0.8 + 0.4 * v) } else { c * (li.min(1.25) * 0.94 + 0.06) };
                    let mut c = [lm(p.color[0] as f32 * k, l[0]), lm(p.color[1] as f32 * k, l[1]), lm(p.color[2] as f32 * k, l[2])];
                    let mut alpha = 255.;
                    if m == Mat::Water { c = [lm(40. + 30. * v2, l[0]), lm(90. + 20. * v2, l[1]), lm(170., l[2])]; alpha = 255.; }
                    if m == Mat::Lava { let f = 0.85 + 0.3 * ((x as f32 * 0.3 + y as f32 * 0.2 + wd.tick as f32 * 0.05).sin() * 0.5 + 0.5); c = [255. * f, 110. * f, 30. * f]; }
                    if m == Mat::Glass { c = [c[0] * 0.6 + 20., c[1] * 0.6 + 26., c[2] * 0.6 + 30.]; }
                    px = [c[0].min(255.) as u8, c[1].min(255.) as u8, c[2].min(255.) as u8, alpha as u8];
                }
            }
        }
        let mut px = px;
        if seen { // wiring / device overlay on the 1 m block layer
            let lg = &wd.logic; let bw = lg.bw; let bk = (y / LB) * bw + x / LB; let (lx, ly) = (x % LB, y % LB);
            let con = |c: usize| c < lg.wire.len() && (lg.wire[c] != 0 || lg.dev[c] != 0);
            if lg.dev[bk] != 0 {
                let d = Dev::from_u8(lg.dev[bk]); let on = lg.dev_s[bk] != 0 || lg.powered[bk] != 0 || lg.burn[bk] > 0;
                let base: [u8; 3] = match d { Dev::Lever => [200, 170, 90], Dev::Plate => [150, 150, 150], Dev::Sensor => [80, 150, 230], Dev::Door | Dev::Floodgate => [150, 110, 70], Dev::Pump => [90, 130, 200], Dev::Lamp => [255, 240, 170], Dev::Tnt => [220, 50, 40], Dev::Fuse => [180, 140, 100], Dev::Tx | Dev::Rx | Dev::Rep => [160, 120, 255], Dev::Vent => [170, 200, 190], Dev::Heat => [230, 90, 40], Dev::Wheel => [90, 160, 210], _ => [110, 220, 160] };
                let edge = lx == 0 || ly == 0 || lx == LB - 1 || ly == LB - 1;
                if !(matches!(d, Dev::Door | Dev::Floodgate) && !edge) {
                    let k = if edge { 0.55 } else if on { 1.15 } else { 0.7 };
                    let blink = lg.burn[bk] > 0 && (lg.burn[bk] / 6) % 2 == 0; // a lit fuse flickers
                    let base = if blink { [255, 230, 120] } else { base };
                    px = [(base[0] as f32 * k).min(255.) as u8, (base[1] as f32 * k).min(255.) as u8, (base[2] as f32 * k).min(255.) as u8, 255];
                    if d.is_thrum() && !edge && lx == 1 && ly == 1 { let ch = lg.dev_p[bk] as u8; px = [40 + ch * 25, 220 - ch * 20, 120, 255]; }
                }
            } else if lg.wire[bk] != 0 {
                let mid = |v: usize| v == 1 || v == 2;
                let draw = (mid(lx) && mid(ly)) || (mid(ly) && ((lx == 3 && x / LB + 1 < bw && con(bk + 1)) || (lx == 0 && x / LB > 0 && con(bk - 1))))
                    || (mid(lx) && ((ly == 3 && con(bk + bw)) || (ly == 0 && bk >= bw && con(bk - bw))));
                if draw { px = if lg.net_lit(bk) { [255, 205, 95, 255] } else { [150, 82, 45, 255] }; }
            }
        }
        if buf[o..o + 4] != px { buf[o..o + 4].copy_from_slice(&px); changed[(y / CH) * (W / CH) + x / CH] = true; }
    } }
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
        if x0 + CH as i32 <= vx0 - 8 || x0 > vx1 + 8 || y0 + CH as i32 <= vy0 - 8 || y0 > vy1 + 8 { continue; }
        if budget == 0 { break; } budget -= 1;
        let w = &gm.w; let mut data = vec![0u8; CH * CH * 4];
        for r in 0..CH { let wy = w.oy + (y0 + r as i32) as i64; let row = w.cache.get(&wy);
            for c in 0..CH { let x = cx * CH + c;
                let (m, seen) = match row { Some((m, f)) => (m[x], f[x] & F_SEEN != 0), None => (w.gen.cell(x, wy, W).0, false) };
                let mm = Mat::from_u8(m); let p = &PROPS[m as usize]; let v = (hash3(x as u32, wy as u32, m as u32) & 255) as f32 / 255.;
                let px: [f32; 3] = if seen {
                    match p.kind { Kind::Empty | Kind::Gas => [30., 24., 22.], _ => { let k = 0.5 + 0.08 * v; [p.color[0] as f32 * k, p.color[1] as f32 * k, p.color[2] as f32 * k] } }
                } else {
                    let n = (hash3(x as u32 / 2, wy as u32 / 2, 5) & 7) as f32;
                    match p.kind { Kind::Empty | Kind::Gas => [8. + n, 7. + n, 11. + n], _ if mm == Mat::Lava => [70. + n, 30., 16.], _ if mm == Mat::Water => [16., 24. + n, 44.],
                        _ => { let k = 0.16 + 0.03 * v; [p.color[0] as f32 * k + 12. + n, p.color[1] as f32 * k + 10. + n, p.color[2] as f32 * k + 15. + n] } }
                };
                let o = (r * CH + c) * 4; data[o] = px[0] as u8; data[o + 1] = px[1] as u8; data[o + 2] = px[2] as u8; data[o + 3] = 255;
            } }
        if let Some(mut im) = images.get_mut(&gm.far[k]) { if let Some(d) = im.data.as_mut() { d.copy_from_slice(&data); } }
        gm.far_done[k] = true;
    }
}

fn sync_sprites(mut commands: Commands, g: Res<Game>, sp: Res<Sprites>, mut q: Query<(Entity, &Pooled, &mut Transform, &mut Sprite, &mut Visibility)>) {
    // kind 0 moles, 1 drops, 2 chests, 3 stations, 4 torches, 5 mushroom plots
    let w = &g.w;
    let mut want: Vec<(u8, usize, Vec2, f32, Handle<Image>, Vec2, bool, Color)> = vec![];
    let lightf = |x: usize, y: usize| { let l = w.light.at(y.min(H - 1) * W + x.min(W - 1)); let v = (l[0] * 0.3 + l[1] * 0.5 + l[2] * 0.2).clamp(0.12, 1.2); Color::srgb(v.min(1.) , v.min(1.), v.min(1.)) };
    for (k, m) in w.moles.iter().enumerate() { if !m.alive { continue; }
        let walking = w.input.left || w.input.right || w.pl.mine.is_some();
        let c = if m.hp < 40. { Color::srgb(1., 0.6, 0.6) } else { let l = lightf(m.x, m.y - 1).to_srgba(); let v = l.red.max(0.55); Color::srgb(v, v, v) };
        let bob = if walking && w.pl.grounded { ((m.anim as f32) * 0.45).sin().abs() * 0.35 } else { 0. };
        want.push((0, k, cell_to_world(g.disp.x, g.disp.y + 1.0 - 1.6 - bob), 5., sp.mole[if walking { ((m.anim / 6) % 2) as usize } else { 0 }].clone(), Vec2::new(26., 19.5), m.facing < 0, c)); }
    for (k, d) in w.drops.iter().enumerate() { if w.g.flags[d.y * W + d.x] & F_SEEN == 0 { continue; } want.push((1, k, cell_to_world(d.x as f32 + 0.5, d.y as f32 + 0.2), 4., sp.icons[d.item as usize].clone(), Vec2::splat(12.), false, lightf(d.x, d.y))); }
    for (k, c) in w.chests.iter().enumerate() { if w.g.flags[c.y * W + c.x] & F_SEEN == 0 || w.g.is_solidish(c.y * W + c.x) && !c.opened { continue; } want.push((2, k, cell_to_world(c.x as f32 + 0.5, c.y as f32 - 0.6), 3., sp.chest[c.opened as usize].clone(), Vec2::new(16., 13.), false, lightf(c.x, c.y))); }
    for (k, b) in w.buildings.iter().enumerate() { want.push((3, k, cell_to_world(b.x as f32 + Station::W as f32 / 2., b.y as f32 + 1. - Station::H as f32 / 2.), 2., sp.station[b.kind as usize][b.built as usize].clone(), Vec2::new(Station::W as f32 * CPX, Station::H as f32 * CPX), false, lightf(b.x + 5, b.y - 2))); }
    for (k, &(x, y)) in w.torches.iter().enumerate() { want.push((4, k, cell_to_world(x as f32 + 0.5, y as f32 + 0.5), 4.5, sp.torch.clone(), Vec2::splat(10.), false, Color::WHITE)); }
    for (k, f) in w.farms.iter().enumerate() { let st = if f.growth < 0.3 { 0 } else if f.growth < 0.85 { 1 } else { 2 }; want.push((5, k, cell_to_world(f.x as f32 + 0.5, f.y as f32 - 0.9), 3.5, sp.shroom[st].clone(), Vec2::splat(10.), false, lightf(f.x, f.y))); }
    // aim: a faint outline on the cell under the cursor; mining: the targeted cell brightens and cracks as it gives way
    // aim: with a dig tool, a soft ring shows the brush; otherwise a faint cell outline. Mining: the ring tightens as the bite gives way.
    let brush = brush_of(g.slots[g.hot]);
    if let Some((x, y, reach)) = g.aim {
        if let Some(b) = brush { let r = brush_r(&g, b); let d = 2. * r.max(0.5) * CPX + CPX * 0.5;
            let f = if matches!(b, sw::Brush::Dig(_)) { w.pl.mine.map_or(0., |(_, a, b)| a as f32 / b.max(1) as f32) } else { 0. };
            // dig: warm white ring that tightens as the bite gives way; place: ring in the block's colour
            let c = if !reach { Color::srgba(1., 0.45, 0.35, 0.35) } else if let sw::Brush::Place(it) = b { let k = it.color(); Color::srgba(k[0] as f32 / 255. * 1.2, k[1] as f32 / 255. * 1.2, k[2] as f32 / 255. * 1.2, 0.8) } else { Color::srgba(1., 0.97, 0.9, 0.6 + 0.4 * f) };
            want.push((7, 0, cell_to_world(x as f32 + 0.5, y as f32 + 0.5), 6., sp.ring.clone(), Vec2::splat(d * (1. - 0.15 * f)), false, c)); }
        else { let c = if reach { Color::srgba(1., 1., 0.85, 0.22) } else { Color::srgba(1., 0.4, 0.3, 0.10) }; want.push((7, 0, cell_to_world(x as f32 + 0.5, y as f32 + 0.5), 6., sp.px.clone(), Vec2::splat(CPX), false, c)); }
    }
    let mut used = std::collections::HashSet::new();
    for (e, p, mut t, mut s, mut vis) in &mut q {
        if let Some(wn) = want.iter().find(|wn| wn.0 == p.0 && wn.1 == p.1) {
            t.translation = wn.2.extend(wn.3); s.image = wn.4.clone(); s.custom_size = Some(wn.5); s.flip_x = wn.6; s.color = wn.7; *vis = Visibility::Visible; used.insert((p.0, p.1));
        } else { commands.entity(e).despawn(); }
    }
    for wn in want { if !used.contains(&(wn.0, wn.1)) { commands.spawn((Sprite { image: wn.4, custom_size: Some(wn.5), flip_x: wn.6, color: wn.7, ..default() }, Transform::from_translation(wn.2.extend(wn.3)), Pooled(wn.0, wn.1))); } }
}

fn sound(mut g: ResMut<Game>, mut au: NonSendMut<audio::Audio>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>) {
    if keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some() { au.unlock(); }
    let ev = std::mem::take(&mut g.ev);
    for e in ev { au.play(e, if e == sw::Event::Dig { 0.6 } else { 1. }); }
    if g.frame % 30 == 0 { let d = g.w.max_depth / 60.; au.ambient(d); }
}
